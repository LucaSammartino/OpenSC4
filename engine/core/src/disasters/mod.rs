// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The disaster layer (`cSC4DisasterLayer`, class `0x61F6ABF5`) and its seven
//! managers, `docs/specs/disasters.md`. Section references (`§n`) point at
//! that spec.
//!
//! # Structure
//!
//! - `layer.rs`: the layer of §2 (enable counter, begin / end with the
//!   emergency pause, damage assessment, dispatch timers, locations).
//! - `fire.rs`: the fire instance of §5 and the fire manager's slots.
//! - `riot.rs`: the riot of §6.
//! - `toxic.rs`: the toxic spill of §9.
//! - `quake.rs`, `pipe.rs`, `volcano.rs`, `effect.rs`: the short-form
//!   disasters of §7, §8, §10 and §11.
//! - `system.rs`: [`Disasters`], the one object that owns the layer, the
//!   manager framework of §3 and every instance, with the re-entrancy guard.
//! - `tests.rs`: a fake host and the §18 tests.
//!
//! Everything outside the layer goes through [`DisasterHost`]: the
//! simulator's emergency pause, the city's save flag, the occupants and
//! their properties, the fire-protection, police and flammability grids, the
//! occupant manager, the demolition utility, the automata, the terrain, the
//! view and the message server. Objects are host tokens (`u64`); disaster
//! instances are core IDs (`u32`, from 1) that the host maps to its GZCOM
//! objects.
//!
//! # The clock *[compat]*
//!
//! Disasters run on the wall clock (§0). The core reads it only through
//! [`DisasterHost::now_ms`] (the fire's and the toxic spill's millisecond
//! timers, the riot's and the dispatch timers' seconds) and through the
//! microseconds the host passes to [`Disasters::tick`] (the animation tick's
//! data2, §7, §8, §10). Tests drive both, so every result is deterministic.
//!
//! # Choices (`[choice]`) and literal readings (`[question]`)
//!
//! The numbered questions of the lane report (disasters.md §17 Q6 onwards)
//! are cited in the code as `Q6`... Highlights:
//!
//! - **The fire's generator** (§13, flammability §13.1: a `cRZRandom` of the
//!   fire instance): seeded with `derive_seed(S, 0x69C1718F)` (the instance
//!   class, foundations §6.3 rule for member generators seeded from the
//!   clock) [choice, Q6]. `Random(4000)` is `RandomUint32Uniform(4000)`; the
//!   spread draw is `RandomDoubleRangeUniform(0, 1)`, made only by a step
//!   that reaches the spread; the two cell draws are
//!   `RandomDoubleRangeGaussianFast` over the box widened by the radius and
//!   the wind vector, x first (§5.10 step 9, §13).
//! - **`IgniteCell`** follows flammability.md §13.1 where disasters.md §5.5
//!   is shorter: the grid is read and the curve evaluated on every call, the
//!   probability draw is `RandomDoubleRangeUniform(0, 100)` and fails only
//!   when `r > c`, and a zero fire ID is drawn **once per call**
//!   (`RandomUint32Uniform()`) after the probability check, then passed to
//!   every `Ignite` (Q8).
//! - **Re-entrancy**: host callbacks may deliver messages to the layer (an
//!   occupant removed by a burn-down). [`Disasters`] defers a message that
//!   arrives while the core is busy and handles it when the outer call
//!   returns; any other re-entrant call returns its fallback and is counted
//!   (`reentrant_refused`) [choice].
//! - **Short forms** (§7–§11): the geometry the spec leaves free (the fault
//!   line, the leak spread) is the simplest that uses the spec's tuning
//!   [choice]; the transitions the spec does not give are listed as questions
//!   in the module that needs them.

mod effect;
mod fire;
mod layer;
mod pipe;
mod quake;
mod riot;
mod system;
mod toxic;
mod volcano;

#[cfg(test)]
mod tests;

