// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! [`Disasters`]: the layer, the manager framework of §3.1 and every
//! instance in one object, with the re-entrancy guard of the module
//! documentation.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use super::effect::EffectState;
use super::fire::{FireSnapshot, FireState, FireTuning};
use super::layer::{LayerSnapshot, LayerState, Location};
use super::pipe::{PipeSnapshot, PipeState, PipeTuning};
use super::quake::{QuakeSnapshot, QuakeState, QuakeTuning};
use super::riot::{RiotSnapshot, RiotState, RiotTuning};
use super::toxic::{ToxicSnapshot, ToxicState};
use super::volcano::{VolcanoSnapshot, VolcanoState, VolcanoTuning};
use super::{
    instance_class, kind, msg, seed, CellRect, DerivedSeeds, DisasterHost, DisasterMessage, DisasterRandom, Generator,
    Target,
};
use crate::rng::{derive_seed, RzRandom};

/// A manager (§3.1): its instance list, AddRef'd, with the deferred add and
/// remove lists used while its `Shutdown` walks the list.
#[derive(Clone, Debug, Default)]
pub(crate) struct Manager {
    pub mtype: u32,
    pub class: u32,
    pub instances: Vec<u32>,
    pub shutting_down: bool,
    pub pending_add: Vec<u32>,
    pub pending_remove: Vec<u32>,
    pub initialised: bool,
}

/// A manager's observable state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManagerSnapshot {
    pub mtype: u32,
    pub class: u32,
    pub initialised: bool,
    pub instances: Vec<u32>,
    pub active: u32,
}

pub(crate) enum InstanceState {
    Fire(Box<FireState>),
    Riot(Box<RiotState>),
    Quake(Box<QuakeState>),
    Pipe(Box<PipeState>),
    Toxic(Box<ToxicState>),
    Volcano(Box<VolcanoState>),
    Effect(Box<EffectState>),
}

pub(crate) struct Instance {
    pub mtype: u32,
    pub class: u32,
    /// 0 idle, 1 active, 3 ended (§3.2).
    pub phase: u32,
    pub tick_registered: bool,
    pub subscriptions: Vec<u32>,
    pub state: InstanceState,
}

/// An instance's `cISC4DisasterInstance` answers (§3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstanceInfo {
    pub id: u32,
    /// `GetID`: the manager type.
    pub mtype: u32,
    pub class: u32,
    pub phase: u32,
    /// `GetLocation`: `None` = false (the fire, §3.2).
    pub location: Option<(i32, i32)>,
    pub tick_registered: bool,
}

pub(crate) struct Core {
    pub host: Rc<dyn DisasterHost>,
    pub session_seed: u32,
    pub layer: LayerState,
    pub managers: BTreeMap<u32, Manager>,
    pub instances: BTreeMap<u32, Instance>,
    pub next_id: u32,
    pub rng_fire: Box<dyn DisasterRandom>,
    pub rng_riot: Box<dyn DisasterRandom>,
    pub rng_quake: Box<dyn DisasterRandom>,
    pub rng_pipe: Box<dyn DisasterRandom>,
    /// A scripted throwaway generator for the argument-less pipe burst.
    pub rng_pipe_start: Option<Box<dyn DisasterRandom>>,
    pub pipe_start_count: u32,
    pub fire_instance: Option<u32>,
    pub toxic_instance: Option<u32>,
    pub fire_tuning: Option<FireTuning>,
    pub riot_tuning: Option<RiotTuning>,
    /// The network occupants the riots have locked (network-manager.md §10).
    pub riot_locked: Vec<u64>,
    pub quake_tuning: Option<QuakeTuning>,
    pub pipe_tuning: Option<PipeTuning>,
    pub volcano_tuning: Option<VolcanoTuning>,
}

fn default_rng(session: u32, k: u32) -> Box<dyn DisasterRandom> {
    let s = derive_seed(session, k);
    Box::new(RzRandom::new(s, &DerivedSeeds(s)))
}

