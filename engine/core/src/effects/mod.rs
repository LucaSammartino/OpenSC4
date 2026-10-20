// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects the simulation depends on (`docs/specs/effects.md`), short
//! form. Section references (`§n`) point at that spec.
//!
//! The original's `cSC4EffectsManager` is largely a render object: effects are
//! not saved (§13), the tick only advances components the draw pass will show
//! (§6), and the damage a brush or game component does is applied inside that
//! tick. What the *simulation* needs from it is a small set of rules, and they
//! are the ones the re-integrate1 measurements corrected:
//!
//! - **§5.2** the level-of-detail ranges are not uniform;
//! - **§7** the finish test, and the fact that the bit it tests is a *runtime*
//!   bit, not the description bit;
//! - **§10.1** when `volcano_control` really finishes;
//! - **§10.2** which slaves can post `data1 = the instance`.
//!
//! Those rules are here as pure predicates over the record shapes that
//! `fc4_formats::effects` parses. The rendering, the particles, the decals and
//! the brushes themselves are not modelled.

/// The effect flags of §7, and the runtime component flags of `comp + 0x40`.
pub mod flags {
    /// Effect flag bit 1 ("no auto stop"): the effect never finishes on its
    /// own (§7 rule 2).
    pub const NO_AUTO_STOP: u32 = 2;
    /// Effect flag bit 7: the effect is not finished while it is soft-stopping
    /// (§7 rule 4).
    pub const STOPPING_ONLY: u32 = 0x80;

    /// Runtime component bit 1 (value 2), set on table components of type
    /// 3-15 (§7).
    pub const TABLE: u8 = 2;
    /// Runtime component bit 2 (value 4): the copy of the description's
    /// `flags & 1` ignore bit, and **the bit the finish test tests** (§7).
    pub const IGNORED: u8 = 4;
}

/// The runtime flags a component is built with (§7, conformance test 12).
///
/// `0x00591730` copies `desc+0x10 & 1` into bit 2 (value 4) of `comp+0x40`, and
/// a table component (types 3-15) additionally gets bit 1 (value 2). An
/// implementer must test the **runtime** bit, not the description bit.
pub fn runtime_flags(desc_flags: u32, component_type: u8) -> u8 {
    let mut f = 0;
    if desc_flags & 1 != 0 {
        f |= flags::IGNORED;
    }
    if (3..=15).contains(&component_type) {
        f |= flags::TABLE;
    }
    f
}

/// One runtime component of an effect, as the finish test sees it (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeComponent {
    /// `comp + 0x40` (see [`runtime_flags`]).
    pub flags: u8,
    /// The component's `IsActive` (component vtable slot 2).
    pub is_active: bool,
}

impl RuntimeComponent {
    /// A runtime component built from its description's `flags` and type.
    pub fn from_description(desc_flags: u32, component_type: u8, is_active: bool) -> RuntimeComponent {
        RuntimeComponent {
            flags: runtime_flags(desc_flags, component_type),
            is_active,
        }
    }

    /// Whether the finish test skips this component (§7): `flags & 4`.
    pub fn ignored(&self) -> bool {
        self.flags & flags::IGNORED != 0
    }

    /// Whether this component blocks the finish (§7 rule 3).
    pub fn blocks_finish(&self) -> bool {
        !self.ignored() && self.is_active
    }
}

/// A visual effect as the finish test sees it (§7). Offsets are the runtime
/// object's: `+0x20` index (−1 = none), `+0x24` `started`, `+0x25` `stopping`,
/// `+0x28` flags, `+0x2C` / `+0x30` end message type and data1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectRuntime {
    /// `+0x20`: the description index, −1 when the name resolved to nothing.
    pub index: i32,
    /// `+0x24`.
    pub started: bool,
    /// `+0x25`.
    pub stopping: bool,
    /// `+0x28`.
    pub flags: u32,
    /// `+0x2C`: 0 means "post nothing".
    pub end_message_type: u32,
    /// `+0x30`.
    pub end_message_data1: u64,
}

/// What one draw-pass sweep does to an effect (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepResult {
    /// Rule 1: not started, or index −1. It counts as done at once; it is not
    /// stopped and posts nothing.
    DoneWithoutMessage,
    /// Rule 2-4 hold: it does not finish on this sweep.
    StillRunning,
    /// It finishes: `Stop(1)`, then one **queued** end message when
    /// `endMessageType != 0`.
    Finished { post_end_message: bool },
}