pub use fire::{
    fire_prop, shipped_flame, FireEntry, FireSnapshot, FireTuning, SHIPPED_D3, SHIPPED_D4, SHIPPED_D5, SHIPPED_D6,
    SHIPPED_D7, SHIPPED_D8, SHIPPED_D9, SHIPPED_DA, SHIPPED_ZOOM,
};
pub use layer::{DamageAssessment, LayerSnapshot, Location, SpecialBuilding, Timer};
pub use pipe::{Leak, PipeSnapshot, PipeTuning};
pub use quake::{QuakeSnapshot, QuakeTuning};
pub use riot::{
    riot_prop, shipped_riot, Leader, RiotSnapshot, RiotTuning, SHIPPED_LEADERS_SPAWNED, SHIPPED_POLICE_CURVE,
};
pub use system::{instance_class_of, Disasters, InstanceInfo, ManagerSnapshot};
pub use toxic::{ToxicSnapshot, ToxicSpill};
pub use volcano::{
    shipped_volcano, volcano_phase, volcano_prop, VolcanoSnapshot, VolcanoTuning, SHIPPED_LAVA_SECONDS,
    SHIPPED_PRODUCTION, SHIPPED_SCALE, SHIPPED_SOLIDIFICATION,
};

use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::rng::{RzRandom, SeedSource};

/// The layer's class and interfaces (§1).
pub const CLSID_LAYER: u32 = 0x61F6_ABF5;
pub const IID_DISASTER_LAYER: u32 = 0x29B0_2A2C;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
/// `cISC4DisasterManager` (the IID of every manager's `CreateInstance`).
pub const IID_DISASTER_MANAGER: u32 = 0x021E_9CBF;
/// `cISC4DisasterManagerFire`.
pub const IID_DISASTER_MANAGER_FIRE: u32 = 0x49B4_3F57;

/// Manager types (`GetID`, §1). An instance's `GetID` is its manager's type.
pub mod kind {
    pub const VOLCANO: u32 = 0xC4C9_0997;
    pub const FIRE: u32 = 0x621C_DA33;
    pub const EARTHQUAKE: u32 = 0x4296_3812;
    pub const PIPE_BURST: u32 = 0x4A13_32E8;
    pub const RIOT: u32 = 0xCA38_0B06;
    pub const EFFECT: u32 = 0x6A7B_4397;
    pub const TOXIC: u32 = 0xEBFB_0404;
}

/// The static manager table of §1, (type, class) in table order *[compat]*.
pub const MANAGER_TABLE: [(u32, u32); 7] = [
    (kind::VOLCANO, 0xA9B5_AB9F),
    (kind::FIRE, 0x69C1_718E),
    (kind::EARTHQUAKE, 0x0A08_6BD3),
    (kind::PIPE_BURST, 0x4A13_2FDF),
    (kind::RIOT, 0x4A38_206C),
    (kind::EFFECT, 0x6A7B_3E9B),
    (kind::TOXIC, 0xEBFB_0402),
];

/// Whether `t` is one of the seven layer managers (§11 menu tool).
pub fn is_layer_manager(t: u32) -> bool {
    MANAGER_TABLE.iter().any(|(m, _)| *m == t)
}

/// Instance classes (§1).
pub mod instance_class {
    pub const VOLCANO: u32 = 0x29B5_AB89;
    pub const FIRE: u32 = 0x69C1_718F;
    pub const EARTHQUAKE: u32 = 0xAA08_6D86;
    pub const PIPE_BURST: u32 = 0x0A13_3012;
    pub const RIOT: u32 = 0x4A38_206D;
    pub const EFFECT: u32 = 0x6A7B_3E9C;
    pub const TOXIC: u32 = 0xEBFB_0403;
}

/// The fire occupant class (§5.11).
pub const CLSID_FIRE_OCCUPANT: u32 = 0x69D2_5C5F;