impl Core {
    fn new(host: Rc<dyn DisasterHost>, session_seed: u32) -> Core {
        Core {
            host,
            session_seed,
            layer: LayerState::default(),
            managers: BTreeMap::new(),
            instances: BTreeMap::new(),
            next_id: 1,
            rng_fire: default_rng(session_seed, seed::FIRE),
            rng_riot: default_rng(session_seed, seed::RIOT),
            rng_quake: default_rng(session_seed, seed::EARTHQUAKE),
            rng_pipe: default_rng(session_seed, seed::PIPE_BURST),
            rng_pipe_start: None,
            pipe_start_count: 0,
            fire_instance: None,
            toxic_instance: None,
            fire_tuning: None,
            riot_tuning: None,
            riot_locked: Vec::new(),
            quake_tuning: None,
            pipe_tuning: None,
            volcano_tuning: None,
        }
    }

    // ---- the manager framework (§3.1) ----

    pub(crate) fn create_manager(&mut self, mtype: u32, class: u32) {
        self.managers.insert(
            mtype,
            Manager {
                mtype,
                class,
                ..Manager::default()
            },
        );
    }

    /// Manager `Init` (slot 3). Only the toxic manager has a behaviour the
    /// spec gives: it subscribes to the "ToxicSpill" cheat (§9). The fire
    /// manager creates its instance on first use (flammability §13.1)
    /// [choice].
    pub(crate) fn manager_init(&mut self, mtype: u32) {
        let Some(m) = self.managers.get_mut(&mtype) else { return };
        if m.initialised {
            return;
        }
        m.initialised = true;
        if mtype == kind::TOXIC {
            self.host
                .subscribe(Target::Manager(kind::TOXIC), msg::TOXIC_SPILL_CHEAT);
        }
    }

    /// Manager `Shutdown` (slot 4): each instance's `Shutdown`, then its
    /// release; adds and removes during the walk go to the pending lists.
    pub(crate) fn manager_shutdown(&mut self, mtype: u32) {
        let Some(m) = self.managers.get_mut(&mtype) else { return };
        m.shutting_down = true;
        let list = m.instances.clone();
        for id in list {
            self.instance_shutdown(id);
        }
        let Some(m) = self.managers.get_mut(&mtype) else { return };
        m.shutting_down = false;
        let pending_add = std::mem::take(&mut m.pending_add);
        let pending_remove = std::mem::take(&mut m.pending_remove);
        let mut released = std::mem::take(&mut m.instances);
        released.extend(pending_remove);
        m.instances = pending_add;
        if mtype == kind::RIOT {
            self.clear_riot_network();
        }
        let Some(m) = self.managers.get_mut(&mtype) else { return };
        if m.initialised && mtype == kind::TOXIC {
            self.host
                .unsubscribe(Target::Manager(kind::TOXIC), msg::TOXIC_SPILL_CHEAT);
        }
        if let Some(m) = self.managers.get_mut(&mtype) {
            m.initialised = false;
        }
        for id in released {
            self.release_instance(id);
        }
    }

    /// `AddInstanceToList` (slot 8).
    pub(crate) fn add_instance_to_list(&mut self, mtype: u32, id: u32) {
        let Some(m) = self.managers.get_mut(&mtype) else { return };
        if m.shutting_down {
            m.pending_add.push(id);
        } else if !m.instances.contains(&id) {
            m.instances.push(id);
        }
    }

    /// `RemoveInstanceFromList` (slot 9); the instance is released (dropped)
    /// when no list holds it any more.
    pub(crate) fn remove_instance_from_list(&mut self, mtype: u32, id: u32) {
        let Some(m) = self.managers.get_mut(&mtype) else {
            self.release_instance(id);
            return;
        };
        if m.shutting_down {
            m.pending_remove.push(id);
            return;
        }
        m.instances.retain(|i| *i != id);
        let last = m.instances.is_empty();
        self.release_instance(id);
        if last && mtype == kind::RIOT {
            self.clear_riot_network();
        }
    }

    fn release_instance(&mut self, id: u32) {
        if self.instances.remove(&id).is_some() {
            if self.fire_instance == Some(id) {
                self.fire_instance = None;
            }
            if self.toxic_instance == Some(id) {
                self.toxic_instance = None;
            }
        }
    }

