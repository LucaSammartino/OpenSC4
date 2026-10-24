// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects manager and the effects the simulation depends on
//! (`docs/specs/effects.md`): the effect directory ([`directory`]), effect
//! creation with the priority filter, levels of detail and select groups,
//! the dilated animation tick, finishing and the end message
//! ([`EffectsManager`]), the brush effect ([`brush`]) and the game effect
//! ([`game`]), and the shipped records and disaster mapping as data
//! ([`shipped`]). After re-answers34: the shipped animation time dilation
//! table ([`dilation`], §6, EF5), the particle component's stop protocol
//! and emitter self-stop ([`particles`], §7.1, EF1) and the per-tick
//! activation ([`EffectsManager::effects_tick`], §6.1, EF4); the open
//! choices there are EF11-EF13.
//!
//! Out of scope (the host's): drawing, particles, decals, sounds, the
//! effect maps and the automata attractors. Their components exist in the
//! core as [`Component::Render`] placeholders, whose `IsActive` the host
//! answers ([`EffectsHost::render_component_active`], false by default).
//!
//! **Finishing** (§7, EF2). The original finishes effects, and posts their
//! end messages, only in the 3D view's draw pass, so a session that never
//! draws never ends a volcano. FreeCity4 finishes them at a deterministic
//! point the host calls ([`EffectsManager::finish_hook`], for example after
//! each animation tick; [`FinishMode::Hook`], the default); the original's
//! behaviour is kept as [`FinishMode::DrawPass`].
//!
//! Spec gaps decided here (numbered questions, EF6 onwards): the render
//! sections are located, not read (EF6); a brush's second `Stop` ends the long terrain
//! operation again (EF8); a meta component updates its child although the
//! child is also on the active list (EF9).

pub mod brush;
pub mod collection;
pub mod dilation;
pub mod directory;
pub mod game;
mod manager;
pub mod particle_owners;
pub mod particles;
pub mod shipped;
pub mod transform;

#[cfg(test)]
mod tests;

pub use brush::{BrushEffect, BRUSH_DATA4};
pub use directory::{
    game_flag, AttractorRecord, BrushRecord, ComponentRecord, EffectDescription, EffectDirectory, GameRecord,
    ParamBinding, ParseError, SectionOffsets,
};
pub use game::{DemolishFilter, EffectRect, GameEffect, OccupantFacts, BURN_CAUSE};
pub use manager::{Component, EffectId, EffectInfo, EffectsManager, EffectsStats, FinishMode};

use crate::terrain::BrushParams;

/// `cSC4EffectsManager` (§1).
pub const CLSID_EFFECTS_MANAGER: u32 = 0x4982_2F75;
/// `cISC4EffectsManager`.
pub const IID_EFFECTS_MANAGER: u32 = 0xC97C_F5CD;

/// The effect directory resource (§0, §3).
pub const DIRECTORY_TYPE: u32 = 0xEA51_18B0;
pub const DIRECTORY_GROUP: u32 = 0xEA51_18B1;
pub const DIRECTORY_INSTANCE: u32 = 1;

/// The select generator's constant seed (§2).
pub const SELECT_SEED: u32 = 0x1105;
/// `RandomUint32Uniform(0x10000)` per select group (§5.3).
pub const SELECT_RANGE: u32 = 0x1_0000;
/// The max priority at construction (§2).
pub const DEFAULT_MAX_PRIORITY: u32 = 5;
/// Activation rounds per creation (§5.1).
pub const MAX_ACTIVATION_ROUNDS: usize = 10;
/// World units per cell (the game record's cell, §9.2).
pub const CELL_SIZE: f32 = 16.0;

/// Messages (§12).
pub mod msg {
    /// `kSC4MessageAnimationTick` (data2 = µs).
    pub const ANIMATION_TICK: u32 = 0x86AD_10EE;
    /// `kSC4MessageEffectDone`: an effect disaster's end message.
    pub const EFFECT_DONE: u32 = 0x2A7B_582D;
    /// `kSC4MessageEffectLocation` (game records 19, 23, 36).
    pub const EFFECT_LOCATION: u32 = 0x2A7B_582E;
    /// `kSC4MessageTornadoLocation` (game record 26).
    pub const TORNADO_LOCATION: u32 = 0x0A36_F1E9;
    /// `volcano_control`'s end message to the volcano.
    pub const VOLCANO_EFFECT_END: u32 = 0xCA49_3C1D;
}