/// Generator constants (§1, foundations §6.3).
pub mod seed {
    /// The fire instance's own `cRZRandom`: the instance class [choice, Q6].
    pub const FIRE: u32 = super::instance_class::FIRE;
    /// File-local generators (foundations §6.3 table).
    pub const EARTHQUAKE: u32 = crate::rng::file_seed::EARTHQUAKE;
    pub const PIPE_BURST: u32 = crate::rng::file_seed::PIPE_BURST;
    pub const RIOT: u32 = crate::rng::file_seed::RIOT;
    /// The argument-less pipe burst's throwaway generator (§8): a temporary
    /// seeded from the clock, so `K` = the manager class XOR n (foundations
    /// §6.3).
    pub const PIPE_BURST_MANAGER: u32 = 0x4A13_2FDF;
}

/// Messages (§12, §2.5, §5.3, §8, §9, §11).
pub mod msg {
    pub const DISASTER_PERIOD_STARTED: u32 = 0xCA1B_1E64;
    pub const DISASTER_PERIOD_ENDED: u32 = 0xAA1B_2002;
    pub const FIRE_STARTED_ENDED: u32 = 0x49E3_DB1D;
    pub const IGNITIONS: u32 = 0x49D1_5B73;
    pub const SEVERITY: u32 = 0x49D1_5B72;
    pub const PHASE: u32 = 0xA9C7_CAB6;
    pub const PIPE_BURST_AT_CELL: u32 = 0xEA1F_0DCD;
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    pub const DISPATCH_BEGIN: u32 = 0xE946_71EB;
    pub const DISPATCH_ACTIVE: u32 = 0xE946_71EC;
    pub const DISPATCH_INACTIVE: u32 = 0xE946_71ED;
    pub const DISPATCH_END: u32 = 0x8946_71F6;
    /// The damage assessment's subscriptions (§2.5 step 1), in order.
    pub const ASSESSMENT: [u32; 5] = [
        OCCUPANT_REMOVED,
        DISPATCH_ACTIVE,
        DISPATCH_BEGIN,
        DISPATCH_END,
        DISPATCH_INACTIVE,
    ];
    pub const VIEW_LEVEL_CHANGED: u32 = 0x06CD_B65B;
    pub const VIEW_UPDATE: u32 = 0xA6B7_9602;
    pub const PRE_VIEW_LEVEL_CHANGE: u32 = 0xC6CD_BF1A;
    pub const ANIMATION_TICK: u32 = 0x86AD_10EE;
    pub const HOUR: u32 = 0x6695_6813;
    pub const TOXIC_SPILL_CHEAT: u32 = 0x0BFB_41BA;
    pub const VOLCANO_MESSAGE: u32 = 0xCA49_3C1D;
    pub const EFFECT_FINISHED: u32 = 0x2A7B_582D;
    pub const EFFECT_MOVED: u32 = 0x2A7B_582E;
}

/// History series (§12).
pub mod history {
    pub const FIRE_EPISODES: u32 = 0x49D1_5B74;
    pub const RIOTS: u32 = 0xAA8C_87C7;
}

/// Occupant types (§5.3 step 5, §2.5).
pub mod occupant {
    pub const FLORA: u32 = 0x7475_8926;
    pub const BUILDING: u32 = 0x2781_28A0;
    pub const PROP: u32 = 0xA823_821E;
    pub const NETWORK: u32 = 0xC772_BF98;
    /// The fire's occupant-type filter (§5.3 step 5).
    pub fn burnable(t: u32) -> bool {
        t == FLORA || t == BUILDING || t == PROP
    }
}