    /// `GetActiveInstanceCount` (slot 10): instances in phase 1.
    pub(crate) fn active_count(&self, mtype: u32) -> u32 {
        self.managers.get(&mtype).map_or(0, |m| {
            m.instances
                .iter()
                .filter(|i| self.instances.get(i).is_some_and(|x| x.phase == 1))
                .count() as u32
        })
    }

    /// A new instance of a manager, not yet in its list.
    pub(crate) fn new_instance(&mut self, mtype: u32, class: u32, state: InstanceState) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.instances.insert(
            id,
            Instance {
                mtype,
                class,
                phase: 0,
                tick_registered: false,
                subscriptions: Vec::new(),
                state,
            },
        );
        id
    }

    pub(crate) fn register_tick(&mut self, id: u32, priority: u32) {
        let host = self.host.clone();
        if let Some(i) = self.instances.get_mut(&id) {
            if !i.tick_registered {
                i.tick_registered = true;
                host.register_tick(id, priority);
            }
        }
    }

    pub(crate) fn unregister_tick(&mut self, id: u32) {
        let host = self.host.clone();
        if let Some(i) = self.instances.get_mut(&id) {
            if i.tick_registered {
                i.tick_registered = false;
                host.unregister_tick(id);
            }
        }
    }

    pub(crate) fn subscribe(&mut self, id: u32, m: u32) {
        let host = self.host.clone();
        if let Some(i) = self.instances.get_mut(&id) {
            if !i.subscriptions.contains(&m) {
                i.subscriptions.push(m);
                host.subscribe(Target::Instance(id), m);
            }
        }
    }

    fn unsubscribe_all(&mut self, id: u32) {
        let host = self.host.clone();
        if let Some(i) = self.instances.get_mut(&id) {
            for m in std::mem::take(&mut i.subscriptions) {
                host.unsubscribe(Target::Instance(id), m);
            }
        }
    }

    /// An instance's `Shutdown` (slot 3): the kind's own cleanup, then the
    /// tick and the subscriptions go. It does not end the disaster
    /// (`DisasterHasEnded` belongs to each kind's own end) [choice, Q13].
    pub(crate) fn instance_shutdown(&mut self, id: u32) {
        let Some(i) = self.instances.get(&id) else { return };
        match i.state {
            InstanceState::Fire(_) => self.fire_shutdown(id),
            InstanceState::Riot(_) => self.riot_shutdown(id),
            InstanceState::Toxic(_) => self.toxic_shutdown(id),
            InstanceState::Pipe(_) => self.pipe_shutdown(id),
            _ => {}
        }
        self.unregister_tick(id);
        self.unsubscribe_all(id);
    }

    /// A kind's own end: shutdown, then leave the manager.
    pub(crate) fn instance_finish(&mut self, id: u32) {
        let mtype = self.instance_type(id);
        self.instance_shutdown(id);
        self.remove_instance_from_list(mtype, id);
    }

    // ---- dispatch ----

    pub(crate) fn tick(&mut self, id: u32, us: u32) {
        let Some(i) = self.instances.get(&id) else { return };
        match i.state {
            InstanceState::Fire(_) => self.fire_update(),
            InstanceState::Riot(_) => self.riot_tick(id),
            InstanceState::Toxic(_) => self.toxic_tick(id),
            InstanceState::Quake(_) => self.quake_tick(id, us),
            InstanceState::Pipe(_) => self.pipe_tick(id, us),
            InstanceState::Volcano(_) => self.volcano_tick(id, us),
            InstanceState::Effect(_) => {}
        }
    }

    pub(crate) fn do_message(&mut self, target: Target, m: &DisasterMessage) -> bool {
        match target {
            Target::Layer => match m.msg_type {
                msg::OCCUPANT_REMOVED => {
                    self.on_occupant_removed(m.void1);
                    true
                }
                msg::DISPATCH_ACTIVE | msg::DISPATCH_BEGIN | msg::DISPATCH_END | msg::DISPATCH_INACTIVE => {
                    self.on_dispatch(m.msg_type, m.data1, m.data2);
                    true
                }
                _ => false,
            },
            Target::Manager(kind::TOXIC) if m.msg_type == msg::TOXIC_SPILL_CHEAT => {
                self.spill_toxic_waste(m.data1 as i32, m.data2 as i32, m.data3 as i32);
                true
            }
            Target::Manager(_) => false,
            Target::Instance(id) => {
                let Some(i) = self.instances.get(&id) else { return false };
                match i.state {
                    // The view messages only refresh the flame visuals (§5.3
                    // step 6); the hour has no agent (§5.8): host-side.
                    InstanceState::Fire(_) => true,
                    InstanceState::Pipe(_) => {
                        self.pipe_message(id, m);
                        true
                    }
                    InstanceState::Effect(_) => self.effect_message(id, m),
                    InstanceState::Volcano(_) => true,
                    _ => false,
                }
            }
        }
    }

    pub(crate) fn instance_info(&self, id: u32) -> Option<InstanceInfo> {
        let i = self.instances.get(&id)?;
        let location = match &i.state {
            InstanceState::Quake(q) => Some(q.cell),
            InstanceState::Volcano(v) => Some(v.cell),
            InstanceState::Effect(e) => Some(e.cell),
            _ => None,
        };
        Some(InstanceInfo {
            id,
            mtype: i.mtype,
            class: i.class,
            phase: i.phase,
            location,
            tick_registered: i.tick_registered,
        })
    }

    /// `AttemptDisasterStart(x, z)` (manager slot 6).
    pub(crate) fn attempt_start(&mut self, mtype: u32, x: i32, z: i32) -> Option<u32> {
        if !self.managers.contains_key(&mtype) {
            return None;
        }
        match mtype {
            kind::FIRE => {
                self.fire_ignite_cell_manager(x, z, false, false, 0);
                None
            }
            kind::RIOT => self.riot_attempt_start(x, z),
            kind::TOXIC => self.toxic_attempt_start(x, z),
            kind::EARTHQUAKE => self.quake_attempt_start(x, z),
            kind::PIPE_BURST => self.pipe_attempt_start(x, z),
            kind::VOLCANO => self.volcano_attempt_start(x, z),
            _ => None,
        }
    }

    /// The argument-less `AttemptDisasterStart()` (slot 7): the pipe burst's
    /// random start (§8); null for the others (foundations §17 for the
    /// earthquake; the rest are not specified) [choice].
    pub(crate) fn attempt_start_any(&mut self, mtype: u32) -> Option<u32> {
        if mtype == kind::PIPE_BURST && self.managers.contains_key(&mtype) {
            return self.pipe_attempt_start_random();
        }
        None
    }

    /// The city's size in world units (cells × cell width).
    pub(crate) fn city_world_size(&self) -> (f32, f32) {
        let (cx, cz) = self.host.city_cells();
        let w = self.host.cell_width();
        (cx as f32 * w, cz as f32 * w)
    }

    /// The cell of a world position (floor of position / cell width).
    pub(crate) fn world_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let w = self.host.cell_width();
        if w.is_nan() || w <= 0.0 {
            return (0, 0);
        }
        (super::trunc_i32((x / w).floor()), super::trunc_i32((z / w).floor()))
    }

    pub(crate) fn inside_city(&self, x: i32, z: i32) -> bool {
        let (cx, cz) = self.host.city_cells();
        x >= 0 && z >= 0 && (x as i64) < i64::from(cx) && (z as i64) < i64::from(cz)
    }

    pub(crate) fn clamp_rect(&self, r: CellRect) -> CellRect {
        let (cx, cz) = self.host.city_cells();
        let mx = (cx as i32 - 1).max(0);
        let mz = (cz as i32 - 1).max(0);
        CellRect {
            x0: r.x0.clamp(0, mx),
            z0: r.z0.clamp(0, mz),
            x1: r.x1.clamp(0, mx),
            z1: r.z1.clamp(0, mz),
        }
    }
}

