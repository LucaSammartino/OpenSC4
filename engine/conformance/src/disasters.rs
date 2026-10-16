// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/disasters.md`: a scripted city (a wall
//! clock, occupants with their properties, coverage and lots, the
//! flammability, police, road and pipe cells, automata, the tuning
//! exemplars, a stub or the default generators) and ops that drive
//! [`Disasters`]. See README.md ("Disaster ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::disasters::{
    kind, msg, occupant, prop, shipped_flame, shipped_riot, shipped_volcano, tuning_instance, CellRect, DisasterHost, DisasterMessage,
    DisasterPost, DisasterRandom, Disasters, FireEntry, FireTuning, Generator, RiotTuning, Target, WorldBox,
};
use fc4_core::property::PropertyHolder;

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, ji, opt, u32_of, EventLog, R,
};

/// Tokens the world hands out: fire occupants, riot leaders, toxic
/// occupants (each from its base, in creation order).
const FIRE_BASE: u64 = 0xF000;
const LEADER_BASE: u64 = 0xA000;
const TOXIC_BASE: u64 = 0xC000;

#[derive(Clone, Default)]
struct Occ {
    otype: u32,
    cell: (i32, i32),
    rect: Option<CellRect>,
    position: Option<(f32, f32)>,
    in_manager: bool,
    lot: Option<u64>,
    flags: u32,
    props: BTreeMap<u32, f64>,
    burning: bool,
    coverage: u8,
    areas: (f32, f32),
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sys: RefCell<Option<Rc<Disasters>>>,
    session_seed: Cell<u32>,
    now: Cell<u64>,
    cells: Cell<(u32, u32)>,
    width: Cell<f32>,
    paused: Cell<bool>,
    emergency: Cell<i32>,
    save_disabled: Cell<i32>,
    established: Cell<bool>,
    follow: Cell<bool>,
    no_burn_down: Cell<bool>,
    wind: Cell<(f32, f32)>,
    fire_dispatch: Cell<u32>,
    police_dispatch: Cell<u32>,
    /// Fire occupant -> its target: a fire's cells and box are its target's.
    fire_targets: RefCell<BTreeMap<u64, u64>>,
    view_level: Cell<u32>,
    demolish_removes: Cell<bool>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    flammability: RefCell<BTreeMap<(i32, i32), u8>>,
    police: RefCell<BTreeMap<(i32, i32), u8>>,
    roads: RefCell<BTreeSet<(i32, i32)>>,
    network: RefCell<BTreeMap<(i32, i32), u64>>,
    pipes: RefCell<BTreeMap<(i32, i32), u64>>,
    pipe_neighbours: RefCell<BTreeMap<u64, Vec<(i32, i32)>>>,
    automata: RefCell<BTreeMap<u64, Option<(i32, i32)>>>,
    tuning: RefCell<BTreeMap<u32, Rc<PropertyHolder>>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    fires: Cell<u64>,
    leaders: Cell<u64>,
    toxics: Cell<u64>,
    stub: Cell<bool>,
    u32s: RefCell<VecDeque<u32>>,
    doubles: RefCell<VecDeque<f64>>,
}