/// Occupant properties.
pub mod prop {
    /// The demolition cost (int64, §2.5).
    pub const DEMOLITION_COST: u32 = 0x8A1C_3E72;
    /// Flammability (non-zero = can burn, §5.4 step 3).
    pub const FLAMMABILITY: u32 = 0x2924_4DB5;
    /// "Burning" (§5.4 step 5, §5.11).
    pub const BURNING: u32 = 0x69DB_EBE1;
    /// The maximum fire stage (default 2, §5.10).
    pub const MAX_STAGE: u32 = 0x49BE_DA31;
    /// Burn down through demolition (default true, §5.10 `ToastOccupant`).
    pub const BURN_DOWN_DEMOLISHES: u32 = 0x2A89_B677;
    /// Extra demolition flags (§5.10).
    pub const DEMOLITION_FLAGS: u32 = 0x6A43_150F;
    /// Radiation spew amount and radius (§5.10).
    pub const RADIATION_AMOUNT: u32 = 0xAA5C_04C5;
    pub const RADIATION_RADIUS: u32 = 0xAA5C_04C8;
    /// The disaster's zoom level (u8, every tuning exemplar).
    pub const ZOOM_LEVEL: u32 = 0xCA82_1B8C;
}

/// The cost of a removed network piece without `0x8A1C3E72` (§2.5 step 3:
/// one int64 constant for every network type).
pub const NETWORK_DEFAULT_COST: i64 = 10;
/// The default maximum fire stage (§5.10).
pub const DEFAULT_MAX_STAGE: u32 = 2;
/// The fire step period, ms (§5.1).
pub const FIRE_PERIOD_MS: u64 = 1000;
/// `ProcessToastList` limit per `Update` (§5.7).
pub const TOAST_PER_UPDATE: usize = 5;
/// The burn-down demolition flags before `0x6A43150F` (§5.10).
pub const BURN_DOWN_FLAGS: u32 = 0x300;
/// The pipe-burst demolition flags (§8).
pub const PIPE_BURST_FLAGS: u32 = 0x302;
/// Animation-tick priorities (§5.9, §7, §9).
pub const TICK_PRIORITY: u32 = 0xC0;
pub const TOXIC_TICK_PRIORITY: u32 = 0xFF;

/// Tuning exemplars: type and group (§5.1, §6.1, §7, §8, §10), and the
/// instances.
pub const TUNING_TYPE: u32 = 0x6534_284A;
pub const TUNING_GROUP: u32 = 0x29B6_C670;
pub mod tuning_instance {
    pub const VOLCANO: u32 = 1;
    pub const EARTHQUAKE: u32 = 2;
    pub const PIPE_BURST: u32 = 3;
    pub const FLAME: u32 = 0xC9B6_B73D;
    pub const RIOT: u32 = 0x4A38_206D;
}

/// A cell rectangle, inclusive on both ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

impl CellRect {
    pub fn cell(x: i32, z: i32) -> CellRect {
        CellRect {
            x0: x,
            z0: z,
            x1: x,
            z1: z,
        }
    }
    pub fn contains(&self, x: i32, z: i32) -> bool {
        x >= self.x0 && x <= self.x1 && z >= self.z0 && z <= self.z1
    }
    pub fn intersects(&self, o: &CellRect) -> bool {
        self.x0 <= o.x1 && o.x0 <= self.x1 && self.z0 <= o.z1 && o.z0 <= self.z1
    }
}

/// A world-space box (x and z, metres).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldBox {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
}

/// A message the core posts (§12). `data1` may carry a host token (the fire
/// occupant of `0x49E3DB1D`); `void1` is an instance (0 = none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisasterPost {
    pub msg_type: u32,
    pub data1: u64,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    /// The instance of `0xCA1B1E64` (0 = none).
    pub void1: u32,
}

impl DisasterPost {
    pub fn new(msg_type: u32) -> DisasterPost {
        DisasterPost {
            msg_type,
            ..DisasterPost::default()
        }
    }
}

/// A message delivered to the layer, a manager or an instance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisasterMessage {
    pub msg_type: u32,
    pub data1: i64,
    pub data2: i64,
    pub data3: i64,
    /// `GetVoid1()` as a host token (the removed occupant; the effect).
    pub void1: u64,
    /// A position (the effect moved, §11), world units.
    pub x: f32,
    pub z: f32,
}