/// The disaster layer, its managers and their instances. Main-thread only
/// (foundations §19.3). See the module documentation for re-entrancy.
pub struct Disasters {
    core: RefCell<Core>,
    deferred: RefCell<VecDeque<(Target, DisasterMessage)>>,
    refused: Cell<u32>,
}

impl Disasters {
    /// The layer, constructed (not yet `Init`). `session_seed` seeds the
    /// generators (foundations §6.3).
    pub fn new(host: Rc<dyn DisasterHost>, session_seed: u32) -> Disasters {
        Disasters {
            core: RefCell::new(Core::new(host, session_seed)),
            deferred: RefCell::new(VecDeque::new()),
            refused: Cell::new(0),
        }
    }

    fn with<R>(&self, fallback: R, f: impl FnOnce(&mut Core) -> R) -> R {
        let r = match self.core.try_borrow_mut() {
            Ok(mut c) => f(&mut c),
            Err(_) => {
                self.refused.set(self.refused.get().wrapping_add(1));
                return fallback;
            }
        };
        self.drain();
        r
    }

    fn read<R>(&self, fallback: R, f: impl FnOnce(&Core) -> R) -> R {
        match self.core.try_borrow() {
            Ok(c) => f(&c),
            Err(_) => {
                self.refused.set(self.refused.get().wrapping_add(1));
                fallback
            }
        }
    }