impl EffectRuntime {
    /// §7 rule 1: counted as done at once, with no stop and no message.
    pub fn counts_as_done_immediately(&self) -> bool {
        !self.started || self.index < 0
    }

    /// The result of one draw-pass sweep (`0x005BFD70`).
    ///
    /// §7 Corrects (re-integrate1): particles and decals are **not** ignored —
    /// they carry `flags = 0` and are exactly the components that hold an
    /// effect. The ignore bit is set uniformly on game, attractor and camera
    /// components and on 98.7% of sound, and it is the **runtime** bit 2 that
    /// is tested.
    pub fn sweep(&self, components: &[RuntimeComponent]) -> SweepResult {
        if self.counts_as_done_immediately() {
            return SweepResult::DoneWithoutMessage;
        }
        // Rule 2: "no auto stop".
        if self.flags & flags::NO_AUTO_STOP != 0 {
            return SweepResult::StillRunning;
        }
        // Rule 3: any non-ignored component still active.
        if components.iter().any(RuntimeComponent::blocks_finish) {
            return SweepResult::StillRunning;
        }
        // Rule 4: bit 7 without a soft stop.
        if self.flags & flags::STOPPING_ONLY != 0 && !self.stopping {
            return SweepResult::StillRunning;
        }
        SweepResult::Finished {
            post_end_message: self.end_message_type != 0,
        }
    }
}

/// The chosen `volcanocreator*` child of `volcano_control`, as §10.1 measures
/// it. Nine meta creators sit in one select group and exactly one is built;
/// its meta component's `IsActive` is the child effect's `started` byte, not
/// the child's components' `IsActive`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VolcanoCreatorChild {
    /// The child was finished by a **previous** draw-pass sweep.
    pub finished_on_an_earlier_sweep: bool,
    /// The child's brush component is still active.
    pub brush_active: bool,
    /// One of the child's 2-4 particle components is still active.
    pub particle_active: bool,
    /// One of the child's 3 decal components is still active.
    pub decal_active: bool,
    /// The child's attractor component (never holds anything: `flags = 1`).
    pub attractor_active: bool,
    /// The child's game record (never holds anything: `flags = 1`).
    pub game_active: bool,
}

impl VolcanoCreatorChild {
    /// §10.1 Corrects: `volcano_control` finishes at the first draw-pass sweep
    /// after (i) the chosen `volcanocreator*` child has itself been finished on
    /// a previous sweep, and (ii) at that sweep that child's **brush**, its
    /// **2-4 particle** components and its **3 decal** components are all
    /// inactive. The attractor and the game record never hold it.
    ///
    /// A static bound: the end is at least the child's brush length plus one
    /// extra draw-pass sweep, and at most that plus any decal lifetime.
    /// **Particles are the sole unknown** (§16 EF1).
    pub fn control_finishes(&self) -> bool {
        self.finished_on_an_earlier_sweep && !self.brush_active && !self.particle_active && !self.decal_active
    }
}

/// How a slave effect was reached (§10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlavePath {
    /// Reached as a **meta component** of a control effect the instance
    /// started.
    MetaChild,
    /// Spawned by a particle (the `_slave`, `_death`, `_collide`, `_terrain`
    /// and `_water` suffixes).
    ParticleSpawned,
}

/// §10.2 Corrects: "`data1 = the instance`" holds **only** for slaves reached
/// as meta children.
///
/// There is no single spawn API: each particle class builds a child effect
/// name from its own base name plus a fixed suffix and calls `CreateVisualEffect`.
/// Parameter 13 reaches **game components only**, so a slave spawned through
/// the particle path gets the particle's own `+0x26C` value and not the
/// disaster instance. Whether the location update matches on something other
/// than `data1 == instance` is open (§16 EF6).
pub fn data1_is_the_instance(path: SlavePath) -> bool {
    matches!(path, SlavePath::MetaChild)
}