impl DisasterMessage {
    pub fn of_type(msg_type: u32) -> DisasterMessage {
        DisasterMessage {
            msg_type,
            ..DisasterMessage::default()
        }
    }
}

/// Who receives a message or a subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Layer,
    /// A manager, by type.
    Manager(u32),
    /// An instance, by core ID.
    Instance(u32),
}

/// The draws the disasters make (§13, foundations §6.1 method names).
pub trait DisasterRandom {
    /// `RandomUint32Uniform()`.
    fn raw_u32(&mut self) -> u32;
    /// `RandomUint32Uniform(n)`.
    fn uniform_u32(&mut self, n: u32) -> u32;
    /// `RandomDoubleUniform()`.
    fn uniform_f64(&mut self) -> f64;
    /// `RandomDoubleRangeUniform(a, b)`.
    fn range_f64(&mut self, a: f64, b: f64) -> f64;
    /// `RandomDoubleRangeGaussianFast(a, b)` (three uniform draws).
    fn range_gaussian_fast(&mut self, a: f64, b: f64) -> f64;
}

impl DisasterRandom for RzRandom {
    fn raw_u32(&mut self) -> u32 {
        self.next_raw()
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        RzRandom::uniform_u32(self, n)
    }
    fn uniform_f64(&mut self) -> f64 {
        RzRandom::uniform_f64(self)
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        RzRandom::range_f64(self, a, b)
    }
    fn range_gaussian_fast(&mut self, a: f64, b: f64) -> f64 {
        RzRandom::range_gaussian_fast(self, a, b)
    }
}

/// Which generator a draw comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generator {
    Fire,
    Riot,
    Earthquake,
    PipeBurst,
    /// The argument-less pipe burst's throwaway generator (§8).
    PipeBurstStart,
}

/// A generator for a core that mirrors the original layer (the plugin's
/// `disasters_shadow`; tooling, not spec behaviour). The original's draws
/// cannot be read, so the mirror core makes none of its own decisions that
/// depend on one; every outcome the original drew arrives as a mirrored
/// call instead:
///
/// - `raw_u32` (a new fire ID) counts from `0x8000_0001`, so the mirror's
///   own IDs are recognisable;
/// - `uniform_u32(n)` is 0 (a covered fire's change, `Random(4000)`);
/// - `uniform_f64` is 1.0;
/// - `range_f64(a, b)` is `b`: the spread test `r < 1 / D4[s]` (§5.10 step
///   9, `r` drawn on [0, 1]) never passes for a D4 of 1 or more, so the
///   original's spreads arrive as its own `Ignite` calls, and the
///   `IgniteCell` probability draw fails for any curve value below 100;
/// - `range_gaussian_fast(a, b)` is `a`.
#[derive(Clone, Debug, Default)]
pub struct MirrorRandom {
    next_id: u32,
}

/// The first fire ID a [`MirrorRandom`] gives.
pub const MIRROR_FIRST_ID: u32 = 0x8000_0001;