/// The stub generator: queued u32s (then `RandomUint32Uniform(n)` gives
/// n − 1 and the raw draw 1) and doubles (then 0.99; ranges map a queued u
/// to `a + u (b − a)`, default u = 0.99 for uniform ranges, so a fire does
/// not spread and an `IgniteCell` probability draw fails below 99, and 0.5
/// for Gaussian ones). Every draw is an event.
struct Stub(Weak<World>, &'static str);

impl Stub {
    fn w(&self) -> Rc<World> {
        self.0.upgrade().expect("world alive")
    }
}

impl DisasterRandom for Stub {
    fn raw_u32(&mut self) -> u32 {
        let w = self.w();
        w.events.push("raw", vec![("generator", Json::Str(self.1.into()))]);
        let v = w.u32s.borrow_mut().pop_front();
        v.unwrap_or(1)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let w = self.w();
        w.events
            .push("u32", vec![("generator", Json::Str(self.1.into())), ("n", ji(n))]);
        let v = w.u32s.borrow_mut().pop_front();
        v.unwrap_or(n.saturating_sub(1))
    }
    fn uniform_f64(&mut self) -> f64 {
        let w = self.w();
        w.events.push("double", vec![("generator", Json::Str(self.1.into()))]);
        let v = w.doubles.borrow_mut().pop_front();
        v.unwrap_or(0.99)
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let w = self.w();
        w.events.push(
            "range",
            vec![
                ("generator", Json::Str(self.1.into())),
                ("a", Json::Float(a)),
                ("b", Json::Float(b)),
            ],
        );
        let u = w.doubles.borrow_mut().pop_front().unwrap_or(0.99);
        a + u * (b - a)
    }
    fn range_gaussian_fast(&mut self, a: f64, b: f64) -> f64 {
        let w = self.w();
        w.events.push(
            "gaussian_fast",
            vec![
                ("generator", Json::Str(self.1.into())),
                ("a", Json::Float(a)),
                ("b", Json::Float(b)),
            ],
        );
        let u = w.doubles.borrow_mut().pop_front().unwrap_or(0.5);
        a + u * (b - a)
    }
}

fn target_json(t: Target) -> Json {
    match t {
        Target::Layer => Json::Str("layer".into()),
        Target::Manager(m) => Json::Object(vec![("manager".into(), ji(m))]),
        Target::Instance(i) => Json::Object(vec![("instance".into(), ji(i))]),
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sys: RefCell::new(None),
            session_seed: Cell::new(0x1234_5678),
            now: Cell::new(0),
            cells: Cell::new((64, 64)),
            width: Cell::new(16.0),
            paused: Cell::new(false),
            emergency: Cell::new(0),
            save_disabled: Cell::new(0),
            established: Cell::new(true),
            follow: Cell::new(false),
            no_burn_down: Cell::new(false),
            wind: Cell::new((0.0, 0.0)),
            fire_dispatch: Cell::new(0),
            police_dispatch: Cell::new(0),
            fire_targets: RefCell::new(BTreeMap::new()),
            view_level: Cell::new(1),
            demolish_removes: Cell::new(false),
            occs: RefCell::new(BTreeMap::new()),
            flammability: RefCell::new(BTreeMap::new()),
            police: RefCell::new(BTreeMap::new()),
            roads: RefCell::new(BTreeSet::new()),
            network: RefCell::new(BTreeMap::new()),
            pipes: RefCell::new(BTreeMap::new()),
            pipe_neighbours: RefCell::new(BTreeMap::new()),
            automata: RefCell::new(BTreeMap::new()),
            tuning: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            fires: Cell::new(0),
            leaders: Cell::new(0),
            toxics: Cell::new(0),
            stub: Cell::new(true),
            u32s: RefCell::new(VecDeque::new()),
            doubles: RefCell::new(VecDeque::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w.tuning
            .borrow_mut()
            .insert(tuning_instance::FLAME, Rc::new(shipped_flame()));
        w.tuning
            .borrow_mut()
            .insert(tuning_instance::RIOT, Rc::new(shipped_riot()));
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sys(&self) -> R<Rc<Disasters>> {
        self.sys
            .borrow()
            .clone()
            .ok_or_else(|| "no disaster layer (dis_create)".into())
    }

    fn create(&self) -> Rc<Disasters> {
        let s = Rc::new(Disasters::new(
            self.rc() as Rc<dyn DisasterHost>,
            self.session_seed.get(),
        ));
        if self.stub.get() {
            for (g, name) in [
                (Generator::Fire, "fire"),
                (Generator::Riot, "riot"),
                (Generator::Earthquake, "earthquake"),
                (Generator::PipeBurst, "pipe_burst"),
                (Generator::PipeBurstStart, "pipe_start"),
            ] {
                s.set_random(g, Box::new(Stub(self.me.borrow().clone(), name)));
            }
        }
        *self.sys.borrow_mut() = Some(s.clone());
        s
    }

    fn occ<T>(&self, o: u64, f: impl FnOnce(&Occ) -> T) -> Option<T> {
        self.occs.borrow().get(&o).map(f)
    }

    fn ev(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }
}

impl DisasterHost for World {
    fn now_ms(&self) -> u64 {
        self.now.get()
    }
    fn emergency_pause(&self) {
        self.emergency.set(self.emergency.get() + 1);
        self.ev("emergency_pause", vec![]);
    }
    fn emergency_resume(&self) {
        self.emergency.set(self.emergency.get() - 1);
        self.ev("emergency_resume", vec![]);
    }
    fn disable_save(&self) {
        self.save_disabled.set(self.save_disabled.get() + 1);
        self.ev("disable_save", vec![]);
    }
    fn enable_save(&self) {
        self.save_disabled.set(self.save_disabled.get() - 1);
        self.ev("enable_save", vec![]);
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
        self.cells.get()
    }
    fn cell_width(&self) -> f32 {
        self.width.get()
    }
    fn post(&self, p: &DisasterPost) {
        self.ev(
            "post",
            vec![
                ("type", ji(p.msg_type)),
                ("data1", ji(p.data1 as i64)),
                ("data2", ji(p.data2)),
                ("data3", ji(p.data3)),
                ("data4", ji(p.data4)),
                ("void1", ji(p.void1)),
            ],
        );
    }
    fn subscribe(&self, t: Target, m: u32) {
        self.ev("subscribe", vec![("target", target_json(t)), ("type", ji(m))]);
    }
    fn unsubscribe(&self, t: Target, m: u32) {
        self.ev("unsubscribe", vec![("target", target_json(t)), ("type", ji(m))]);
    }
    fn register_tick(&self, i: u32, p: u32) {
        self.ev("register_tick", vec![("instance", ji(i)), ("priority", ji(p))]);
    }
    fn unregister_tick(&self, i: u32) {
        self.ev("unregister_tick", vec![("instance", ji(i))]);
    }
    fn record_history(&self, s: u32, v: i32) {
        self.ev("history", vec![("series", ji(s)), ("value", ji(v))]);
    }
    fn tuning(&self, i: u32) -> Option<Rc<PropertyHolder>> {
        self.ev("tuning", vec![("instance", ji(i))]);
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
        self.ev("go_to", vec![("x", ji(x)), ("z", ji(z)), ("zoom", ji(zoom))]);
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
        let o = self.fire_targets.borrow().get(&o).copied().unwrap_or(o);
        self.occ(o, |x| x.rect.unwrap_or(CellRect::cell(x.cell.0, x.cell.1)))
            .unwrap_or_default()
    }
    fn occupant_position(&self, o: u64) -> (f32, f32) {
        let w = self.width.get();
        self.occ(o, |x| {
            x.position
                .unwrap_or(((x.cell.0 as f32 + 0.5) * w, (x.cell.1 as f32 + 0.5) * w))
        })
        .unwrap_or((0.0, 0.0))
    }
    fn occupant_bbox(&self, o: u64) -> WorldBox {
        let w = self.width.get();
        let r = self.occupant_cell_rect(o);
        WorldBox {
            x0: r.x0 as f32 * w,
            z0: r.z0 as f32 * w,
            x1: (r.x1 + 1) as f32 * w,
            z1: (r.z1 + 1) as f32 * w,
        }
    }
    fn occupant_bbox_areas(&self, o: u64) -> (f32, f32) {
        self.occ(o, |x| x.areas).unwrap_or((0.0, 0.0))
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
        self.ev("remove_occupant", vec![("occupant", ji(o as i64))]);
        if let Some(x) = self.occs.borrow_mut().get_mut(&o) {
            x.in_manager = false;
        }
    }
    fn occupant_lot(&self, o: u64) -> Option<u64> {
        self.occ(o, |x| x.lot).flatten()
    }
    fn set_lot_condition(&self, lot: u64, c: u32) {
        self.ev("condition", vec![("lot", ji(lot as i64)), ("condition", ji(c))]);
    }
    fn fire_coverage(&self, o: u64) -> u8 {
        self.occ(o, |x| x.coverage).unwrap_or(0)
    }
    fn flammability_at(&self, x: i32, z: i32) -> u8 {
        self.flammability.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn create_fire_occupant(&self, i: u32, target: u64, id: u32, flames: (u32, u32)) -> Option<u64> {
        let t = FIRE_BASE + self.fires.get();
        self.fires.set(self.fires.get() + 1);
        if let Some(x) = self.occs.borrow_mut().get_mut(&target) {
            x.burning = true;
        }
        self.fire_targets.borrow_mut().insert(t, target);
        self.ev(
            "create_fire",
            vec![
                ("instance", ji(i)),
                ("fire", ji(t as i64)),
                ("target", ji(target as i64)),
                ("fire_id", ji(id)),
                ("top", ji(flames.0)),
                ("side", ji(flames.1)),
            ],
        );
        Some(t)
    }
    fn remove_fire_occupant(&self, fire: u64, target: u64) {
        if let Some(x) = self.occs.borrow_mut().get_mut(&target) {
            x.burning = false;
        }
        self.fire_targets.borrow_mut().remove(&fire);
        self.ev(
            "remove_fire",
            vec![("fire", ji(fire as i64)), ("target", ji(target as i64))],
        );
    }
    fn add_fire_aura(&self, o: u64) {
        self.ev("aura", vec![("occupant", ji(o as i64))]);
    }
    fn tutorial_no_burn_down(&self) -> bool {
        self.no_burn_down.get()
    }
    fn wind(&self) -> (f32, f32) {
        self.wind.get()
    }
    fn play_effect(&self, name: &str, o: u64) {
        self.ev(
            "effect",
            vec![("name", Json::Str(name.into())), ("occupant", ji(o as i64))],
        );
    }
    fn play_effect_at(&self, name: &str, x: f32, z: f32) {
        self.ev(
            "effect_at",
            vec![
                ("name", Json::Str(name.into())),
                ("x", scalar::jf(x)),
                ("z", scalar::jf(z)),
            ],
        );
    }
    fn burn_flora(&self, o: u64) {
        self.ev("burn_flora", vec![("occupant", ji(o as i64))]);
    }
    fn demolish_lot(&self, o: u64, lot: u64, flags: u32) {
        self.ev(
            "demolish",
            vec![
                ("occupant", ji(o as i64)),
                ("lot", ji(lot as i64)),
                ("flags", ji(flags)),
            ],
        );
        if self.demolish_removes.get() {
            if let Some(x) = self.occs.borrow_mut().get_mut(&o) {
                x.in_manager = false;
            }
            let sys = self.sys.borrow().clone();
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
        self.ev(
            "radiation",
            vec![
                ("occupant", ji(o as i64)),
                ("amount", scalar::jf(a)),
                ("radius", scalar::jf(r)),
            ],
        );
    }
    fn police_coverage(&self, x: i32, z: i32) -> u8 {
        self.police.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn is_road_cell(&self, x: i32, z: i32) -> bool {
        self.roads.borrow().contains(&(x, z))
    }
    fn cell_corner_position(&self, x: i32, z: i32) -> (f32, f32) {
        let w = self.width.get();
        (x as f32 * w, z as f32 * w)
    }
    fn create_riot_leader(&self, x: f32, z: f32) -> Option<u64> {
        let t = LEADER_BASE + self.leaders.get();
        self.leaders.set(self.leaders.get() + 1);
        let w = self.width.get();
        let cell = ((x / w).floor() as i32, (z / w).floor() as i32);
        self.automata.borrow_mut().insert(t, Some(cell));
        self.ev(
            "leader",
            vec![("automaton", ji(t as i64)), ("x", scalar::jf(x)), ("z", scalar::jf(z))],
        );
        Some(t)
    }
    fn network_occupant_at(&self, x: i32, z: i32) -> Option<u64> {
        self.network.borrow().get(&(x, z)).copied()
    }
    fn unset_removal_flags(&self, o: u64, flags: u32) {
        self.ev("unset_removal", vec![("occupant", ji(o as i64)), ("flags", ji(flags))]);
    }
    fn set_removal_flags(&self, o: u64, flags: u32) {
        self.ev("set_removal", vec![("occupant", ji(o as i64)), ("flags", ji(flags))]);
    }
    fn automaton_cell(&self, a: u64) -> Option<(i32, i32)> {
        self.automata.borrow().get(&a).copied().flatten()
    }
    fn set_riot_effect(&self, a: u64, s: i32) {
        self.ev("riot_effect", vec![("automaton", ji(a as i64)), ("stage", ji(s))]);
    }
    fn send_riot_message(&self, a: u64, s: i32) {
        self.ev("riot_message", vec![("automaton", ji(a as i64)), ("stage", ji(s))]);
    }
    fn end_riot_leader(&self, a: u64) {
        self.ev("end_leader", vec![("automaton", ji(a as i64))]);
    }
    fn riot_leader_tick(&self, a: u64) {
        self.ev("leader_tick", vec![("automaton", ji(a as i64))]);
    }
    fn create_toxic_occupant(&self, i: u32, x: i32, z: i32, amount: i32) -> Option<u64> {
        let t = TOXIC_BASE + self.toxics.get();
        self.toxics.set(self.toxics.get() + 1);
        self.ev(
            "toxic",
            vec![
                ("instance", ji(i)),
                ("occupant", ji(t as i64)),
                ("x", ji(x)),
                ("z", ji(z)),
                ("amount", ji(amount)),
            ],
        );
        Some(t)
    }
    fn set_toxic_amount(&self, o: u64, a: i32) {
        self.ev("toxic_amount", vec![("occupant", ji(o as i64)), ("amount", ji(a))]);
    }
    fn remove_toxic_occupant(&self, o: u64) {
        self.ev("remove_toxic", vec![("occupant", ji(o as i64))]);
    }
    fn apply_terrain_brush(&self, b: u32, x: f32, z: f32, s: f32, w: f32) {
        self.ev(
            "brush",
            vec![
                ("brush", ji(b)),
                ("x", scalar::jf(x)),
                ("z", scalar::jf(z)),
                ("strength", scalar::jf(s)),
                ("width", scalar::jf(w)),
            ],
        );
    }
    fn redraw_terrain(&self, r: CellRect) {
        self.ev(
            "redraw",
            vec![("x0", ji(r.x0)), ("z0", ji(r.z0)), ("x1", ji(r.x1)), ("z1", ji(r.z1))],
        );
    }
    fn end_long_terrain_operation(&self) {
        self.ev("end_long_terrain", vec![]);
    }
    fn stop_terrain_texture_updates(&self) {
        self.ev("stop_textures", vec![]);
    }
    fn place_repulsor(&self, x: f32, z: f32) {
        self.ev("repulsor", vec![("x", scalar::jf(x)), ("z", scalar::jf(z))]);
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
        let ok = level == 1 || level == 3;
        self.ev(
            "leak_effect",
            vec![("pipe", ji(p as i64)), ("level", ji(level)), ("shown", Json::Bool(ok))],
        );
        ok
    }
    fn connected_pipe_cells(&self, p: u64) -> Vec<(i32, i32)> {
        self.pipe_neighbours.borrow().get(&p).cloned().unwrap_or_default()
    }
    fn demolish_pipe(&self, p: u64, flags: u32) {
        self.ev("demolish_pipe", vec![("pipe", ji(p as i64)), ("flags", ji(flags))]);
        self.pipes.borrow_mut().retain(|_, t| *t != p);
    }
    fn volcano_start_deformation(&self, i: u32, _x: f32, _z: f32) {
        self.ev("volcano_deformation", vec![("instance", ji(i))]);
    }
    fn volcano_start_lava(&self, i: u32, _x: f32, _z: f32, r: f32) {
        self.ev("lava", vec![("instance", ji(i)), ("radius", scalar::jf(r))]);
    }
    fn volcano_lava_rates(&self, _i: u32, _p: f32, _s: f32) {}
    fn volcano_end_lava(&self, i: u32) {
        self.ev("lava_end", vec![("instance", ji(i))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "disasters", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("dis_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "flame_tuning" => flame_tuning(op),
        "riot_tuning" => riot_tuning(op),
        "occupant" => occupant_op(&w, op),
        "cell" => cell_op(&w, op),
        "automaton" => automaton_op(&w, op),
        "create" => {
            if let Some(v) = opt(op, "session_seed") {
                match u32_of(v, "session_seed") {
                    Ok(s) => w.session_seed.set(s),
                    Err(e) => return Some(Err(e)),
                }
            }
            w.create();
            Ok(())
        }
        "init" => w.sys().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sys().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "clock" => clock(&w, op),
        "call" => call(&w, op),
        "message" => message(&w, op),
        "tick" => tick(&w, op),
        "seconds" => seconds(&w, op),
        "expect" => expect(&w, op),
        "expect_fire" => expect_fire(&w, op),
        "expect_riot" => expect_riot(&w, op),
        "expect_instance" => expect_instance(&w, op),
        "expect_volcano" => expect_volcano(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- names ----------------------------------------------------------------------------------------

/// A manager type: a name or an ID.
fn manager_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("volcano") => kind::VOLCANO,
        Some("fire") => kind::FIRE,
        Some("earthquake") => kind::EARTHQUAKE,
        Some("pipe_burst") => kind::PIPE_BURST,
        Some("riot") => kind::RIOT,
        Some("effect") => kind::EFFECT,
        Some("toxic") => kind::TOXIC,
        _ => u32_of(j, "manager")?,
    })
}

fn occupant_type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("building") => occupant::BUILDING,
        Some("flora") => occupant::FLORA,
        Some("prop") => occupant::PROP,
        Some("network") => occupant::NETWORK,
        _ => u32_of(j, "type")?,
    })
}

fn message_type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("DispatchBegin") => msg::DISPATCH_BEGIN,
        Some("DispatchActive") => msg::DISPATCH_ACTIVE,
        Some("DispatchInactive") => msg::DISPATCH_INACTIVE,
        Some("DispatchEnd") => msg::DISPATCH_END,
        Some("ToxicSpill") => msg::TOXIC_SPILL_CHEAT,
        Some("PreViewLevelChange") => msg::PRE_VIEW_LEVEL_CHANGE,
        Some("EffectFinished") => msg::EFFECT_FINISHED,
        Some("EffectMoved") => msg::EFFECT_MOVED,
        Some("VolcanoMessage") => msg::VOLCANO_MESSAGE,
        _ => u32_of(j, "type")?,
    })
}

fn target_of(j: &Json) -> R<Target> {
    if j.as_str() == Some("layer") {
        return Ok(Target::Layer);
    }
    if let Some(m) = j.get("manager") {
        return Ok(Target::Manager(manager_of(m)?));
    }
    if let Some(i) = j.get("instance") {
        return Ok(Target::Instance(u32_of(i, "instance")?));
    }
    Err("target must be \"layer\", {\"manager\": …} or {\"instance\": …}".into())
}

fn tok(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

// ---- setup ----------------------------------------------------------------------------------------

/// The tuning of one exemplar: "shipped", null (none) or {"props": …}.
fn tuning_of(instance: u32, v: &Json) -> R<Option<PropertyHolder>> {
    Ok(match v {
        Json::Null => None,
        Json::Str(s) if s == "shipped" => match instance {
            tuning_instance::FLAME => Some(shipped_flame()),
            tuning_instance::RIOT => Some(shipped_riot()),
            tuning_instance::VOLCANO => Some(shipped_volcano()),
            _ => return Err("only the flame, riot and volcano exemplars have a \"shipped\" form".into()),
        },
        other => Some(holder_of(field(other, "props")?)?),
    })
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair(v, "cells")?;
        w.cells.set((x as u32, z as u32));
    }
    if let Some(v) = opt(op, "cell_width") {
        w.width.set(f32_of(v, "cell_width")?);
    }
    for (k, c) in [
        ("paused", &w.paused),
        ("established", &w.established),
        ("follow", &w.follow),
        ("no_burn_down", &w.no_burn_down),
        ("demolish_removes", &w.demolish_removes),
    ] {
        if let Some(v) = opt(op, k) {
            c.set(bool_of(v, k)?);
        }
    }
    if let Some(v) = opt(op, "emergency") {
        w.emergency.set(i64_of(v, "emergency")? as i32);
    }
    if let Some(v) = opt(op, "wind") {
        // [x, z] (weather slot 22's vector); a number is (x, 0).
        w.wind.set(match v {
            Json::Array(_) => {
                let a = arr(v, "wind")?;
                if a.len() != 2 {
                    return Err("'wind' must be [x, z] or a number".into());
                }
                (f32_of(&a[0], "wind")?, f32_of(&a[1], "wind")?)
            }
            n => (f32_of(n, "wind")?, 0.0),
        });
    }
    if let Some(v) = opt(op, "fire_dispatch_count") {
        w.fire_dispatch.set(u32_of(v, "fire_dispatch_count")?);
    }
    if let Some(v) = opt(op, "police_dispatch_count") {
        w.police_dispatch.set(u32_of(v, "police_dispatch_count")?);
    }
    if let Some(v) = opt(op, "view_level") {
        w.view_level.set(u32_of(v, "view_level")?);
    }
    if let Some(v) = op.get("tuning") {
        for (k, t) in v.as_object().ok_or("tuning must be an object")? {
            let inst = match k.as_str() {
                "flame" => tuning_instance::FLAME,
                "riot" => tuning_instance::RIOT,
                "earthquake" => tuning_instance::EARTHQUAKE,
                "pipe_burst" => tuning_instance::PIPE_BURST,
                "volcano" => tuning_instance::VOLCANO,
                other => return Err(format!("unknown tuning '{other}'")),
            };
            match tuning_of(inst, t)? {
                Some(h) => w.tuning.borrow_mut().insert(inst, Rc::new(h)),
                None => w.tuning.borrow_mut().remove(&inst),
            };
        }
    }
    if let Some(v) = opt(op, "random") {
        match v.as_str() {
            Some("stub") => w.stub.set(true),
            Some("default") => w.stub.set(false),
            _ => return Err("random must be \"stub\" or \"default\"".into()),
        }
    }
    if let Some(v) = opt(op, "u32s") {
        for d in arr(v, "u32s")? {
            w.u32s.borrow_mut().push_back(u32_of(d, "u32")?);
        }
    }
    if let Some(v) = opt(op, "doubles") {
        for d in arr(v, "doubles")? {
            w.doubles
                .borrow_mut()
                .push_back(scalar::num(d).ok_or("doubles must be numbers")?);
        }
    }
    Ok(())
}

/// Pairs `[[x, y], …]` of a lookup: `f(x)` must equal (or be close to) `y`.
fn lookups(op: &Json, key: &str, f: impl Fn(&Json) -> R<Json>) -> R<()> {
    let Some(v) = opt(op, key) else { return Ok(()) };
    for p in arr(v, key)? {
        let p = arr(p, key)?;
        if p.len() != 2 {
            return Err(format!("'{key}' entries must be [input, output]"));
        }
        let got = f(&p[0])?;
        if !scalar::matches(&got, &p[1], false) {
            return Err(format!(
                "{key}({}): got {}, expected {}",
                scalar::show(&p[0]),
                scalar::show(&got),
                scalar::show(&p[1])
            ));
        }
    }
    Ok(())
}

fn u32_list(v: &[u32]) -> Json {
    Json::Array(v.iter().map(|x| ji(*x)).collect())
}

/// `dis_flame_tuning`: the "Flame" exemplar `tuning` ("shipped", null or
/// {"props": …}); `state` (partial: `d3`, `d3cum`, `d4`, `d5`, `d6`,
/// `zoom`); lookups `stages` [[counter, stage]], `d9` [[coverage, value]],
/// `resistance` [[coverage, value]], `da` [[flammability, value]].
fn flame_tuning(op: &Json) -> R<()> {
    let h = tuning_of(tuning_instance::FLAME, field(op, "tuning")?)?;
    let t = FireTuning::from_holder(h.as_ref());
    if let Some(want) = opt(op, "state") {
        let got = vec![
            ("d3".to_string(), u32_list(&t.d3)),
            ("d3cum".into(), u32_list(&t.d3cum)),
            ("d4".into(), u32_list(&t.d4)),
            ("d5".into(), u32_list(&t.d5)),
            ("d6".into(), scalar::jf(t.d6)),
            ("zoom".into(), ji(t.zoom)),
            ("d9_empty".into(), Json::Bool(t.d9.is_empty())),
        ];
        check_state(&got, want)?;
    }
    lookups(op, "stages", |c| Ok(ji(t.stage_of(i64_of(c, "counter")? as i32))))?;
    lookups(op, "d9", |c| Ok(scalar::jf(t.d9.eval(f32_of(c, "coverage")?))))?;
    lookups(op, "resistance", |c| Ok(ji(t.resistance(u32_of(c, "coverage")? as u8))))?;
    lookups(op, "da", |c| Ok(scalar::jf(t.da.eval(f32_of(c, "flammability")?))))
}

/// `dis_riot_tuning`: the "Riot" exemplar `tuning`; `state` (partial:
/// `max_riots`, `max_leaders`, `stages`, `seconds_per_stage`,
/// `leaders_spawned`, `zoom`); lookups `police` [[coverage, value]] and
/// `stage_of` [[counter, stage]].
fn riot_tuning(op: &Json) -> R<()> {
    let h = tuning_of(tuning_instance::RIOT, field(op, "tuning")?)?;
    let t = RiotTuning::from_holder(h.as_ref());
    if let Some(want) = opt(op, "state") {
        let got = vec![
            ("max_riots".to_string(), ji(t.max_riots)),
            ("max_leaders".into(), ji(t.max_leaders)),
            ("stages".into(), ji(t.stages)),
            ("seconds_per_stage".into(), ji(t.seconds_per_stage)),
            ("leaders_spawned".into(), u32_list(&t.leaders_spawned)),
            ("zoom".into(), ji(t.zoom)),
        ];
        check_state(&got, want)?;
    }
    lookups(op, "police", |c| Ok(scalar::jf(t.police.eval(f32_of(c, "coverage")?))))?;
    lookups(op, "stage_of", |c| Ok(ji(t.stage_of(i64_of(c, "counter")? as i32))))
}

/// `dis_occupant`: `occupant`, `type?` (building), `cell` [x, z], `rect?`
/// [x0, z0, x1, z1], `position?` [x, z] (world), `in_manager?` (true), `lot?`
/// (default occupant + 0x100000; null: none), `flags?` (BDS), `props?`
/// (`{"0xID": number}`; the flammability `0x29244DB5` defaults to 100),
/// `coverage?` (0), `areas?` [top, side] ([0, 0]). An existing occupant is
/// updated (only the given keys).
fn occupant_op(w: &World, op: &Json) -> R<()> {
    let o = tok(field(op, "occupant")?, "occupant")?;
    let mut occs = w.occs.borrow_mut();
    let fresh = !occs.contains_key(&o);
    let e = occs.entry(o).or_insert_with(|| {
        let mut props = BTreeMap::new();
        props.insert(prop::FLAMMABILITY, 100.0);
        Occ {
            otype: occupant::BUILDING,
            in_manager: true,
            lot: Some(o + 0x10_0000),
            props,
            ..Occ::default()
        }
    });
    let _ = fresh;
    if let Some(v) = opt(op, "type") {
        e.otype = occupant_type_of(v)?;
    }
    if let Some(v) = opt(op, "cell") {
        e.cell = pair(v, "cell")?;
    }
    if let Some(v) = opt(op, "rect") {
        let a = arr(v, "rect")?;
        let g = |i: usize| -> R<i32> { Ok(i64_of(a.get(i).ok_or("rect needs 4 values")?, "rect")? as i32) };
        e.rect = Some(CellRect {
            x0: g(0)?,
            z0: g(1)?,
            x1: g(2)?,
            z1: g(3)?,
        });
    }
    if let Some(v) = opt(op, "position") {
        let a = arr(v, "position")?;
        e.position = Some((f32_of(&a[0], "position")?, f32_of(&a[1], "position")?));
    }
    if let Some(v) = opt(op, "in_manager") {
        e.in_manager = bool_of(v, "in_manager")?;
    }
    match op.get("lot") {
        Some(Json::Null) => e.lot = None,
        Some(v) => e.lot = Some(tok(v, "lot")?),
        None => {}
    }
    if let Some(v) = opt(op, "flags") {
        e.flags = u32_of(v, "flags")?;
    }
    if let Some(v) = opt(op, "props") {
        for (k, x) in v.as_object().ok_or("props must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad property id")? as u32;
            match x {
                Json::Null => {
                    e.props.remove(&id);
                }
                Json::Bool(b) => {
                    e.props.insert(id, if *b { 1.0 } else { 0.0 });
                }
                other => {
                    e.props
                        .insert(id, scalar::num(other).ok_or("property values must be numbers")?);
                }
            }
        }
    }
    if let Some(v) = opt(op, "coverage") {
        e.coverage = u32_of(v, "coverage")? as u8;
    }
    if let Some(v) = opt(op, "areas") {
        let a = arr(v, "areas")?;
        e.areas = (f32_of(&a[0], "areas")?, f32_of(&a[1], "areas")?);
    }
    Ok(())
}

/// `dis_cell`: `cell` [x, z] (or `cells`: a list, or `rect` [x0, z0, x1,
/// z1] inclusive) with any of
/// `flammability`, `police`, `road` (bool), `network` (the network
/// occupant's token; null: none), `network_base` (each cell its own network
/// occupant: the base plus the cell's index), `pipe` (a token; null: none),
/// `pipe_neighbours` (for `pipe`: the connected cells).
fn cell_op(w: &World, op: &Json) -> R<()> {
    let cells: Vec<(i32, i32)> = if let Some(v) = opt(op, "cells") {
        arr(v, "cells")?.iter().map(|c| pair(c, "cells")).collect::<R<_>>()?
    } else if let Some(v) = opt(op, "rect") {
        let a = arr(v, "rect")?;
        let g = |i: usize| -> R<i32> { Ok(i64_of(a.get(i).ok_or("rect needs 4 values")?, "rect")? as i32) };
        let (x0, z0, x1, z1) = (g(0)?, g(1)?, g(2)?, g(3)?);
        (z0..=z1).flat_map(|z| (x0..=x1).map(move |x| (x, z))).collect()
    } else {
        vec![pair(field(op, "cell")?, "cell")?]
    };
    let base = opt(op, "network_base").map(|v| tok(v, "network_base")).transpose()?;
    for (index, c) in cells.into_iter().enumerate() {
        if let Some(b) = base {
            w.network.borrow_mut().insert(c, b + index as u64);
        }
        if let Some(v) = opt(op, "flammability") {
            w.flammability.borrow_mut().insert(c, u32_of(v, "flammability")? as u8);
        }
        if let Some(v) = opt(op, "police") {
            w.police.borrow_mut().insert(c, u32_of(v, "police")? as u8);
        }
        match op.get("network") {
            Some(Json::Null) => {
                w.network.borrow_mut().remove(&c);
            }
            Some(v) => {
                w.network.borrow_mut().insert(c, tok(v, "network")?);
            }
            None => {}
        }
        if let Some(v) = opt(op, "road") {
            if bool_of(v, "road")? {
                w.roads.borrow_mut().insert(c);
            } else {
                w.roads.borrow_mut().remove(&c);
            }
        }
        match op.get("pipe") {
            Some(Json::Null) => {
                w.pipes.borrow_mut().remove(&c);
            }
            Some(v) => {
                let p = tok(v, "pipe")?;
                w.pipes.borrow_mut().insert(c, p);
                if let Some(n) = opt(op, "pipe_neighbours") {
                    let list = arr(n, "pipe_neighbours")?
                        .iter()
                        .map(|c| pair(c, "pipe_neighbours"))
                        .collect::<R<_>>()?;
                    w.pipe_neighbours.borrow_mut().insert(p, list);
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// `dis_automaton`: `automaton`, `cell` ([x, z]; null: gone).
fn automaton_op(w: &World, op: &Json) -> R<()> {
    let a = tok(field(op, "automaton")?, "automaton")?;
    let c = match op.get("cell") {
        Some(Json::Null) | None => None,
        Some(v) => Some(pair(v, "cell")?),
    };
    w.automata.borrow_mut().insert(a, c);
    Ok(())
}

/// `dis_clock`: `advance` (ms) or `set` (ms).
fn clock(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "set") {
        w.now.set(i64_of(v, "set")? as u64);
    }
    if let Some(v) = opt(op, "advance") {
        w.now.set(w.now.get() + i64_of(v, "advance")? as u64);
    }
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}
fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}
fn a_tok(op: &Json, i: usize) -> R<u64> {
    tok(arg(op, i)?, "argument")
}
fn a_bool(op: &Json, i: usize) -> R<bool> {
    bool_of(arg(op, i)?, "argument")
}
fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}
fn opt_id(v: Option<u32>) -> Json {
    v.map_or(Json::Null, ji)
}

/// `dis_call`: `method`, `args?`, `expect?`.
fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let m = scalar::str_of(field(op, "method")?, "method")?;
    let got = match m {
        "enable_disasters" => Json::Bool(s.enable_disasters(a_bool(op, 0)?)),
        "disasters_enabled" => Json::Bool(s.disasters_enabled()),
        "has_manager" => Json::Bool(s.has_manager(manager_of(arg(op, 0)?)?)),
        "disaster_has_begun" => {
            s.disaster_has_begun(a_u32(op, 0)?);
            Json::Null
        }
        "disaster_has_ended" => {
            s.disaster_has_ended();
            Json::Null
        }
        "is_disaster_ongoing" => Json::Bool(s.is_disaster_ongoing()),
        "register_location" => {
            s.register_location(
                a_u32(op, 0)?,
                a_i32(op, 1)?,
                a_i32(op, 2)?,
                a_tok(op, 3)?,
                a_u32(op, 4)?,
                a_bool(op, 5)?,
            );
            Json::Null
        }
        "unregister_locations" => {
            s.unregister_locations(a_u32(op, 0)?);
            Json::Null
        }
        "unregister_location_by_occupant" => {
            s.unregister_location_by_occupant(a_u32(op, 0)?, a_tok(op, 1)?);
            Json::Null
        }
        "most_recent_location" => match s.most_recent_location(a_u32(op, 0)?) {
            Some((x, z)) => Json::Array(vec![ji(x), ji(z)]),
            None => Json::Null,
        },
        "location_count" => ji(s.location_count() as i64),
        "go_to_next_disaster" => {
            s.go_to_next_disaster();
            Json::Null
        }
        "disaster_cost" => ji(s.disaster_cost()),
        "reset_disaster_cost" => {
            s.reset_disaster_cost();
            Json::Null
        }
        "attempt_start" => opt_id(s.attempt_start(manager_of(arg(op, 0)?)?, a_i32(op, 1)?, a_i32(op, 2)?)),
        "attempt_start_any" => opt_id(s.attempt_start_any(manager_of(arg(op, 0)?)?)),
        "active_count" => ji(s.active_count(manager_of(arg(op, 0)?)?)),
        "manager_instances" => Json::Array(
            s.manager_instances(manager_of(arg(op, 0)?)?)
                .into_iter()
                .map(ji)
                .collect(),
        ),
        "instance_shutdown" => {
            s.instance_shutdown(a_u32(op, 0)?);
            Json::Null
        }
        "fire_instance" => ji(s.fire_instance()),
        "ignite_occupant" => Json::Bool(s.ignite_occupant(a_tok(op, 0)?)),
        "ignite" => Json::Bool(s.ignite(a_tok(op, 0)?, a_bool(op, 1)?, a_u32(op, 2)?, a_bool(op, 3)?)),
        "ignite_cell" => Json::Bool(s.ignite_cell(
            a_i32(op, 0)?,
            a_i32(op, 1)?,
            a_bool(op, 2)?,
            a_bool(op, 3)?,
            a_u32(op, 4)?,
        )),
        "manager_ignite_cell" => Json::Bool(s.manager_ignite_cell(
            a_i32(op, 0)?,
            a_i32(op, 1)?,
            a_bool(op, 2)?,
            a_bool(op, 3)?,
            a_u32(op, 4)?,
        )),
        "extinguish_cell" => {
            s.extinguish_cell(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?);
            Json::Null
        }
        "quick_burn_cell" => {
            s.quick_burn_cell(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Null
        }
        "ignite_cell_radius" => {
            s.ignite_cell_radius(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_i32(op, 3)?);
            Json::Null
        }
        "extinguish" => {
            s.extinguish(a_tok(op, 0)?);
            Json::Null
        }
        "burn_down" => {
            s.burn_down(a_tok(op, 0)?);
            Json::Null
        }
        "set_cause" => ji(s.set_cause(a_u32(op, 0)?)),
        "cause" => ji(s.cause()),
        "fire_count" => ji(s.fire_count()),
        "fire_count_in_rect" => ji(s.fire_count_in_rect(CellRect {
            x0: a_i32(op, 0)?,
            z0: a_i32(op, 1)?,
            x1: a_i32(op, 2)?,
            z1: a_i32(op, 3)?,
        })),
        "fire_update" => {
            s.fire_update();
            Json::Null
        }
        "fire_set_hosed" => Json::Bool(s.fire_set_hosed(a_tok(op, 0)?, a_i32(op, 1)?)),
        "spill_toxic_waste" => opt_id(s.spill_toxic_waste(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?)),
        "toxic_set_hosed" => Json::Bool(s.toxic_set_hosed(a_tok(op, 0)?, a_i32(op, 1)?)),
        "volcano_request_end" => Json::Bool(s.volcano_request_end(a_u32(op, 0)?)),
        "start_disaster_effect" => opt_id(s.start_disaster_effect(a_tok(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?)),
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

/// `dis_message`: `target`, `type`, `data1..3?`, `void1?`, `x?`, `z?`,
/// `expect?`.
fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let t = target_of(field(op, "target")?)?;
    let d = |k: &str| -> R<i64> { opt(op, k).map_or(Ok(0), |v| i64_of(v, k)) };
    let m = DisasterMessage {
        msg_type: message_type_of(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: d("data3")?,
        void1: d("void1")? as u64,
        x: opt(op, "x").map_or(Ok(0.0), |v| f32_of(v, "x"))?,
        z: opt(op, "z").map_or(Ok(0.0), |v| f32_of(v, "z"))?,
    };
    check_expect(op, Json::Bool(s.do_message(t, &m)))
}

/// `dis_tick`: `instance` (an ID or "fire"), `us?` (16000), `count?` (1),
/// `advance_ms?` (0, added to the clock before each tick).
fn tick(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let id = match field(op, "instance")? {
        Json::Str(x) if x == "fire" => s.fire_instance(),
        v => u32_of(v, "instance")?,
    };
    let us = opt(op, "us").map_or(Ok(16_000), |v| u32_of(v, "us"))?;
    let n = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    let adv = opt(op, "advance_ms").map_or(Ok(0), |v| i64_of(v, "advance_ms"))? as u64;
    for _ in 0..n {
        w.now.set(w.now.get() + adv);
        s.tick(id, us);
    }
    Ok(())
}

/// `dis_seconds`: `count` times, one second of wall clock then one tick of
/// `instance` ("fire" by default: the fire's `Update`).
fn seconds(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let n = i64_of(field(op, "count")?, "count")?;
    let id = match op.get("instance") {
        None | Some(Json::Null) => None,
        Some(Json::Str(x)) if x == "fire" => None,
        Some(v) => Some(u32_of(v, "instance")?),
    };
    for _ in 0..n {
        w.now.set(w.now.get() + 1000);
        match id {
            None => s.fire_update(),
            Some(i) => s.tick(i, 16_000),
        }
    }
    Ok(())
}

// ---- checks ---------------------------------------------------------------------------------------

fn fire_json(e: &FireEntry) -> Json {
    Json::Object(vec![
        ("occupant".into(), ji(e.occupant as i64)),
        ("target".into(), ji(e.target as i64)),
        ("counter".into(), ji(e.counter)),
        ("stage".into(), ji(e.stage)),
        ("max_stage".into(), ji(e.max_stage)),
        ("fire_id".into(), ji(e.fire_id)),
        ("hosed".into(), ji(e.hosed)),
        (
            "extinguish_at".into(),
            e.extinguish_at.map_or(Json::Null, |v| ji(v as i64)),
        ),
        ("top_flames".into(), ji(e.top_flames)),
        ("side_flames".into(), ji(e.side_flames)),
    ])
}

/// `dis_expect`: `state` (partial) over the layer, the fire, the toxic
/// spill and the world's counters.
fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let l = s.layer_snapshot();
    let pairs = |v: &[(u32, u64)]| {
        Json::Array(
            v.iter()
                .map(|(k, t)| Json::Array(vec![ji(*k), ji(*t as i64)]))
                .collect(),
        )
    };
    let mut got: Vec<(String, Json)> = vec![
        ("initialised".into(), Json::Bool(l.initialised)),
        ("enabled".into(), Json::Bool(l.disable_counter == 0)),
        ("disable_counter".into(), ji(l.disable_counter)),
        ("ongoing".into(), ji(l.ongoing)),
        ("assessing".into(), Json::Bool(l.assessing)),
        ("building_count".into(), ji(l.damage.building_count)),
        ("total_cost".into(), ji(l.damage.total_cost)),
        (
            "kinds".into(),
            Json::Array(
                l.damage
                    .kinds
                    .iter()
                    .map(|(c, v)| Json::Array(vec![ji(*c), ji(*v)]))
                    .collect(),
            ),
        ),
        ("special_count".into(), ji(l.damage.special.len() as i64)),
        ("fire_timers".into(), pairs(&l.fire_timers)),
        ("police_timers".into(), pairs(&l.police_timers)),
        ("overall_s".into(), ji(l.overall_s as i64)),
        ("fire_effectiveness".into(), scalar::jf(l.fire_effectiveness)),
        ("police_effectiveness".into(), scalar::jf(l.police_effectiveness)),
        ("cursor".into(), l.cursor.map_or(Json::Null, |c| ji(c as i64))),
        (
            "locations".into(),
            Json::Array(
                l.locations
                    .iter()
                    .map(|x| {
                        Json::Object(vec![
                            ("instance".into(), ji(x.instance)),
                            ("x".into(), ji(x.x)),
                            ("z".into(), ji(x.z)),
                            ("zoom".into(), ji(x.zoom)),
                            ("occupant".into(), ji(x.occupant as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "managers".into(),
            Json::Array(l.managers.iter().map(|m| ji(*m)).collect()),
        ),
        ("emergency".into(), ji(w.emergency.get())),
        ("save_disabled".into(), ji(w.save_disabled.get())),
        ("fire_count".into(), ji(s.fire_count())),
        ("reentrant_refused".into(), ji(s.reentrant_refused())),
        (
            "riot_locked".into(),
            Json::Array(s.riot_locked().iter().map(|t| ji(*t as i64)).collect()),
        ),
    ];
    if let Some(f) = s.fire_snapshot() {
        got.extend([
            ("fire_phase".into(), ji(f.phase)),
            ("fires".into(), Json::Array(f.fires.iter().map(fire_json).collect())),
            (
                "fire_ids".into(),
                Json::Array(f.fire_ids.iter().map(|i| ji(*i)).collect()),
            ),
            (
                "toast".into(),
                Json::Array(f.toast.iter().map(|t| ji(*t as i64)).collect()),
            ),
            ("cause".into(), ji(f.cause)),
            ("elapsed_ms".into(), ji(f.elapsed_ms as i64)),
            ("next_ms".into(), ji(f.next_ms as i64)),
            ("toasted".into(), ji(f.toasted)),
            ("fire_tick".into(), Json::Bool(f.tick_registered)),
            (
                "d3cum".into(),
                Json::Array(f.tuning.d3cum.iter().map(|v| ji(*v)).collect()),
            ),
            ("zoom".into(), ji(f.tuning.zoom)),
        ]);
    }
    if let Some(t) = s.toxic_snapshot() {
        got.extend([
            ("toxic_instance".into(), ji(t.id)),
            (
                "toxic_amounts".into(),
                Json::Array(t.spills.iter().map(|x| ji(x.amount)).collect()),
            ),
        ]);
    } else {
        got.push(("toxic_instance".into(), Json::Null));
    }
    check_state(&got, field(op, "state")?)
}

/// `dis_expect_fire`: `index` (in the list) and `state` (partial), or
/// `absent: true` (no fire at that index).
fn expect_fire(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let i = opt(op, "index").map_or(Ok(0), |v| i64_of(v, "index"))? as usize;
    let f = s.fire_snapshot().and_then(|f| f.fires.get(i).copied());
    if opt(op, "absent").map_or(Ok(false), |v| bool_of(v, "absent"))? {
        return match f {
            None => Ok(()),
            Some(_) => Err(format!("fire {i} exists")),
        };
    }
    let f = f.ok_or_else(|| format!("no fire {i}"))?;
    let Json::Object(kv) = fire_json(&f) else {
        unreachable!()
    };
    check_state(&kv, field(op, "state")?)
}

/// `dis_expect_riot`: `instance`, `state` (partial: `phase`, `leader_count`,
/// `elapsed_s`, `next_s`, `leaders` ([{automaton, counter, stage, max_stage, cell}])), or
/// `absent: true`.
fn expect_riot(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let id = u32_of(field(op, "instance")?, "instance")?;
    let r = s.riot_snapshot(id);
    if opt(op, "absent").map_or(Ok(false), |v| bool_of(v, "absent"))? {
        return match r {
            None => Ok(()),
            Some(_) => Err(format!("riot {id} exists")),
        };
    }
    let r = r.ok_or_else(|| format!("no riot {id}"))?;
    let got = vec![
        ("phase".to_string(), ji(r.phase)),
        ("leader_count".into(), ji(r.leaders.len() as i64)),
        ("elapsed_s".into(), ji(r.elapsed_s as i64)),
        ("next_s".into(), ji(r.next_s as i64)),
        (
            "leaders".into(),
            Json::Array(
                r.leaders
                    .iter()
                    .map(|l| {
                        Json::Object(vec![
                            ("automaton".into(), ji(l.automaton as i64)),
                            ("counter".into(), ji(l.counter)),
                            ("stage".into(), ji(l.stage)),
                            ("max_stage".into(), ji(l.max_stage)),
                            ("cell".into(), Json::Array(vec![ji(l.cell.0), ji(l.cell.1)])),
                        ])
                    })
                    .collect(),
            ),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `dis_expect_instance`: `instance` and `state` (partial: `type`, `class`,
/// `phase`, `location` ([x, z] or null), `tick`), or `absent: true`.
fn expect_instance(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let id = u32_of(field(op, "instance")?, "instance")?;
    let i = s.instance_info(id);
    if opt(op, "absent").map_or(Ok(false), |v| bool_of(v, "absent"))? {
        return match i {
            None => Ok(()),
            Some(_) => Err(format!("instance {id} exists")),
        };
    }
    let i = i.ok_or_else(|| format!("no instance {id}"))?;
    let got = vec![
        ("type".to_string(), ji(i.mtype)),
        ("class".into(), ji(i.class)),
        ("phase".into(), ji(i.phase)),
        (
            "location".into(),
            i.location.map_or(Json::Null, |(x, z)| Json::Array(vec![ji(x), ji(z)])),
        ),
        ("tick".into(), Json::Bool(i.tick_registered)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `dis_expect_volcano`: `instance` and `state` (partial: `phase` (its
/// `GetPhase`), `state`, `lava_us`, `end_requested`, `lava_flowing`,
/// `lava_has_run`, `lava_phases`), or `absent: true`.
fn expect_volcano(w: &World, op: &Json) -> R<()> {
    let s = w.sys()?;
    let id = u32_of(field(op, "instance")?, "instance")?;
    let v = s.volcano_snapshot(id);
    if opt(op, "absent").map_or(Ok(false), |v| bool_of(v, "absent"))? {
        return match v {
            None => Ok(()),
            Some(_) => Err(format!("volcano {id} exists")),
        };
    }
    let v = v.ok_or_else(|| format!("no volcano {id}"))?;
    let got = vec![
        ("phase".to_string(), ji(v.phase)),
        ("state".into(), ji(v.state)),
        ("lava_us".into(), ji(v.lava_us as i64)),
        ("end_requested".into(), Json::Bool(v.end_requested)),
        ("lava_flowing".into(), Json::Bool(v.lava_flowing)),
        ("lava_has_run".into(), Json::Bool(v.lava_has_run)),
        ("lava_phases".into(), ji(v.lava_phases)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `dis_expect_refs`: `refs` (`{"token": count}`; references held by the
/// core).
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let want = field(op, "refs")?.as_object().ok_or("refs must be an object")?;
    for (k, v) in want {
        let t = Json::Str(k.clone()).as_i64().ok_or("bad token")? as u64;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        let n = i64_of(v, "count")?;
        if got != n {
            return Err(format!("token {k}: {got} references, expected {n}"));
        }
    }
    Ok(())
}