    fn drain(&self) {
        loop {
            let Ok(mut c) = self.core.try_borrow_mut() else { return };
            let Some((t, m)) = self.deferred.borrow_mut().pop_front() else {
                return;
            };
            c.do_message(t, &m);
        }
    }

    /// Replaces a generator (tests and replays).
    pub fn set_random(&self, g: Generator, r: Box<dyn DisasterRandom>) {
        self.with((), |c| match g {
            Generator::Fire => c.rng_fire = r,
            Generator::Riot => c.rng_riot = r,
            Generator::Earthquake => c.rng_quake = r,
            Generator::PipeBurst => c.rng_pipe = r,
            Generator::PipeBurstStart => c.rng_pipe_start = Some(r),
        })
    }

    /// Re-entrant calls refused so far (diagnostics).
    pub fn reentrant_refused(&self) -> u32 {
        self.refused.get()
    }

    /// Messages waiting for the core (non-zero only inside a callback).
    pub fn deferred_messages(&self) -> usize {
        self.deferred.borrow().len()
    }

    // ---- the layer (§2) ----

    pub fn init(&self) -> bool {
        self.with(false, Core::layer_init)
    }
    pub fn shutdown(&self) -> bool {
        self.with(false, Core::layer_shutdown)
    }
    /// `EnableDisasters` (slot 5).
    pub fn enable_disasters(&self, enable: bool) -> bool {
        self.with(false, |c| c.enable_disasters(enable))
    }
    /// `DisastersEnabled` (slot 6).
    pub fn disasters_enabled(&self) -> bool {
        self.read(false, Core::disasters_enabled)
    }
    /// `GetDisasterManager(type)` (slot 7): whether the map holds it.
    pub fn has_manager(&self, mtype: u32) -> bool {
        self.read(false, |c| c.managers.contains_key(&mtype))
    }
    pub fn disaster_has_begun(&self, instance: u32) {
        self.with((), |c| c.disaster_has_begun(instance))
    }
    pub fn disaster_has_ended(&self) {
        self.with((), Core::disaster_has_ended)
    }
    pub fn is_disaster_ongoing(&self) -> bool {
        self.read(false, Core::is_disaster_ongoing)
    }
    pub fn register_location(&self, instance: u32, x: i32, z: i32, occ: u64, zoom: u32, focus: bool) {
        self.with((), |c| c.register_location(instance, x, z, occ, zoom, focus))
    }
    pub fn unregister_locations(&self, instance: u32) {
        self.with((), |c| c.unregister_locations(instance))
    }
    pub fn unregister_location_by_occupant(&self, instance: u32, occ: u64) {
        self.with((), |c| c.unregister_location_by_occupant(instance, occ))
    }
    /// `GetMostRecentDisasterLocation` (slot 12); `instance` 0 = any.
    pub fn most_recent_location(&self, instance: u32) -> Option<(i32, i32)> {
        self.read(None, |c| c.most_recent_location((instance != 0).then_some(instance)))
    }
    pub fn location_count(&self) -> usize {
        self.read(0, |c| c.layer.locations.len())
    }
    pub fn location_by_index(&self, i: usize) -> Option<Location> {
        self.read(None, |c| c.location_by_index(i))
    }
    pub fn go_to_next_disaster(&self) {
        self.with((), Core::go_to_next_disaster)
    }
    /// `GetDisasterCost` (slot 18).
    pub fn disaster_cost(&self) -> i64 {
        self.read(0, |c| c.layer.damage.total_cost)
    }
    pub fn reset_disaster_cost(&self) {
        self.with((), Core::reset_cost)
    }
    pub fn layer_snapshot(&self) -> LayerSnapshot {
        self.read(LayerSnapshot::default(), Core::layer_snapshot)
    }