/// Component types (§3, `+0x0C`).
pub mod component_type {
    pub const PARTICLES: u8 = 0;
    pub const DECAL: u8 = 1;
    pub const META: u8 = 2;
    pub const BRUSH: u8 = 3;
    pub const ATTRACTOR: u8 = 4;
    pub const GAME: u8 = 5;
    pub const SEQUENCE: u8 = 6;
    pub const SOUND: u8 = 7;
    pub const CAMERA: u8 = 8;
    pub const MODEL: u8 = 0x10;
}

/// Visual effect flags (§11.1).
pub mod effect_flag {
    /// Bit 1: no auto stop.
    pub const NO_AUTO_STOP: u32 = 2;
    /// Bit 2: a soft stop becomes a hard one.
    pub const HARD_STOP: u32 = 4;
    /// Bit 4: slot 13 does not forward.
    pub const NO_FORWARD: u32 = 0x10;
    /// Bit 5: takes the pick ray (render).
    pub const PICK_RAY: u32 = 0x20;
    /// Bit 7: finishes only after a soft stop.
    pub const FINISH_AFTER_SOFT_STOP: u32 = 0x80;
}

/// A queued message the effects post (§12): an end message (data2 = data3
/// = 0) or a game record's (data2, data3 = the cell).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectPost {
    pub msg_type: u32,
    /// A host token (the disaster instance) or the record's data.
    pub data1: u64,
    pub data2: i32,
    pub data3: i32,
}

/// What the effects need from the game. Every call happens inside a
/// manager call (the tick, a start or stop, the sweep, a zoom change);
/// posts are queued by the host (§12: their receivers run after the tick).
pub trait EffectsHost {
    /// The simulator's animation time dilation (slot 32, simulator.md §7),
    /// or `None` without a simulator (the tick then uses µs as is, and game
    /// effects never start).
    fn simulator(&mut self) -> Option<f32>;
    /// Whether the simulator exists (the game effect's `Start`).
    fn simulator_exists(&mut self) -> bool {
        self.simulator().is_some()
    }
    /// Simulator slot 9 `Pause` / slot 12 resume.
    fn sim_pause(&mut self) {}
    fn sim_resume(&mut self) {}
    /// Simulator slot 10 `HiddenPause` / slot 13 hidden resume.
    fn sim_hidden_pause(&mut self) {}
    fn sim_hidden_resume(&mut self) {}
    /// The 24-hour clock's slot 26 with 1 (`true`) or 0.
    fn clock_pause(&mut self, _on: bool) {}
    /// The city's cells (city slots 93, 94).
    fn city_cells(&mut self) -> (i32, i32);
    /// City slot 98 `LocationIsInBounds` (world units).
    fn location_in_bounds(&mut self, x: f32, z: f32) -> bool;
    /// Terrain slot 57 `ApplyBrush`.
    fn apply_brush(&mut self, brush_id: u32, params: &mut BrushParams) -> bool;
    /// Terrain slots 87 and 88.
    fn begin_long_terrain_operation(&mut self) {}
    fn end_long_terrain_operation(&mut self) {}
    /// Fire manager slot 8 `IgniteCellRadius(x, z, r0, r1)`.
    fn ignite_cell_radius(&mut self, _x: i32, _z: i32, _r0: i32, _r1: i32) {}
    /// Fire manager slot 9 `SetCause`.
    fn set_fire_cause(&mut self, _cause: u32) {}
    /// The toxic spill manager's `AttemptDisasterStart(x, z)` (slot 6).
    fn toxic_attempt_start(&mut self, _x: i32, _z: i32) {}
    /// Demolition slot 6 `DemolishRegion(true, rect, privilege 0, flags,
    /// dezone, filter, …)`; the host asks [`DemolishFilter::skips`] per
    /// occupant.
    fn demolish_region(&mut self, _rect: EffectRect, _flags: u32, _dezone: bool, _filter: &DemolishFilter) {}
    /// Fire manager slot 6 `ExtinguishCell(x, z, amount)`.
    fn extinguish_cell(&mut self, _x: i32, _z: i32, _amount: i32) {}
    /// A queued post (`MessagePost`).
    fn post(&mut self, msg: EffectPost);
    /// Render service boundary for game-effect maps (§9.3). Shape fields stay
    /// in the description; rasterization and rectangle geometry are foreign.
    /// Returns an opaque host map identity, retained by the component.
    fn apply_effect_map(&mut self, _record: &GameRecord, _cell: (i32, i32)) -> u64 {
        0
    }
    /// Remove the map with the supplied opaque host identity.
    fn remove_effect_map(&mut self, _map: u64) {}
    /// `IsActive` of a render component (the finish test, §7), by effect,
    /// component index and type. False by default: render components do
    /// not hold an effect [choice, EF1].
    fn render_component_active(&mut self, _effect: EffectId, _component: u32, _kind: u8) -> bool {
        false
    }
}
