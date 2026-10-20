// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The calls other subsystems make into the automata layer, as host-facing
//! operations (§8, §9): the riot leader's start, stage rise and end (§8.1),
//! the fire occupant's `SetTarget` and `Shutdown` (§8.2), and the U-Drive-It
//! pause and resume of the four managers (§9). Each operation makes its
//! calls through a host trait, in the order the spec gives; the objects are
//! host tokens.

use super::automaton::{state_type, SetStateArgs};
use super::codec::Vec3;
use super::manager::{AutomataManager, CreateArgs};
use super::{OCCUPANT_TYPE_BUILDING, RIOT_AUTOMATA_GROUP};
use crate::stream::ResKey;

/// `CreateAutomaton`'s automata flags for a riot leader (§7.2 caller table:
/// `(0x4107, cell position, 2, zero key, null, 0xFF, null)`).
pub const RIOT_CREATE_FLAGS: u32 = 2;
/// `CreateAutomaton`'s priority for a riot leader: `0xFF`, which forces the
/// creation (§7.2).
pub const RIOT_CREATE_PRIORITY: u8 = 0xFF;
/// `SetState(8, 0, −1.0, 1, 0, 0, 0)`: the RiotLeader state (§8.1 step 5).
/// The `1` in the fourth argument is **`final`** (§6.2), so the later
/// `SetState(1, …)` of [`RIOT_END_STATE_ARGS`] is a no-op (§8.1 corrects).
pub const RIOT_LEADER_STATE_ARGS: SetStateArgs = SetStateArgs {
    controller: 0,
    timeout_ms: -1.0,
    final_flag: 1,
    anim_id: 0,
    sfx_id: 0,
    template: 0,
};
/// `SetState(1, 0, −1.0, 0, 0, 0, 0)` at the end (§8.1 end step 1). The call
/// is **made**; against a `final` leader it returns 0 and changes nothing
/// (§8.1 corrects), and steps 2–4 of the end run all the same.
pub const RIOT_END_STATE_ARGS: SetStateArgs = SetStateArgs {
    controller: 0,
    timeout_ms: -1.0,
    final_flag: 0,
    anim_id: 0,
    sfx_id: 0,
    template: 0,
};
/// The number of riot stages: a new maximum stage below it attaches a
/// controller (disasters §6.5 step 4).
pub const RIOT_STAGES: i32 = 6;
/// The fire occupant's occupant group (§8.2).
pub const FIRE_OCCUPANT_GROUP: &str = "fire_occupant";

/// The template name of a riot stage: `riot_leader_%d` (§8.1).
pub fn riot_leader_template(stage: i32) -> String {
    format!("riot_leader_{stage}")
}

/// The game objects the riot leader hooks call (§8.1).
pub trait RiotHost {
    /// Step 1: the nearest road tile's position; `None` = none.
    fn nearest_road_tile(&mut self, x: i32, z: i32) -> Option<Vec3>;
    /// Step 2: the leader count has reached its cap (the disaster layer's
    /// police-coverage and count check, disasters §6.4 step 2).
    fn leader_cap_reached(&mut self, leaders: usize) -> bool;
    /// Pedestrian manager (city slot 44) slot 15 `CreateAutomaton`; 0 = null.
    fn create_automaton(&mut self, args: CreateArgs) -> u64;
    fn add_ref(&mut self, automaton: u64);
    /// Automaton slot 51 `SetState(type, …)`.
    fn set_state(&mut self, automaton: u64, state_type: u32, args: SetStateArgs);
    /// Automaton slot 9 `SetVisible`.
    fn set_visible(&mut self, automaton: u64, visible: bool);
    /// Automaton slot 5 `AsOccupant`.
    fn as_occupant(&mut self, automaton: u64) -> u64;
    /// The disaster layer (city slot 72): register the leader's location.
    fn register_location(&mut self, automaton: u64, position: Vec3);
    /// Controller manager slot 5 `AddControllerFromTemplateName`; false =
    /// null.
    fn add_controller_from_template_name(&mut self, occupant: u64, name: &str, extra: u32) -> bool;
    /// Controller manager slot 14 `RemoveControllers`.
    fn remove_controllers(&mut self, occupant: u64);
    /// The disaster layer slot 15: unregister the location by occupant.
    fn unregister_location(&mut self, occupant: u64);
    /// Automaton slot 2 `Release`.
    fn release(&mut self, automaton: u64);
}

/// One riot leader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiotLeader {
    pub automaton: u64,
    pub position: Vec3,
    pub stage: i32,
    /// The highest stage reached.
    pub max_stage: i32,
}

/// A riot's leaders and the automata calls they make (§8.1).
#[derive(Clone, Debug, Default)]
pub struct RiotLeaders {
    leaders: Vec<RiotLeader>,
}

impl RiotLeaders {
    pub fn new() -> RiotLeaders {
        RiotLeaders::default()
    }