    // ---- messages and ticks ----

    /// A message for the layer, a manager or an instance. A message that
    /// arrives while the core is busy is deferred (returns true).
    pub fn do_message(&self, target: Target, m: &DisasterMessage) -> bool {
        match self.core.try_borrow_mut() {
            Ok(mut c) => {
                let r = c.do_message(target, m);
                drop(c);
                self.drain();
                r
            }
            Err(_) => {
                self.deferred.borrow_mut().push_back((target, *m));
                true
            }
        }
    }

    /// The animation tick (`0x86AD10EE`, data2 = µs) for an instance.
    pub fn tick(&self, instance: u32, us: u32) {
        self.with((), |c| c.tick(instance, us))
    }

    // ---- managers (§3) ----

    pub fn attempt_start(&self, mtype: u32, x: i32, z: i32) -> Option<u32> {
        self.with(None, |c| c.attempt_start(mtype, x, z))
    }
    pub fn attempt_start_any(&self, mtype: u32) -> Option<u32> {
        self.with(None, |c| c.attempt_start_any(mtype))
    }
    pub fn active_count(&self, mtype: u32) -> u32 {
        self.read(0, |c| c.active_count(mtype))
    }
    pub fn manager_snapshot(&self, mtype: u32) -> Option<ManagerSnapshot> {
        self.read(None, |c| {
            let m = c.managers.get(&mtype)?;
            Some(ManagerSnapshot {
                mtype: m.mtype,
                class: m.class,
                initialised: m.initialised,
                instances: m.instances.clone(),
                active: c.active_count(mtype),
            })
        })
    }
    pub fn instance_info(&self, id: u32) -> Option<InstanceInfo> {
        self.read(None, |c| c.instance_info(id))
    }
    /// An instance's `Shutdown` (instance slot 3). It stays in its manager's
    /// list until the manager removes it.
    pub fn instance_shutdown(&self, id: u32) {
        self.with((), |c| c.instance_shutdown(id))
    }

    // ---- fire (§5) ----

    /// The fire manager's instance (`GetFireInstance`, manager slot 11),
    /// created and `Init`ed on first use; 0 without a fire manager.
    pub fn fire_instance(&self) -> u32 {
        self.with(0, |c| c.ensure_fire().unwrap_or(0))
    }
    /// Manager slot 4 (`IgniteOccupant`): true for any non-null occupant.
    pub fn ignite_occupant(&self, o: u64) -> bool {
        self.with(false, |c| c.fire_ignite_occupant(o))
    }
    /// Instance slot 6 (`Ignite`).
    pub fn ignite(&self, o: u64, force: bool, fire_id: u32, focus: bool) -> bool {
        self.with(false, |c| {
            c.ensure_fire().is_some() && c.fire_ignite(o, force, fire_id, focus)
        })
    }
    /// Instance slot 7 (`IgniteCell`): whether any occupant ignited.
    pub fn ignite_cell(&self, x: i32, z: i32, use_probability: bool, teleport: bool, fire_id: u32) -> bool {
        self.with(false, |c| {
            c.ensure_fire().is_some() && c.fire_ignite_cell(x, z, use_probability, teleport, fire_id)
        })
    }
    /// Manager slot 5 (`IgniteCell`): forwards, true whenever the manager
    /// exists (flammability §13.1).
    pub fn manager_ignite_cell(&self, x: i32, z: i32, use_probability: bool, teleport: bool, fire_id: u32) -> bool {
        self.with(false, |c| {
            c.fire_ignite_cell_manager(x, z, use_probability, teleport, fire_id)
        })
    }
    pub fn extinguish_cell(&self, x: i32, z: i32, amount: i32) {
        self.with((), |c| c.fire_extinguish_cell(x, z, amount))
    }
    pub fn quick_burn_cell(&self, x: i32, z: i32) {
        self.with((), |c| c.fire_quick_burn_cell(x, z))
    }
    pub fn ignite_cell_radius(&self, x: i32, z: i32, r0: i32, r1: i32) {
        self.with((), |c| c.fire_ignite_cell_radius(x, z, r0, r1))
    }
    pub fn extinguish(&self, o: u64) {
        self.with((), |c| c.fire_extinguish_occupant(o))
    }
    pub fn burn_down(&self, o: u64) {
        self.with((), |c| c.fire_burn_down_occupant(o))
    }
    pub fn set_cause(&self, cause: u32) {
        self.with((), |c| c.fire_set_cause(cause))
    }
    pub fn cause(&self) -> u32 {
        self.read(0, Core::fire_cause)
    }
    pub fn fire_count(&self) -> u32 {
        self.read(0, Core::fire_count)
    }
    pub fn fire_count_in_rect(&self, r: CellRect) -> u32 {
        self.read(0, |c| c.fire_count_in_rect(r))
    }
    /// Instance slot 4 (`Update`), as the animation tick runs it.
    pub fn fire_update(&self) {
        self.with((), Core::fire_update)
    }
    /// The fire occupant's `SetHosed(+n / −n)` (§5.11, clamped at 0).
    pub fn fire_set_hosed(&self, fire_occupant: u64, delta: i32) -> bool {
        self.with(false, |c| c.fire_set_hosed(fire_occupant, delta))
    }
    pub fn fire_snapshot(&self) -> Option<FireSnapshot> {
        self.read(None, Core::fire_snapshot)
    }