impl DisasterRandom for MirrorRandom {
    fn raw_u32(&mut self) -> u32 {
        let id = MIRROR_FIRST_ID.wrapping_add(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
    fn uniform_u32(&mut self, _n: u32) -> u32 {
        0
    }
    fn uniform_f64(&mut self) -> f64 {
        1.0
    }
    fn range_f64(&mut self, _a: f64, b: f64) -> f64 {
        b
    }
    fn range_gaussian_fast(&mut self, a: f64, _b: f64) -> f64 {
        a
    }
}

/// A seed source that never reads a clock: FreeCity4 derives every seed
/// from the session seed (foundations §6.3), and a zero multiplicative
/// state falls back to the same derived seed.
pub(crate) struct DerivedSeeds(pub u32);

impl SeedSource for DerivedSeeds {
    fn wall_clock_seed(&self) -> u32 {
        self.0
    }
    fn process_clock(&self) -> u32 {
        self.0
    }
}

/// Everything the disasters read from, or do to, the rest of the game.
/// Objects are host tokens; instances are core IDs. Callbacks may deliver
/// messages back to [`Disasters::do_message`] (they are deferred); other
/// calls into the core from a callback are refused.
pub trait DisasterHost {
    // ---- the clock (§0) ----
    /// The wall clock in milliseconds (any epoch; only differences count).
    fn now_ms(&self) -> u64;

    // ---- simulator and city (§2.4, §5.4) ----
    /// Simulator slot 11 / slot 14.
    fn emergency_pause(&self);
    fn emergency_resume(&self);
    /// City slot 104 / slot 103.
    fn disable_save(&self);
    fn enable_save(&self);
    /// Simulator slot 15 (`IsPaused`) and slot 17 (`IsEmergencyPaused`).
    fn sim_paused(&self) -> bool;
    fn sim_emergency_paused(&self) -> bool;
    /// City slot 22 (`GetEstablished`).
    fn city_established(&self) -> bool;
    /// The city's cell counts (x, z).
    fn city_cells(&self) -> (u32, u32);
    /// City slot 91: the cell width in metres (16).
    fn cell_width(&self) -> f32;

    // ---- messages, ticks, history ----
    fn post(&self, post: &DisasterPost);
    fn subscribe(&self, target: Target, msg_type: u32);
    fn unsubscribe(&self, target: Target, msg_type: u32);
    /// Animation tick manager slot 5 / slot 6 for an instance.
    fn register_tick(&self, instance: u32, priority: u32);
    fn unregister_tick(&self, instance: u32);
    /// History slot 12.
    fn record_history(&self, series: u32, value: i32);
    /// A tuning exemplar of group [`TUNING_GROUP`] ([`tuning_instance`]).
    fn tuning(&self, instance: u32) -> Option<Rc<PropertyHolder>>;

    // ---- references ----
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);

    // ---- the view (§2.7) ----
    /// The "follow disasters" preference byte.
    fn follow_disasters(&self) -> bool;
    /// `GoToNextDisaster`'s camera move to a cell, zooming in to `zoom`.
    fn go_to_location(&self, x: i32, z: i32, zoom: u32);

    // ---- damage assessment (§2.5) ----
    /// Fire protection slot 25 / police slot 26: `GetTotalDispatchCount()`,
    /// the dispatch units over all stations (the effectiveness divisor).
    fn fire_dispatch_count(&self) -> u32;
    fn police_dispatch_count(&self) -> u32;
    /// The BDS building-type flags of a building occupant.
    fn building_type_flags(&self, occupant: u64) -> u32;

    // ---- occupants ----
    fn occupant_type(&self, occupant: u64) -> u32;
    /// Occupant manager slot 32 with 1: still in the occupant manager.
    fn occupant_in_manager(&self, occupant: u64) -> bool;
    /// The occupant's cell (a location entry's x and z); `None` when it is
    /// no longer in the occupant manager.
    fn occupant_cell(&self, occupant: u64) -> Option<(i32, i32)>;
    /// The occupant's cell rectangle (clamped to the city).
    fn occupant_cell_rect(&self, occupant: u64) -> CellRect;
    /// The occupant's position (x, z), world units.
    fn occupant_position(&self, occupant: u64) -> (f32, f32);
    /// The occupant's bounding box (x, z), world units.
    fn occupant_bbox(&self, occupant: u64) -> WorldBox;
    /// The top and side areas of the bounding box (the flame counts, §5.11).
    fn occupant_bbox_areas(&self, occupant: u64) -> (f32, f32);
    /// Property readers (`None` = absent).
    fn occupant_prop_u32(&self, occupant: u64, id: u32) -> Option<u32>;
    fn occupant_prop_i64(&self, occupant: u64, id: u32) -> Option<i64>;
    fn occupant_prop_f32(&self, occupant: u64, id: u32) -> Option<f32>;
    fn occupant_prop_bool(&self, occupant: u64, id: u32) -> Option<bool>;
    /// Every occupant in a cell rectangle (occupant manager slot 40), in the
    /// manager's order; the core applies the type filter.
    fn occupants_in_rect(&self, rect: CellRect) -> Vec<u64>;
    /// Occupant manager slot 21.
    fn remove_occupant(&self, occupant: u64);
    /// The lot manager's lot for the occupant.
    fn occupant_lot(&self, occupant: u64) -> Option<u64>;
    /// Lot slot 36.
    fn set_lot_condition(&self, lot: u64, condition: u32);

    // ---- fire (§5) ----
    /// The average fire-protection coverage over the occupant's cell
    /// rectangle, clamped to the city, as a byte (§5.4 step 4).
    fn fire_coverage(&self, occupant: u64) -> u8;
    /// The adjusted flammability grid value at a cell (§5.5).
    fn flammability_at(&self, x: i32, z: i32) -> u8;
    /// Creates a fire occupant (`0x69D25C5F`), `Init`s it, sets its target
    /// (the target's `0x69DBEBE1` := true, occupant slot 12 with 2, the
    /// "fire_occupant" automata for a building), its fire ID and flame
    /// counts, and inserts it in the occupant manager (§5.4 step 7, §5.11).
    fn create_fire_occupant(&self, instance: u32, target: u64, fire_id: u32, flames: (u32, u32)) -> Option<u64>;
    /// Removes a fire occupant from the occupant manager; the target's
    /// `0x69DBEBE1` goes back to false (§5.10 `KillFire`, §5.11).
    fn remove_fire_occupant(&self, fire: u64, target: u64);
    /// A transient aura effect at the occupant's position (aura.md §8.2).
    fn add_fire_aura(&self, occupant: u64);
    /// Tutorial system slot 8.
    fn tutorial_no_burn_down(&self) -> bool;
    /// Weather slot 22: the wind vector (x, z).
    fn wind(&self) -> (f32, f32);
    /// A visual effect on an occupant.
    fn play_effect(&self, name: &str, occupant: u64);
    /// A visual effect at a position (world units).
    fn play_effect_at(&self, name: &str, x: f32, z: f32);
    /// Flora simulator slot 23.
    fn burn_flora(&self, occupant: u64);
    /// `DemolishRegion(1, the lot's region, 0, flags, …)`.
    fn demolish_lot(&self, occupant: u64, lot: u64, flags: u32);
    /// Pollution slot 66 at the occupant's position.
    fn spew_radiation(&self, occupant: u64, amount: f32, radius: f32);

    // ---- riot (§6) ----
    /// The police coverage at a cell (police slot 5 grid, `GetCellValue`).
    fn police_coverage(&self, x: i32, z: i32) -> u8;
    /// The cell has a road tile of network type 0, 3 or 6.
    fn is_road_cell(&self, x: i32, z: i32) -> bool;
    /// City slot 96: a cell's corner in world units.
    fn cell_corner_position(&self, x: i32, z: i32) -> (f32, f32);
    /// Creates a riot-leader pedestrian (kind `0x4107`) at a position on the
    /// terrain, sets it to riot-leader mode and starts it.
    fn create_riot_leader(&self, x: f32, z: f32) -> Option<u64>;
    /// The occupant of the traffic network map's tile at a cell
    /// (`SafeGetTileInfoAtCell`, slot 14); `None` for no tile, no occupant
    /// or a cell off the map (network-manager.md §10).
    fn network_occupant_at(&self, x: i32, z: i32) -> Option<u64>;
    /// `AsOccupant().UnsetRemovalFlags(flags)` (cISC4Occupant slot 13): the
    /// riot's removal lock decrements the player's counter (flags 2).
    fn unset_removal_flags(&self, occupant: u64, flags: u32);
    /// `SetRemovalFlags(flags)`: the lock's release increments it back.
    fn set_removal_flags(&self, occupant: u64, flags: u32);
    /// The automaton's cell; `None` when it is gone.
    fn automaton_cell(&self, automaton: u64) -> Option<(i32, i32)>;
    /// The "Riot%d" effects for a stage.
    fn set_riot_effect(&self, automaton: u64, stage: i32);
    /// The formatted riot message to the automata controller manager.
    fn send_riot_message(&self, automaton: u64, stage: i32);
    /// Automaton slot 51 with 1 and slot 14 (the leader ends).
    fn end_riot_leader(&self, automaton: u64);
    /// `DoRiotLeaderTick`.
    fn riot_leader_tick(&self, automaton: u64);

    // ---- toxic spill (§9) ----
    fn create_toxic_occupant(&self, instance: u32, x: i32, z: i32, amount: i32) -> Option<u64>;
    fn set_toxic_amount(&self, occupant: u64, amount: i32);
    /// Removes it from the occupant manager and releases it.
    fn remove_toxic_occupant(&self, occupant: u64);

    // ---- terrain and automata (§7, §8, §10) ----
    /// Terrain slot 57 (`ApplyBrush`).
    fn apply_terrain_brush(&self, brush: u32, x: f32, z: f32, strength: f32, width: f32);
    /// Terrain slot 69 over the cells touched this tick.
    fn redraw_terrain(&self, rect: CellRect);
    /// `EndLongTerrainOperation`.
    fn end_long_terrain_operation(&self);
    /// Stop accumulating terrain texture updates (earthquake state 4).
    fn stop_terrain_texture_updates(&self);
    /// An automata repulsor at a position.
    fn place_repulsor(&self, x: f32, z: f32);

    // ---- pipe burst (§8) ----
    /// The first watered pipe in a cell (the manager's pipe filter).
    fn find_watered_pipe(&self, x: i32, z: i32) -> Option<u64>;
    /// The pipe's cell.
    fn pipe_cell(&self, pipe: u64) -> (i32, i32);
    /// The current view level (1 surface, 2 subway, 3 water pipes).
    fn view_level(&self) -> u32;
    /// Shows "burst_pipe_aboveground" (view level 1) or
    /// "burst_pipe_underground" (3) at the pipe; false (no effect) at any
    /// other level.
    fn create_leak_effect(&self, pipe: u64, view_level: u32) -> bool;
    /// The neighbouring connected pipe cells inside the city.
    fn connected_pipe_cells(&self, pipe: u64) -> Vec<(i32, i32)>;
    /// `DemolishRegion(1, pipe, 0, flags, …)`.
    fn demolish_pipe(&self, pipe: u64, flags: u32);

    // ---- volcano (§10) ----
    /// Starts the "volcano_control" effect; its completion is delivered as
    /// `0xCA493C1D` with data1 = the instance (§10 state 3).
    ///
    /// `volcano_control` is nine meta creators in one select group plus one
    /// attractor; exactly one creator is built. The effect finishes only after
    /// that child has finished on an earlier draw-pass sweep **and** the
    /// child's brush, particle and decal components are all inactive on this
    /// one (effects.md §10.1 Corrects). See
    /// [`crate::effects::VolcanoCreatorChild`].
    fn volcano_start_deformation(&self, instance: u32, x: f32, z: f32);
    fn volcano_start_lava(&self, instance: u32, x: f32, z: f32, radius: f32);
    fn volcano_lava_rates(&self, instance: u32, production: f32, solidification: f32);
    fn volcano_end_lava(&self, instance: u32);
}

/// Reads the zoom level byte of a tuning exemplar.
pub(crate) fn zoom_of(h: Option<&PropertyHolder>, default: u32) -> u32 {
    h.and_then(|h| crate::props::u32_of(h, prop::ZOOM_LEVEL))
        .map_or(default, |v| v & 0xFF)
}

/// `trunc` toward zero of an `f32` to `i32` (saturating; NaN → 0).
pub(crate) fn trunc_i32(v: f32) -> i32 {
    if v.is_nan() {
        0
    } else {
        v.trunc().clamp(i32::MIN as f32, i32::MAX as f32) as i32
    }
}