    pub fn leaders(&self) -> &[RiotLeader] {
        &self.leaders
    }

    /// `AddRiotLeader` (§8.1 start), in order: the road tile (none → false),
    /// the cap (reached → false), `CreateAutomaton(0x4107, …)` (null →
    /// false), `AddRef`, `SetState(8, …)`, `SetVisible(true)`, the location.
    /// Returns the new leader's index.
    pub fn add(&mut self, host: &mut dyn RiotHost, x: i32, z: i32) -> Option<usize> {
        let position = host.nearest_road_tile(x, z)?;
        if host.leader_cap_reached(self.leaders.len()) {
            return None;
        }
        let a = host.create_automaton(CreateArgs {
            group: RIOT_AUTOMATA_GROUP,
            position,
            flags: RIOT_CREATE_FLAGS,
            model_key: ResKey::default(),
            callback: 0,
            priority: RIOT_CREATE_PRIORITY,
            orientation: None,
        });
        if a == 0 {
            return None;
        }
        host.add_ref(a);
        host.set_state(a, state_type::RIOT_LEADER, RIOT_LEADER_STATE_ARGS);
        host.set_visible(a, true);
        host.register_location(a, position);
        self.leaders.push(RiotLeader {
            automaton: a,
            position,
            stage: 0,
            max_stage: 0,
        });
        Some(self.leaders.len() - 1)
    }

    /// The stage update (`Update`, §8.1 stage rise): when `stage` is a new
    /// maximum below [`RIOT_STAGES`], `AddControllerFromTemplateName(leader
    /// as occupant, "riot_leader_<stage>", 0)`. Returns whether the call was
    /// made.
    pub fn set_stage(&mut self, host: &mut dyn RiotHost, index: usize, stage: i32) -> bool {
        let Some(l) = self.leaders.get_mut(index) else {
            return false;
        };
        l.stage = stage;
        if stage <= l.max_stage || stage >= RIOT_STAGES {
            return false;
        }
        l.max_stage = stage;
        let a = l.automaton;
        let occ = host.as_occupant(a);
        host.add_controller_from_template_name(occ, &riot_leader_template(stage), 0);
        true
    }

    /// The end (§8.1), in order: `SetState(1, …)`, `RemoveControllers(leader
    /// as occupant)`, unregister the location, release, erase.
    pub fn end(&mut self, host: &mut dyn RiotHost, index: usize) -> bool {
        if index >= self.leaders.len() {
            return false;
        }
        let l = self.leaders.remove(index);
        host.set_state(l.automaton, state_type::RIOT_END, RIOT_END_STATE_ARGS);
        let occ = host.as_occupant(l.automaton);
        host.remove_controllers(occ);
        host.unregister_location(occ);
        host.release(l.automaton);
        true
    }
}

/// The game objects the fire occupant's hooks call (§8.2).
pub trait FireHost {
    /// The target occupant's `GetType`.
    fn occupant_type(&mut self, occupant: u64) -> u32;
    /// Controller manager slot 10 `AddControllersFromOccupantGroup(name,
    /// occupant, extra)`.
    fn add_controllers_from_occupant_group_name(&mut self, name: &str, occupant: u64, extra: u32) -> bool;
    /// Controller manager slot 14 `RemoveControllers(occupant)`.
    fn remove_controllers(&mut self, occupant: u64);
}

/// The fire occupant's automata hooks (§8.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireHook {
    target: u64,
}

impl FireHook {
    pub fn new() -> FireHook {
        FireHook::default()
    }

    pub fn target(&self) -> u64 {
        self.target
    }

    /// `SetTarget`: the target is kept; a building (`0x278128A0`) gets the
    /// `"fire_occupant"` controllers (slot 10). Returns whether the call was
    /// made.
    pub fn set_target(&mut self, host: &mut dyn FireHost, target: u64) -> bool {
        self.target = target;
        if target != 0 && host.occupant_type(target) == OCCUPANT_TYPE_BUILDING {
            host.add_controllers_from_occupant_group_name(FIRE_OCCUPANT_GROUP, target, 0);
            return true;
        }
        false
    }

    /// `Shutdown`: `RemoveControllers(target)` (slot 14), then the target is
    /// dropped. Nothing is called without a target.
    pub fn shutdown(&mut self, host: &mut dyn FireHost) -> bool {
        let t = std::mem::take(&mut self.target);
        if t == 0 {
            return false;
        }
        host.remove_controllers(t);
        true
    }
}

/// U-Drive-It (§9): when a mission activates, the city situation manager
/// pauses all four managers (slot 9).
pub fn pause_all(managers: &mut [&mut AutomataManager]) {
    for m in managers.iter_mut() {
        m.pause();
    }
}

/// U-Drive-It (§9): the start countdown's end resumes them (slot 10).
pub fn resume_all(managers: &mut [&mut AutomataManager]) {
    for m in managers.iter_mut() {
        m.resume();
    }
}