    // ---- riot (§6) ----

    pub fn riot_snapshot(&self, id: u32) -> Option<RiotSnapshot> {
        self.read(None, |c| c.riot_snapshot(id))
    }
    /// The network occupants the riots hold locked, in locking order.
    pub fn riot_locked(&self) -> Vec<u64> {
        self.read(Vec::new(), |c| c.riot_locked())
    }

    // ---- toxic spill (§9) ----

    /// `SpillToxicWaste(x, z, amount)` (the cheat's path).
    pub fn spill_toxic_waste(&self, x: i32, z: i32, amount: i32) -> Option<u32> {
        self.with(None, |c| c.spill_toxic_waste(x, z, amount))
    }
    pub fn toxic_set_hosed(&self, toxic_occupant: u64, delta: i32) -> bool {
        self.with(false, |c| c.toxic_set_hosed(toxic_occupant, delta))
    }
    pub fn toxic_snapshot(&self) -> Option<ToxicSnapshot> {
        self.read(None, Core::toxic_snapshot)
    }

    // ---- short forms (§7, §8, §10, §11) ----

    pub fn quake_snapshot(&self, id: u32) -> Option<QuakeSnapshot> {
        self.read(None, |c| c.quake_snapshot(id))
    }
    pub fn pipe_snapshot(&self, id: u32) -> Option<PipeSnapshot> {
        self.read(None, |c| c.pipe_snapshot(id))
    }
    pub fn volcano_snapshot(&self, id: u32) -> Option<VolcanoSnapshot> {
        self.read(None, |c| c.volcano_snapshot(id))
    }
    /// Asks a volcano to end after its current lava phase (§10 "asked to
    /// end").
    pub fn volcano_request_end(&self, id: u32) -> bool {
        self.with(false, |c| c.volcano_request_end(id))
    }
    /// `StartDisasterEffect(effect, position)` (§11): a new effect instance.
    pub fn start_disaster_effect(&self, effect: u64, x: f32, z: f32) -> Option<u32> {
        self.with(None, |c| c.effect_start(effect, x, z))
    }
}

/// The instance class of a manager type (§1).
pub fn instance_class_of(mtype: u32) -> u32 {
    match mtype {
        kind::VOLCANO => instance_class::VOLCANO,
        kind::FIRE => instance_class::FIRE,
        kind::EARTHQUAKE => instance_class::EARTHQUAKE,
        kind::PIPE_BURST => instance_class::PIPE_BURST,
        kind::RIOT => instance_class::RIOT,
        kind::EFFECT => instance_class::EFFECT,
        kind::TOXIC => instance_class::TOXIC,
        _ => 0,
    }
}
