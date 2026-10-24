// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects manager (effects.md §2, §4–§7): effects created by name,
//! their components chosen by priority, level of detail and select groups,
//! ticked with the dilated animation step, and finished by a sweep that
//! posts their end messages.

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::rng::{FixedSeeds, RzRandom, RzRandomSuper};
use crate::seeding::{self, keys};
use crate::x87::r24;

use super::brush::BrushEffect;
use super::directory::EffectDirectory;
use super::game::GameEffect;
use super::transform::{ParentTransform, Placement, Precision};
use super::{
    component_type, effect_flag, EffectPost, EffectsHost, DEFAULT_MAX_PRIORITY, MAX_ACTIVATION_ROUNDS, SELECT_RANGE,
    SELECT_SEED,
};

/// An effect's ID: from 1, never reused.
pub type EffectId = u32;

/// Where effects finish (§7, EF2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FinishMode {
    /// FreeCity4: the sweep runs when the host calls
    /// [`EffectsManager::finish_hook`] at a deterministic point (after each
    /// animation tick, say); [`EffectsManager::draw_pass`] finishes nothing,
    /// so the result does not depend on the frame rate.
    #[default]
    Hook,
    /// The original (*[compat]*): the sweep runs only in the 3D view's draw
    /// pass ([`EffectsManager::draw_pass`]); a session that never draws never
    /// finishes an effect (quirk 1, EF2). The hook does nothing.
    DrawPass,
}

/// A built component (§5.2).
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Brush(BrushEffect),
    Game(GameEffect),
    /// A nested visual effect (the child, which the parent holds a
    /// reference on).
    Meta(EffectId),
    /// A render or automata component (particles, decals, attractors,
    /// sounds, …): the host's. Its `IsActive` is the host's
    /// [`EffectsHost::render_component_active`].
    Render {
        kind: u8,
        started: bool,
    },
}

#[derive(Clone, Debug)]
struct Effect {
    name: String,
    index: i32,
    flags: u32,
    refs: u32,
    started: bool,
    stopping: bool,
    end_message: (u32, u64),
    position: [f32; 3],
    comps: Vec<Option<Component>>,
}

/// An effect's observable state.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectInfo {
    pub id: EffectId,
    pub name: String,
    /// The description index; −1 = none qualified (§4.1).
    pub index: i32,
    pub flags: u32,
    /// References, the manager's included (1 = only the manager's).
    pub refs: u32,
    pub started: bool,
    pub stopping: bool,
    /// (type, data1); type 0 = none.
    pub end_message: (u32, u64),
    pub position: [f32; 3],
    /// Components built.
    pub components: usize,
    pub active: bool,
}

/// Counters (diagnostics).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectsStats {
    pub created: u64,
    pub recycled: u64,
    pub end_posts: u64,
    pub select_draws: u64,
    pub ticks: u64,
    pub sweeps: u64,
}

/// `cSC4EffectsManager` (§1, §2).
pub struct EffectsManager {
    dir: Option<Rc<EffectDirectory>>,
    max_priority: u32,
    lod: u32,
    /// `+0xC88`: the level of detail before the last zoom change (−1 from
    /// the constructor, EF12).
    lod_prev: i32,
    /// `+0xB8C`: a reload is running (EF12).
    reload: bool,
    /// The `+0xC78` object's slot 14: effect files are queued (EF12).
    reload_pending: bool,
    /// `+0x1AD`: `UpdateActiveVisualEffects` does anything at all (§6.1).
    services_ready: bool,
    select: RzRandomSuper,
    brush_rng: RzRandom,
    total_us: u64,
    total_seconds: f32,
    effects: BTreeMap<EffectId, Effect>,
    active: Vec<EffectId>,
    new_list: Vec<EffectId>,
    next_id: EffectId,
    finish_mode: FinishMode,
    nested_update: bool,
    stats: EffectsStats,
}

/// Recursion limit of the nested updates, starts and stops (a guard; the
/// activation already limits nesting to 10 levels per creation).
const MAX_DEPTH: u32 = 256;