/// The game component's `data` (`+0x58`) a location message carries as
/// `data1` (§10.2). Parameter 13 (`SetParameter(13, &instance, 1)`) reaches
/// game components only, so this is the instance only for a slave the instance
/// itself started as a meta child.
pub fn location_message_data1(path: SlavePath, particle_param: u64, instance: u64) -> u64 {
    match path {
        SlavePath::MetaChild => instance,
        SlavePath::ParticleSpawned => particle_param,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [EF3] §7 Corrects, conformance test 12: the description bit is not the
    /// tested bit.
    #[test]
    fn the_description_bit_is_copied_to_runtime_bit_2() {
        // A component with desc+0x10 & 1 set is built with runtime bit 2 (4)
        // and is skipped by the finish test.
        let ignored = RuntimeComponent::from_description(1, component_type::GAME, true);
        assert_eq!(ignored.flags, 6, "bit 2 (4) plus the table bit (2)");
        assert!(ignored.ignored());
        assert!(!ignored.blocks_finish(), "an ignored component never blocks");

        // One with it clear is built with bit 2 clear and blocks while active.
        let counting = RuntimeComponent::from_description(0, component_type::PARTICLES, true);
        assert_eq!(counting.flags, 0);
        assert!(!counting.ignored());
        assert!(counting.blocks_finish());
        assert!(!RuntimeComponent::from_description(0, component_type::PARTICLES, false).blocks_finish());

        // Table types 3-15 additionally get runtime bit 1 (value 2).
        for t in 3u8..=15 {
            assert_eq!(runtime_flags(0, t), 2, "type {t}");
            assert_eq!(runtime_flags(1, t), 6, "type {t}");
        }
        for t in [0u8, 1, 2, 16] {
            assert_eq!(runtime_flags(0, t), 0, "type {t}");
            assert_eq!(runtime_flags(1, t), 4, "type {t}");
        }
    }

    /// [EF3] §7 Corrects: particles and decals are not ignored. Measured over
    /// the 3373 shipped component records, `flags = 0` on all 1821 + 100
    /// particle, 406 decal, 591 meta, 25 brush, 10 sequence and 3 model
    /// components.
    #[test]
    fn particles_and_decals_count() {
        for (desc_flags, kind, name) in [
            (0u32, 0u8, "particles"),
            (0, 1, "decal"),
            (0, 2, "meta"),
            (0, 3, "brush"),
            (0, 6, "sequence"),
            (0, 16, "model"),
            (2, 0, "particles with the render bit"),
        ] {
            let c = RuntimeComponent::from_description(desc_flags, kind, true);
            assert!(c.blocks_finish(), "{name} holds an effect");
        }
        // The ignore bit is uniform on game, attractor and camera (56, 64 and
        // 41 shipped records), and on 299 of 303 sound components.
        for (kind, name) in [(4u8, "attractor"), (5, "game"), (8, "camera"), (7, "sound")] {
            let c = RuntimeComponent::from_description(1, kind, true);
            assert!(!c.blocks_finish(), "{name} never holds an effect");
        }
    }

    /// §7 conformance test 11: the finish rule.
    #[test]
    fn the_finish_rule() {
        let live = |is_active| RuntimeComponent { flags: 0, is_active };
        let ignored_live = RuntimeComponent {
            flags: 4,
            is_active: true,
        };
        let base = EffectRuntime {
            index: 3,
            started: true,
            stopping: false,
            flags: 0,
            end_message_type: 0x2A7B_582D,
            end_message_data1: 7,
        };

        // Rule 1: not started or index -1 is done at once, with no message.
        assert_eq!(
            EffectRuntime { started: false, ..base }.sweep(&[]),
            SweepResult::DoneWithoutMessage
        );
        assert_eq!(
            EffectRuntime { index: -1, ..base }.sweep(&[]),
            SweepResult::DoneWithoutMessage
        );

        // Rules 2-4.
        assert_eq!(
            EffectRuntime {
                flags: flags::NO_AUTO_STOP,
                ..base
            }
            .sweep(&[]),
            SweepResult::StillRunning
        );
        assert_eq!(base.sweep(&[live(true)]), SweepResult::StillRunning);
        assert_eq!(
            EffectRuntime {
                flags: flags::STOPPING_ONLY,
                ..base
            }
            .sweep(&[]),
            SweepResult::StillRunning
        );
        assert_eq!(
            EffectRuntime {
                flags: flags::STOPPING_ONLY,
                stopping: true,
                ..base
            }
            .sweep(&[]),
            SweepResult::Finished { post_end_message: true }
        );

        // An ignored component that is still active does not hold it.
        assert_eq!(
            base.sweep(&[ignored_live]),
            SweepResult::Finished { post_end_message: true }
        );
        // Nothing left active: it finishes, and posts one queued message.
        assert_eq!(
            base.sweep(&[live(false)]),
            SweepResult::Finished { post_end_message: true }
        );
        // ...and nothing at all when the end message type is 0.
        assert_eq!(
            EffectRuntime {
                end_message_type: 0,
                ..base
            }
            .sweep(&[]),
            SweepResult::Finished {
                post_end_message: false
            }
        );
    }

    /// §10.1 Corrects, conformance test 15: the `volcano_control` end
    /// condition.
    #[test]
    fn volcano_control_finishes_after_the_child_and_its_components() {
        let idle = VolcanoCreatorChild {
            finished_on_an_earlier_sweep: true,
            ..VolcanoCreatorChild::default()
        };
        assert!(idle.control_finishes());

        // (i) not finished on a previous sweep: it waits a full extra sweep.
        assert!(!VolcanoCreatorChild {
            finished_on_an_earlier_sweep: false,
            ..idle
        }
        .control_finishes());
        // (ii) the child's brush, particles and decals all have to be inactive.
        for held in [
            VolcanoCreatorChild {
                brush_active: true,
                ..idle
            },
            VolcanoCreatorChild {
                particle_active: true,
                ..idle
            },
            VolcanoCreatorChild {
                decal_active: true,
                ..idle
            },
        ] {
            assert!(!held.control_finishes());
        }
        // The attractor and the game record never hold it (both `flags = 1`).
        assert!(VolcanoCreatorChild {
            attractor_active: true,
            game_active: true,
            ..idle
        }
        .control_finishes());
    }

    /// §10.2 Corrects: `data1 = the instance` only for meta-reached slaves.
    #[test]
    fn data1_is_the_instance_only_for_meta_children() {
        const INSTANCE: u64 = 0x1111;
        const PARTICLE_PARAM: u64 = 0x26C;
        assert!(data1_is_the_instance(SlavePath::MetaChild));
        assert!(!data1_is_the_instance(SlavePath::ParticleSpawned));
        assert_eq!(
            location_message_data1(SlavePath::MetaChild, PARTICLE_PARAM, INSTANCE),
            INSTANCE
        );
        assert_eq!(
            location_message_data1(SlavePath::ParticleSpawned, PARTICLE_PARAM, INSTANCE),
            PARTICLE_PARAM,
            "a particle-spawned slave carries the particle's own value"
        );
    }
}

/// The component types of §3, repeated here so `fc4-core` stays free of a
/// dependency on `fc4-formats`.
//! The effects manager and the effects the simulation depends on
//! (`docs/specs/effects.md`): the effect directory ([`directory`]), effect
//! creation with the priority filter, levels of detail and select groups,
//! the dilated animation tick, finishing and the end message
//! ([`EffectsManager`]), the brush effect ([`brush`]) and the game effect
//! ([`game`]), and the shipped records and disaster mapping as data
//! ([`shipped`]).
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
//! sections are located, not read (EF6); the components' local placement is
//! the identity (EF7); a brush's second `Stop` ends the long terrain
//! operation again (EF8); a meta component updates its child although the
//! child is also on the active list (EF9).

pub mod brush;
pub mod directory;
pub mod game;
mod manager;
pub mod shipped;

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

/// Component types (§3, `+0x0C`).pub mod component_type {
    pub const PARTICLES: u8 = 0;
    pub const DECAL: u8 = 1;
    pub const META: u8 = 2;
    pub const BRUSH: u8 = 3;
    pub const ATTRACTOR: u8 = 4;
    pub const GAME: u8 = 5;
    pub const SEQUENCE: u8 = 6;
    pub const SOUND: u8 = 7;
    pub const CAMERA: u8 = 8;
    pub const MODEL: u8 = 16;
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
    /// `IsActive` of a render component (the finish test, §7), by effect,
    /// component index and type. False by default: render components do
    /// not hold an effect [choice, EF1].
    fn render_component_active(&mut self, _effect: EffectId, _component: u32, _kind: u8) -> bool {
        false
}}