impl EffectsManager {
    /// A manager (§2): max priority 5, the select generator seeded with the
    /// constant `0x1105` (not per process, kept), the brush generator (the
    /// original's static `0x00B22BE8`, seeded from the clock) seeded through
    /// [`keys::BRUSH_EFFECTS`] from `session_seed` (foundations §6.3)
    /// [choice: one per manager]. The level of detail starts at 1 [choice].
    pub fn new(session_seed: u32) -> EffectsManager {
        let no_clock = FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        };
        let key = keys::BRUSH_EFFECTS;
        EffectsManager {
            dir: None,
            max_priority: DEFAULT_MAX_PRIORITY,
            lod: 1,
            lod_prev: -1,
            reload: false,
            reload_pending: false,
            // [choice, §6.1] The original clears `+0x1AD` in the constructor
            // and sets it on `kSC4MessagePostAppServicesInitComplete`; the C
            // ABI has no route for that message yet, so a manager starts
            // ready. A host that models it calls
            // [`EffectsManager::set_services_ready`]`(false)` after `new`.
            services_ready: true,
            select: RzRandomSuper::new(SELECT_SEED, &no_clock),
            brush_rng: RzRandom::new(seeding::seed(session_seed, key), &seeding::clocks(session_seed, key)),
            total_us: 0,
            total_seconds: 0.0,
            effects: BTreeMap::new(),
            active: Vec::new(),
            new_list: Vec::new(),
            next_id: 1,
            finish_mode: FinishMode::Hook,
            nested_update: true,
            stats: EffectsStats::default(),
        }
    }

    pub fn set_directory(&mut self, dir: Rc<EffectDirectory>) {
        self.dir = Some(dir);
    }

    pub fn directory(&self) -> Option<&Rc<EffectDirectory>> {
        self.dir.as_ref()
    }

    /// `+0xB90` (the effects detail option, `DoRenderPropertiesChanged`).
    pub fn set_max_priority(&mut self, p: u32) {
        self.max_priority = p;
    }

    pub fn max_priority(&self) -> u32 {
        self.max_priority
    }

    pub fn level_of_detail(&self) -> u32 {
        self.lod
    }

    pub fn set_finish_mode(&mut self, m: FinishMode) {
        self.finish_mode = m;
    }

    pub fn finish_mode(&self) -> FinishMode {
        self.finish_mode
    }

    /// Whether a meta component updates its child inside its parent's
    /// update (§6 step 3.3, the literal reading: the child, also on the
    /// active list, is then updated twice per tick) [choice, EF9]. Default
    /// true.
    pub fn set_nested_update(&mut self, on: bool) {
        self.nested_update = on;
    }

    pub fn stats(&self) -> EffectsStats {
        self.stats
    }

    /// The brush generator (tests, hosts that replay a session).
    pub fn brush_rng_mut(&mut self) -> &mut RzRandom {
        &mut self.brush_rng
    }

    /// The select generator (§5.3).
    pub fn select_rng_mut(&mut self) -> &mut RzRandomSuper {
        &mut self.select
    }

    /// `+0x1B8`, `+0x1BC`: the total dilated microseconds and seconds.
    pub fn total_time(&self) -> (u64, f32) {
        (self.total_us, self.total_seconds)
    }

    // ---- creation (§4, §5.1) --------------------------------------------------------------

    /// `HasVisualEffect` (slot 5).
    pub fn has_visual_effect(&self, name: &str) -> bool {
        self.dir.as_ref().is_some_and(|d| d.has(name))
    }

    /// `CreateVisualEffect(name)` (slot 7, §4.1): `None` (false) for an
    /// unknown name or without a directory; else the new effect, held by the
    /// manager and the caller, with its components built (and their random
    /// draws made). Its index is −1 when no description's priority is
    /// allowed.
    pub fn create(&mut self, name: &str) -> Option<EffectId> {
        let dir = self.dir.clone()?;
        let index = dir.find(name, self.max_priority)?;
        let id = self.new_effect(&name.to_ascii_lowercase(), index, 2);
        self.activate_new(None);
        Some(id)
    }

    /// Creates the description `index` directly, without the priority filter
    /// (a host that resolved an ID pair itself, slot 6, whose map is not in
    /// the spec) [choice]. `None` for an index out of range.
    pub fn create_by_index(&mut self, index: u32) -> Option<EffectId> {
        let dir = self.dir.clone()?;
        dir.effects.get(index as usize)?;
        let name = dir.name_of(index).unwrap_or("").to_ascii_lowercase();
        let id = self.new_effect(&name, index as i32, 2);
        self.activate_new(None);
        Some(id)
    }

    fn new_effect(&mut self, name: &str, index: i32, refs: u32) -> EffectId {
        let (flags, n) = match (index, &self.dir) {
            (i, Some(d)) if i >= 0 => d
                .effects
                .get(i as usize)
                .map_or((0, 0), |e| (e.flags, e.components.len())),
            _ => (0, 0),
        };
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.effects.insert(
            id,
            Effect {
                name: name.to_owned(),
                index,
                flags,
                refs,
                started: false,
                stopping: false,
                end_message: (0, 0),
                position: [0.0; 3],
                comps: vec![None; n],
            },
        );
        self.new_list.push(id);
        self.stats.created += 1;
        id
    }

    /// `ActivateNewVisualEffects` (§5.1): up to 10 rounds, each moving the
    /// new effects to the active list and building them; a meta component's
    /// child is built in the next round.
    fn activate_new<'h>(&mut self, mut host: Option<&mut (dyn EffectsHost + 'h)>) {
        for _ in 0..MAX_ACTIVATION_ROUNDS {
            if self.new_list.is_empty() {
                break;
            }
            let batch = std::mem::take(&mut self.new_list);
            self.active.extend_from_slice(&batch);
            for id in batch {
                self.build(id, true, host.as_deref_mut());
            }
        }
    }

    /// Whether level `l` (possibly the constructor's −1) is in the
    /// component's `[min, max)`. [choice, EF12] A negative level is in no
    /// range (the spec does not say whether the byte compare is signed).
    fn in_range_at(c: &super::directory::ComponentRecord, l: i32) -> bool {
        l >= 0 && c.in_range(l as u32)
    }

    /// `UpdateVisualEffect(effect, newBuild)` (§5.2, §5.3, EF12), two passes:
    /// 1. an existing component survives only without a reload and with `L`
    ///    in its range; every other one is hard-stopped and released;
    /// 2. a description takes part (draws, adds its weight) only if it just
    ///    entered the range (`L` in, `L_prev` not), or `new_build`, or a
    ///    reload is running; a participating member is created only if `L`
    ///    is in its range. Non-participants leave the group state alone.
    fn build<'h>(&mut self, id: EffectId, new_build: bool, mut host: Option<&mut (dyn EffectsHost + 'h)>) {
        let Some(dir) = self.dir.clone() else { return };
        let Some(e) = self.effects.get(&id) else { return };
        if e.index < 0 {
            return;
        }
        let Some(desc) = dir.effects.get(e.index as usize) else {
            return;
        };
        let lod = self.lod as i32;
        let lod_prev = self.lod_prev;
        let reload = self.reload;
        // Pass 1: existing components. (The survivor refresh `0x0058DC50`
        // when `L != L_prev` is render state the core does not hold.)
        for (i, c) in desc.components.iter().enumerate() {
            let exists = self
                .effects
                .get(&id)
                .is_some_and(|e| e.comps.get(i).is_some_and(Option::is_some));
            if exists && (reload || !Self::in_range_at(c, lod)) {
                if let Some(h) = host.as_deref_mut() {
                    self.stop_component(id, i, 1, h, 0);
                }
                self.release_component(id, i);
            }
        }
        // Pass 2: descriptions. [choice, EF12] Every directory component
        // counts as having a valid type index (`+0x68 >= 0`): the directory
        // keeps no unresolved type.
        let mut prev_group: Option<u16> = None;
        let mut r = 0u32;
        let mut sum = 0u32;
        for (i, c) in desc.components.iter().enumerate() {
            let in_range = Self::in_range_at(c, lod);
            let entering = in_range && !Self::in_range_at(c, lod_prev);
            if !(entering || new_build || reload) {
                continue;
            }
            let g = c.select_group;
            if g > 0 {
                // A new group (different from the last participant's; a
                // group-0 participant resets it) draws `r` and resets the sum.
                if prev_group != Some(g) {
                    r = self.select.uniform_u32(SELECT_RANGE);
                    sum = 0;
                    self.stats.select_draws += 1;
                }
                prev_group = Some(g);
                // Unsigned compare, first running sum >= r; then r := 0x10000
                // (a group summing past 0xFFFF can create a second member).
                sum = sum.wrapping_add(u32::from(c.select_weight));
                if sum < r {
                    continue;
                }
                r = SELECT_RANGE;
            } else {
                prev_group = Some(0);
            }
            let exists = self
                .effects
                .get(&id)
                .is_some_and(|e| e.comps.get(i).is_some_and(Option::is_some));
            if !in_range || exists {
                continue;
            }
            let position = self.effects.get(&id).map_or([0.0; 3], |e| e.position);
            let parent = Placement {
                flags: 1,
                transform: ParentTransform {
                    translation: position,
                    ..ParentTransform::default()
                },
                ..Placement::default()
            };
            let local = Placement::decode(&c.placement);
            let comp = match c.kind {
                component_type::BRUSH => dir.brushes.get(c.index as usize).map(|b| {
                    let mut be = BrushEffect::new(b, &mut self.brush_rng);
                    be.set_placement(&local, &parent, Precision::Single);
                    Component::Brush(be)
                }),
                component_type::GAME => dir.games.get(c.index as usize).map(|g| {
                    let mut ge = GameEffect::new(g);
                    ge.set_placement(&local, &parent, Precision::Single);
                    Component::Game(ge)
                }),
                component_type::META => dir.find(&c.name, self.max_priority).map(|ci| {
                    let child = self.new_effect(&c.name.to_ascii_lowercase(), ci, 2);
                    if let Some(ce) = self.effects.get_mut(&child) {
                        ce.position = position;
                    }
                    Component::Meta(child)
                }),
                kind => Some(Component::Render { kind, started: false }),
            };
            let Some(comp) = comp else { continue };
            let (started, stopping) = match self.effects.get_mut(&id) {
                Some(e) => {
                    e.comps[i] = Some(comp);
                    (e.started, e.stopping)
                }
                None => continue,
            };
            if started && !stopping {
                if let Some(h) = host.as_deref_mut() {
                    self.start_component(id, i, 1, h, 0);
                }
            }
        }
    }

    /// The zoom (`DoZoomChanged` → `UpdateActiveVisualEffects`, §2, §5.2,
    /// §5.3): `L_prev := L`, then `L := max(zoom − offset + 1, 1)`; every
    /// active effect is updated with `newBuild = 0` (only components that
    /// just entered the range take part in a select draw), then the new
    /// list is activated.
    pub fn set_zoom(&mut self, zoom: i32, offset: i32, host: &mut dyn EffectsHost) {
        let l = zoom.saturating_sub(offset).saturating_add(1).max(1);
        self.lod_prev = self.lod as i32;
        self.lod = l as u32;
        self.update_active(host);
    }

    /// `+0xC88`: the level of detail before the last zoom change (−1 before
    /// the first).
    pub fn previous_level_of_detail(&self) -> i32 {
        self.lod_prev
    }

    /// `+0x1AD` (§6.1): set by `kSC4MessagePostAppServicesInitComplete`,
    /// cleared by `Shutdown`; while clear `UpdateActiveVisualEffects` does
    /// nothing.
    pub fn set_services_ready(&mut self, on: bool) {
        self.services_ready = on;
    }

    /// `kSC4MessagePostAppServicesInitComplete` (§6.1): sets `+0x1AD`, then
    /// `UpdateActiveVisualEffects`.
    pub fn post_app_services_init_complete(&mut self, host: &mut dyn EffectsHost) {
        self.services_ready = true;
        self.update_active(host);
    }

    /// `UpdateActiveVisualEffects` `0x00593B90` (§6.1): nothing unless
    /// `+0x1AD` is set; else `UpdateVisualEffect(effect, 0)` on each active
    /// effect in list order, then `ActivateNewVisualEffects`.
    fn update_active(&mut self, host: &mut dyn EffectsHost) {
        if !self.services_ready {
            return;
        }
        for id in self.active.clone() {
            self.build(id, false, Some(&mut *host));
        }
        self.activate_new(Some(host));
    }

    /// Reports that effect files are queued (the `+0xC78` object's slot 14,
    /// §6.1 step 1): the next [`EffectsManager::effects_tick`] reloads.
    pub fn queue_effect_files(&mut self) {
        self.reload_pending = true;
    }

    /// `DoRenderPropertiesChanged` (§6.1): when the effects detail option
    /// (`+0xB90`) changes, it is stored and the active effects are updated
    /// with the reload flag `+0xB8C` = 1.
    pub fn render_properties_changed(&mut self, max_priority: u32, host: &mut dyn EffectsHost) {
        if max_priority == self.max_priority {
            return;
        }
        self.max_priority = max_priority;
        self.reload_active(host);
    }

    /// `+0xB8C := 1`, `UpdateActiveVisualEffects`, `+0xB8C := 0`.
    fn reload_active(&mut self, host: &mut dyn EffectsHost) {
        self.reload = true;
        self.update_active(host);
        self.reload = false;
    }

    /// The internal message `0x697D01ED` (`DoEffectsTick` `0x00593D00`,
    /// §6.1, §12, EF4, EF12): posted once per animation tick in any session
    /// that ticks. It activates and rebuilds **nothing** on an ordinary tick:
    /// only when effect files are queued ([`EffectsManager::queue_effect_files`])
    /// does it parse them (the host's) and reload — `+0xB8C := 1`,
    /// `UpdateActiveVisualEffects` (every component destroyed, every select
    /// group redrawn, the new list activated), `+0xB8C := 0`. The snow-line
    /// step is the host's. Returns how many effects the reload activated
    /// (0 without a pending reload).
    pub fn effects_tick(&mut self, host: &mut dyn EffectsHost) -> usize {
        if !std::mem::take(&mut self.reload_pending) {
            return 0;
        }
        let before = self.active.len();
        self.reload_active(host);
        self.active.len().saturating_sub(before)
    }

    // ---- references ---------------------------------------------------------------------------

    pub fn add_ref(&mut self, id: EffectId) -> bool {
        match self.effects.get_mut(&id) {
            Some(e) => {
                e.refs = e.refs.saturating_add(1);
                true
            }
            None => false,
        }
    }

    /// Releases a caller's reference. The manager's own reference is never
    /// dropped this way (false when only it is left). The effect is
    /// recycled by the next finish sweep once it is done (§7).
    pub fn release(&mut self, id: EffectId) -> bool {
        match self.effects.get_mut(&id) {
            Some(e) if e.refs > 1 => {
                e.refs -= 1;
                true
            }
            _ => false,
        }
    }

    fn release_component(&mut self, id: EffectId, i: usize) {
        let c = self
            .effects
            .get_mut(&id)
            .and_then(|e| e.comps.get_mut(i))
            .and_then(Option::take);
        if let Some(Component::Meta(child)) = c {
            self.release(child);
        }
    }

    // ---- the visual effect's slots (§11.1) ----------------------------------------------------

    /// `Start(mode)` (slot 3): starts every component with `mode`, then
    /// `started := 1`, `stopping := 0`. False for an unknown effect.
    pub fn start(&mut self, id: EffectId, mode: u32, host: &mut dyn EffectsHost) -> bool {
        if !self.effects.contains_key(&id) {
            return false;
        }
        self.start_effect(id, mode, host, 0);
        true
    }

    /// `Stop(mode)` (slot 4): mode 0 on a hard-stop effect becomes 1; stops
    /// every component; mode 0 is a soft stop (`stopping := 1`), any other
    /// clears `started` and `stopping`.
    pub fn stop(&mut self, id: EffectId, mode: u32, host: &mut dyn EffectsHost) -> bool {
        if !self.effects.contains_key(&id) {
            return false;
        }
        self.stop_effect(id, mode, host, 0);
        true
    }

    fn start_effect(&mut self, id: EffectId, mode: u32, host: &mut dyn EffectsHost, depth: u32) {
        if depth > MAX_DEPTH {
            return;
        }
        let n = self.effects.get(&id).map_or(0, |e| e.comps.len());
        for i in 0..n {
            self.start_component(id, i, mode, host, depth);
        }
        if let Some(e) = self.effects.get_mut(&id) {
            e.started = true;
            e.stopping = false;
        }
    }

    fn stop_effect(&mut self, id: EffectId, mode: u32, host: &mut dyn EffectsHost, depth: u32) {
        if depth > MAX_DEPTH {
            return;
        }
        let Some(e) = self.effects.get(&id) else { return };
        let mode = if mode == 0 && e.flags & effect_flag::HARD_STOP != 0 {
            1
        } else {
            mode
        };
        for i in 0..e.comps.len() {
            self.stop_component(id, i, mode, host, depth);
        }
        if let Some(e) = self.effects.get_mut(&id) {
            if mode != 0 {
                e.started = false;
                e.stopping = false;
            } else {
                e.stopping = true;
            }
        }
    }

    fn start_component(&mut self, id: EffectId, i: usize, mode: u32, host: &mut dyn EffectsHost, depth: u32) {
        let child = match self
            .effects
            .get_mut(&id)
            .and_then(|e| e.comps.get_mut(i))
            .and_then(Option::as_mut)
        {
            Some(Component::Brush(b)) => {
                b.start();
                None
            }
            Some(Component::Game(g)) => {
                g.start(host);
                None
            }
            Some(Component::Meta(c)) => Some(*c),
            Some(Component::Render { started, .. }) => {
                *started = true;
                None
            }
            None => None,
        };
        if let Some(c) = child {
            self.start_effect(c, mode, host, depth + 1);
        }
    }

    fn stop_component(&mut self, id: EffectId, i: usize, mode: u32, host: &mut dyn EffectsHost, depth: u32) {
        let child = match self
            .effects
            .get_mut(&id)
            .and_then(|e| e.comps.get_mut(i))
            .and_then(Option::as_mut)
        {
            Some(Component::Brush(b)) => {
                b.stop(host);
                None
            }
            Some(Component::Game(g)) => {
                g.stop(host);
                None
            }
            Some(Component::Meta(c)) => Some(*c),
            Some(Component::Render { started, .. }) => {
                *started = false;
                None
            }
            None => None,
        };
        if let Some(c) = child {
            self.stop_effect(c, mode, host, depth + 1);
        }
    }

    /// `IsStarted` (slot 5).
    pub fn is_started(&self, id: EffectId) -> bool {
        self.effects.get(&id).is_some_and(|e| e.started)
    }

    /// `IsStopping` (slot 6).
    pub fn is_stopping(&self, id: EffectId) -> bool {
        self.effects.get(&id).is_some_and(|e| e.stopping)
    }

    /// Translation-only parent setter; brush/game local placements are decoded
    /// from their resource records (§3.1). Meta forwarding remains translation-only.
    pub fn set_position(&mut self, id: EffectId, pos: [f32; 3]) -> bool {
        self.set_position_at(id, pos, 0)
    }

    fn set_position_at(&mut self, id: EffectId, pos: [f32; 3], depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let Some(e) = self.effects.get_mut(&id) else {
            return false;
        };
        e.position = pos;
        let parent = Placement {
            flags: 1,
            transform: ParentTransform {
                translation: pos,
                ..ParentTransform::default()
            },
            ..Placement::default()
        };
        let description = self.dir.as_ref().and_then(|d| d.effects.get(e.index as usize));
        let mut children = Vec::new();
        for (index, c) in e.comps.iter_mut().enumerate() {
            let Some(c) = c else {
                continue;
            };
            let local = description
                .and_then(|d| d.components.get(index))
                .map_or_else(Placement::default, |c| Placement::decode(&c.placement));
            match c {
                Component::Brush(b) => b.set_placement(&local, &parent, Precision::Single),
                Component::Game(g) => g.set_placement(&local, &parent, Precision::Single),
                Component::Meta(child) => children.push(*child),
                Component::Render { .. } => {}
            }
        }
        for c in children {
            self.set_position_at(c, pos, depth + 1);
        }
        true
    }

    /// `SetEndMessage(type, data1)` (slot 10).
    pub fn set_end_message(&mut self, id: EffectId, msg_type: u32, data1: u64) -> bool {
        match self.effects.get_mut(&id) {
            Some(e) => {
                e.end_message = (msg_type, data1);
                true
            }
            None => false,
        }
    }

    /// Slot 13: forwards a parameter to the components (their slot 6),
    /// unless flag bit 4. A game component takes index 13 as its message
    /// data; a meta component passes it to its child's slot 13.
    pub fn forward_parameter(&mut self, id: EffectId, index: u32, value: u64) -> bool {
        self.forward_at(id, index, value, 0)
    }

    fn forward_at(&mut self, id: EffectId, index: u32, value: u64, depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let Some(e) = self.effects.get_mut(&id) else {
            return false;
        };
        if e.flags & effect_flag::NO_FORWARD != 0 {
            return true;
        }
        let mut children = Vec::new();
        for c in e.comps.iter_mut().flatten() {
            match c {
                Component::Game(g) => g.set_parameter(index, value),
                Component::Meta(child) => children.push(*child),
                _ => {}
            }
        }
        for c in children {
            self.forward_at(c, index, value, depth + 1);
        }
        true
    }

    // ---- the tick (§6) ------------------------------------------------------------------------

    /// `DoAnimationTick` (§6): `dt' := trunc(r24(µs × dilation))` (µs
    /// without a simulator), added to the total; every started effect on the
    /// active list, in list order, is updated with `dt = r24(dt' × 1e-6)`
    /// seconds. Runs while the game is paused, but the shipped dilation is
    /// then 0.0 ([`super::dilation`]), so every update runs with `dt = 0`
    /// (§6 quirk, quirk 3, re-answers34).
    pub fn tick(&mut self, us: u32, host: &mut dyn EffectsHost) {
        let dtp: u64 = super::dilation::dilated_us(us, host.simulator());
        self.total_us = self.total_us.wrapping_add(dtp);
        self.total_seconds = r24(self.total_us as f64 * 1e-6) as f32;
        let dt = r24(dtp as f64 * 1e-6) as f32;
        self.stats.ticks += 1;
        for id in self.active.clone() {
            if self.is_started(id) {
                self.update_effect(id, dt, host, 0);
            }
        }
    }

    /// The effect's update (`0x005BEA10`): each component, in order, whose
    /// `IsActive` holds.
    fn update_effect(&mut self, id: EffectId, dt: f32, host: &mut dyn EffectsHost, depth: u32) {
        if depth > MAX_DEPTH {
            return;
        }
        let n = self.effects.get(&id).map_or(0, |e| e.comps.len());
        for i in 0..n {
            let child = match self
                .effects
                .get_mut(&id)
                .and_then(|e| e.comps.get_mut(i))
                .and_then(Option::as_mut)
            {
                Some(Component::Brush(b)) if b.is_active() => {
                    b.update(dt, host);
                    None
                }
                Some(Component::Game(g)) if g.is_active() => {
                    g.update(dt, host);
                    None
                }
                Some(Component::Meta(c)) => Some(*c),
                _ => None,
            };
            if let Some(c) = child {
                if self.nested_update && self.is_started(c) {
                    self.update_effect(c, dt, host, depth + 1);
                }
            }
        }
    }

    // ---- finishing (§7) -----------------------------------------------------------------------

    /// FreeCity4's deterministic finish point: runs the sweep in
    /// [`FinishMode::Hook`] (true), nothing in the compat mode (false).
    pub fn finish_hook(&mut self, host: &mut dyn EffectsHost) -> bool {
        if self.finish_mode != FinishMode::Hook {
            return false;
        }
        self.sweep(host);
        true
    }

    /// The 3D view's draw pass (`Update(SC4DrawContext)`): runs the sweep in
    /// [`FinishMode::DrawPass`] (true), nothing in the hook mode (false).
    pub fn draw_pass(&mut self, host: &mut dyn EffectsHost) -> bool {
        if self.finish_mode != FinishMode::DrawPass {
            return false;
        }
        self.sweep(host);
        true
    }

    /// `DeactivateUnusedVisualEffects` (§7): tests each active effect in list
    /// order; finishes those that are done (`Stop(1)`, then the end message);
    /// recycles the done ones only the manager holds.
    fn sweep(&mut self, host: &mut dyn EffectsHost) {
        self.stats.sweeps += 1;
        for id in self.active.clone() {
            let Some(e) = self.effects.get(&id) else { continue };
            let done = if !e.started || e.index < 0 {
                true
            } else if e.flags & effect_flag::NO_AUTO_STOP != 0
                || self.held(id, host)
                || self
                    .effects
                    .get(&id)
                    .is_some_and(|e| e.flags & effect_flag::FINISH_AFTER_SOFT_STOP != 0 && !e.stopping)
            {
                false
            } else {
                self.stop_effect(id, 1, host, 0);
                if let Some(&(t, d1)) = self.effects.get(&id).map(|e| &e.end_message) {
                    if t != 0 {
                        host.post(EffectPost {
                            msg_type: t,
                            data1: d1,
                            data2: 0,
                            data3: 0,
                        });
                        self.stats.end_posts += 1;
                    }
                }
                true
            };
            if done && self.effects.get(&id).is_some_and(|e| e.refs <= 1) {
                self.recycle(id);
            }
        }
        // The sweep ends in `ActivateNewVisualEffects` (`0x00592A01`, §6.1,
        // EF12): the new list left over from a creation is activated here,
        // not by the effects tick.
        self.activate_new(Some(host));
    }

    /// Whether a component whose flag bit 0 is clear still reports
    /// `IsActive` (tested in component order, §7 rule 2).
    fn held(&self, id: EffectId, host: &mut dyn EffectsHost) -> bool {
        let Some(e) = self.effects.get(&id) else { return false };
        let Some(desc) = self
            .dir
            .as_ref()
            .and_then(|d| usize::try_from(e.index).ok().and_then(|i| d.effects.get(i)))
        else {
            return false;
        };
        for (i, c) in e.comps.iter().enumerate() {
            let Some(c) = c else { continue };
            if desc.components.get(i).map_or(0, |r| r.flags) & 1 != 0 {
                continue;
            }
            let active = match c {
                Component::Brush(b) => b.is_active(),
                Component::Game(g) => g.is_active(),
                Component::Meta(child) => self.is_started(*child),
                Component::Render { kind, .. } => host.render_component_active(id, i as u32, *kind),
            };
            if active {
                return true;
            }
        }
        false
    }

    fn recycle(&mut self, id: EffectId) {
        self.active.retain(|&a| a != id);
        if let Some(e) = self.effects.remove(&id) {
            for c in e.comps.into_iter().flatten() {
                if let Component::Meta(child) = c {
                    self.release(child);
                }
            }
            self.stats.recycled += 1;
        }
    }

    /// `Shutdown` (slot 4) [choice: not in the spec]: every started effect
    /// is hard-stopped (no end message), then all are dropped.
    pub fn shutdown(&mut self, host: &mut dyn EffectsHost) {
        for id in self.active.clone() {
            if self.is_started(id) {
                self.stop_effect(id, 1, host, 0);
            }
        }
        self.effects.clear();
        self.active.clear();
        self.new_list.clear();
        // [choice, §6.1] `Shutdown` also clears `+0x1AD`; it is kept here
        // because the C ABI cannot set it again (see `services_ready`).
    }

    // ---- introspection --------------------------------------------------------------------------

    pub fn effect(&self, id: EffectId) -> Option<EffectInfo> {
        let e = self.effects.get(&id)?;
        Some(EffectInfo {
            id,
            name: e.name.clone(),
            index: e.index,
            flags: e.flags,
            refs: e.refs,
            started: e.started,
            stopping: e.stopping,
            end_message: e.end_message,
            position: e.position,
            components: e.comps.iter().flatten().count(),
            active: self.active.contains(&id),
        })
    }

    /// The component slots of an effect, one per description component
    /// (`None` = not built).
    pub fn components(&self, id: EffectId) -> Option<&[Option<Component>]> {
        self.effects.get(&id).map(|e| e.comps.as_slice())
    }

    /// The `n`-th built component of `kind` ([`component_type`]) of an
    /// effect, in component order.
    pub fn component_of_kind(&self, id: EffectId, kind: u8, n: usize) -> Option<&Component> {
        self.components(id)?
            .iter()
            .flatten()
            .filter(|c| match c {
                Component::Brush(_) => kind == component_type::BRUSH,
                Component::Game(_) => kind == component_type::GAME,
                Component::Meta(_) => kind == component_type::META,
                Component::Render { kind: k, .. } => *k == kind,
            })
            .nth(n)
    }

    /// The active list, in order.
    pub fn active_list(&self) -> &[EffectId] {
        &self.active
    }

    /// Effects alive (on either list).
    pub fn effect_count(&self) -> usize {
        self.effects.len()
    }
}
