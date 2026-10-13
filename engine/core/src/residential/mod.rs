// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The residential simulator (`cSC4ResidentialSimulator`, class
//! `0x4990C013`), `docs/specs/residential.md`. Section references (`§n`)
//! point at that spec unless they are prefixed `REH`, which cites
//! `docs/specs/residential-education-health.md` (the education and health
//! half of the same class).
//!
//! # Structure
//!
//! **Work in progress.** This module currently holds the frame: the IDs,
//! the agent and message constants, the grid and department tables, the host
//! contract and the two shared data types of §3 / §9.1, plus the two parts
//! that are self-contained and fully specified —
//!
//! - `geometry.rs`: the proximity seed and the four-sweep Manhattan distance
//!   passes of §5.3 ([`seed_proximity`], [`distance_pass`],
//!   [`distance_to_convergence`], [`copy_proximity`]).
//! - `tuning.rs`: the population tuning of §7.1 ([`PopulationTuning`]), the
//!   simulator tuning of §7.2 ([`SimulatorTuning`], whose `read` reproduces
//!   the §9.3 strike-reset quirk), the two per-call curves of §7.3
//!   ([`le_curve`], [`workforce_curve`]), [`new_ages_by_wealth`] (§6.5) and
//!   [`shipped_tuning`].
//!
//! Still to come: `simulator.rs` (the state of §3, the lifecycle of §4, the
//! daily state machine of §5.1–§5.2, `DoPopulationChange` §5.5,
//! `PostPopulationChange` §5.6, the history of §5.7, the queries of §6 and
//! the slot table of §10), `record.rs` (the save record of §9) and `tests.rs`.
//! The host contract below and the constants are written for those, so the
//! remaining work is filling them in.
//!
//! # What is deferred to the education and health half
//!
//! REH specifies the school and hospital building lists, the coverage maps,
//! the grading, the strikes, the auto-budget, the query data and the system
//! ratings. That work needs the budget simulator's departments and line
//! items, the occupant and lot managers, the ordinance simulator and the
//! automata controller manager, and it is large enough to be its own lane.
//! This module keeps the **frame** those states run in — the state machine,
//! the grids they write, the counters they fill and the record that saves
//! them — and hands the building work to [`ResidentialHost`] callbacks with
//! the contracts below. Everything the population half itself computes from
//! REH is implemented here exactly: the newcomer EQ/HQ blend (REH §4.7
//! step 4), `GetTractQuotient` (REH §4.8), `GetCellEQ` / `GetCellHQ`
//! (slots 35/36), `GetGlobalEQ` / `GetGlobalHQ` (slots 32/33), the life
//! expectancy and workforce queries (§6.4) and the ordinance decay products
//! that scale the two decays (§7.1).
//!
//! # Choices (`[choice]`), limits and inferences (`[inferred]`)
//!
//! - **Seeding** (§3, foundations §6.3): the class's one `cRZRandom` is a
//!   *member* generator seeded from the wall clock at construction, so its
//!   constant is the class's own GZCLSID: [`ResidentialRng::from_session`]
//!   uses `derive_seed(session, 0x4990C013)`. [`ResidentialRng::new`] keeps
//!   the original's clock seed for hosts that supply one. The stream is
//!   shared by the buzzards rolls (§5.5) and the monthly strike rolls
//!   (REH §4.9); [`ResidentialHost::strike_checks`] receives it so the host
//!   draws in the original's order. No RNG state is saved (foundations §6.2).
//! - **Time slicer** (§5.2): the 5 ms CPU-cycle budget is replaced by a
//!   deterministic work budget, [`ResidentialSim::work_budget`] (default
//!   [`DEFAULT_WORK_BUDGET`] = `u32::MAX`, effectively unbounded). One work
//!   unit is one distance pass (states 1–3) or one tract (state 4). Each
//!   call still completes at most one state, and an interrupted state resumes
//!   where it stopped (the iterator keeps its position; states 1–3 re-run
//!   their passes, which is idempotent), so the results equal those of an
//!   uninterrupted pass.
//! - **Float arithmetic** follows foundations §18 (24-bit x87): every add,
//!   subtract, multiply and divide is one `f32` rounding, in the order the
//!   spec gives. An integer operand (a population, a histogram count, a
//!   pollution byte) enters that one operation unrounded — computed in `f64`
//!   and rounded once ([`crate::x87::mul_i64_f32`], [`crate::x87::u32_to_float`]).
//!   Every `trunc` is the CRT `_ftol` ([`crate::x87::ftol`]). The per-mille
//!   history uses the 1.5·2²³ idiom ([`crate::x87::rhe`], §5.7).
//! - **`min(cap, v)`** (§5.5 step 4's `min(200, …)`, REH §4.5's grade-grid
//!   `min(1.0, …)`) is the original's `a < b ? a : b` with the cap first, so
//!   a NaN operand is stored rather than capped (REH §4.5 *[inferred]*).
//! - **Uninitialised tuning** (§7.1, §7.2): the newcomer HQ bonuses have no
//!   code default and several simulator values are set only when the exemplar
//!   exists. FreeCity4 uses `0.0` for the HQ bonuses (residential.md §7.1
//!   *[choice]*, which is normative over REH §4.13's "use the shipped
//!   values") and the "this branch" defaults with empty chunk lists for the
//!   rest (§7.2 *[choice]*).
//! - **Curve patterns** (§7.1, §7.3): the three pollution curves are
//!   rebuilt from their code default only when the exemplar exists, then
//!   loaded by key ("default" with the exemplar, "keep" without; an empty
//!   curve evaluates to 0.0). The two per-call curves (life expectancy,
//!   workforce) are fresh per call with the "default" pattern, and a missing
//!   exemplar makes slot 29/34 return 0 and slot 30/31 write 0.0.
//! - **Pollution average** (§5.5 step 1): the `S × S` cell values (each the
//!   pollution slot's 16-bit result) are summed as `u32` and divided by
//!   `S · S` as an unsigned 32-bit division; only the low byte is kept
//!   *[quirk]*. With `S ≤ 0` the average is 0 ([choice]: no cells, no
//!   division).
//! - **Missing grids**: a grid whose reference did not resolve reads 0 and
//!   ignores writes ([`TractGrid`]'s out-of-range rule); `Init` creates every
//!   absent grid of §2, and for the two coverage vectors it creates all
//!   entries when the first is null (§4.1 step 8).
//! - **`GetGlobalEQ` / `GetGlobalHQ`** (REH §4.8) accumulate over the EQ
//!   grid's tracts x outer, z inner ([choice]: the spec gives the per-tract
//!   order but not the traversal order, and float addition is not
//!   associative).
//! - **Slots with undefined returns** (§10.1): 19, 28, 47, 49 return true;
//!   21/22 return the `max` pointer; 23–25 return 1; the AL-only slots 29,
//!   34, 42 zero-extend into EAX.
//! - **`GetCellEQByWealth` / `GetCellHQByWealth`** (slots 37/38) take the
//!   wealth byte decremented as `u8`, so wealth 0 or > 3 reads out of bounds
//!   in the original *[quirk]*; FreeCity4 returns 0.0 ([choice]).
//! - **`Read`** (§9.2) never rejects a version, always returns true and
//!   ignores stream errors: parsing stops at the first failure and keeps
//!   what was read. A list count of 0 or ≥ [`MAX_LIST_COUNT`] reads no
//!   references, and null references are appended (as the original's list
//!   reader does), which the core records as a `None` slot.
//! - **Histograms** (§9.3 *[choice]*): zeroed at construction, so a save
//!   taken during state 4 gives that month's age-group history zeros rather
//!   than uninitialised memory. The loaded running population is kept, so
//!   the tracts already visited are counted twice in that month's population
//!   *[quirk]*.
//! - **Left out**: the dirty flag has no reader (§13 question 5) but is
//!   saved and set as specified; the two unused floats are carried through
//!   the record untouched (§3); `ToggleTractTracking`, `TrackTracts` and
//!   `OutputTractInfo` are no-ops (§10.1 slots 59, 62, 63).
//! - **Day-level order** (§4.3, foundations §13.5): registration order is
//!   the *[choice]* — the original's order is heap-address order and is not
//!   stable between sessions, so there is nothing to match. Note the
//!   same-pass couplings this leaves open: state 1 reads the BDS building
//!   map, and state 4 reads the growth developers' population maps and the
//!   pollution **source** grids, each as they stand when the residential day
//!   step runs. If a differential ever shows a value depending on residential
//!   running before or after one of those writers, that is a *derived*
//!   requirement for foundations §13.4's rank table, not something to fix by
//!   reordering registrations.
//! - **Threading** (foundations §19.3 rule 1): this simulator's state is a
//!   `RefCell`, as every other subsystem core's is. §19 requires `Sync` only
//!   for state the *clock* exposes to the diagnostic threads, which is
//!   `Simulator`'s own (a `Mutex` since 6f59cb2); no subsystem core's state
//!   is published, and §19.3 rule 2 prefers that it stay that way. So
//!   residential is **main-thread-only**: the host must not read its state
//!   from the watchdog or autotest thread. If a future heartbeat needs a
//!   residential value (for example the population), publish it as an
//!   atomic from the main thread rather than making the state `Sync`.
//!
//! Host callbacks must not call back into the residential simulator.

mod geometry;
mod simulator;
mod tuning;

pub use geometry::{copy_proximity, distance_pass, distance_to_convergence, seed_proximity, FAR, MAX_PASSES};
pub use simulator::{
    query_interface, Grids, ResidentialRng, ResidentialSim, ResidentialState, SlotReturn, SlotRow, SLOTS,
};
pub use tuning::{
    id, le_curve, new_ages_by_wealth, pct, shipped_tuning, workforce_curve, PopulationTuning, SimulatorTuning,
    AGE_GROUPS, TUNING_KEY,
};

pub use crate::grid::{GridProvider, ScanIterator, TractGrid};
pub use crate::rng::RzRandom;

use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::stream::RefValue;

/// Class ID (`GetGZCLSID`, §1).
pub const CLSID_RESIDENTIAL_SIMULATOR: u32 = 0x4990_C013;
/// Interface IDs answered by `QueryInterface` (§1).
pub const IID_UNKNOWN: u32 = 0x0000_0001;
pub const IID_RESIDENTIAL_SIMULATOR: u32 = 0x077A_C1EE;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// IIDs used when reading references (§9.1, foundations §2.7).
pub const IID_SIM_GRID: u32 = 0xC9BD_939A;
pub const IID_DEPARTMENT_BUDGET: u32 = 0x223E_2B67;
pub const IID_OCCUPANT: u32 = 0x26BB_767E;
/// `cISC4BuildingOccupant` (§4.3).
pub const IID_BUILDING_OCCUPANT: u32 = 0x87DD_A3A9;

/// City component slot (§1, foundations §3.6) and the city's getter slot.
pub const CITY_COMPONENT: u32 = 36;
pub const CITY_GETTER_SLOT: u32 = 68;

/// Agent registration (§4.3): levels 0, 10, 6, 4 in that order, flags 2.
pub const AGENT_NAME: &str = "cSC4ResidentialSimulator";
pub const AGENT_FLAGS: u32 = 2;
pub const AGENT_LEVELS: [u32; 4] = [0, 10, 6, 4];

/// The daily state machine's budget in milliseconds (§4.3); replaced by
/// [`DEFAULT_WORK_BUDGET`] work units ([choice], §5.2).
pub const DAILY_TIME_BUDGET_MS: u32 = 5;

/// [choice] The default work budget that replaces the 5 ms CPU-cycle slicer
/// (§5.2): effectively unbounded, so one call runs its state to the end.
pub const DEFAULT_WORK_BUDGET: u32 = u32::MAX;

/// A list count at or above this reads no references (§9.2).
pub const MAX_LIST_COUNT: u32 = 10_000_000;

/// The `GetMaxEQ` / `GetMaxHQ` value (slots 50, 51) and the EQ/HQ blend cap.
pub const MAX_QUOTIENT: i32 = 200;

/// Messages (§4.3).
pub mod msg {
    pub const SIM_BEGIN: u32 = 0x6695_6810;
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const SIM_IDLE: u32 = 0x6695_681A;
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    /// The two subscriptions of `Init` step 2, in order.
    pub const SUBSCRIPTIONS: [u32; 2] = [OCCUPANT_INSERTED, OCCUPANT_REMOVED];
    /// Sent (§5.7, §8): the history warehouse's class ID.
    pub const RECORD_HISTORY: u32 = 0x89EF_A536;
    /// Posted by the education/health half (REH §5).
    pub const SCHOOL_STRIKE: u32 = 0x077A_CCD0;
    pub const HEALTH_STRIKE: u32 = 0x077A_CCD2;
}

/// The occupant type of an insert / remove message (§4.3).
pub const OCCUPANT_TYPE_BUILDING: u32 = 0x2781_28A0;

/// History series (§8).
pub mod history {
    /// The population-weighted average life expectancy (§5.6).
    pub const AVERAGE_LIFE_EXPECTANCY: u32 = 0x2A5E_7009;
    /// Age-group population, `+ g` for g = 0..8 (§5.6).
    pub const AGE_GROUP_POPULATION: u32 = 0xCA54_5CAB;
    /// Age-group average EQ, `+ g` for g = 0..8 (§5.6).
    pub const AGE_GROUP_EQ: u32 = 0xCA54_5CCA;
    pub const GLOBAL_EQ: u32 = 0xAA1A_2864;
    pub const GLOBAL_HQ: u32 = 0xAA1A_2866;
    pub const POPULATION: u32 = 0xAA1A_2CCA;
    /// School-covered per mille of population (§5.7).
    pub const SCHOOL_COVERAGE: u32 = 0x4A8A_3FE5;
    /// Hospital-covered per mille (§5.7).
    pub const HOSPITAL_COVERAGE: u32 = 0x4A8A_3FE2;
    /// The nine age groups (§5.6).
    pub const AGE_GROUPS: usize = 9;
    /// The queue length set for every series at `Init` step 5.
    pub const QUEUE_LENGTH: u32 = 2;
}

/// The daily state machine's states (§5.1).
pub mod state {
    pub const STRIKES_AND_LISTS: u32 = 0;
    pub const PROXIMITY_LOW: u32 = 1;
    pub const PROXIMITY_MEDIUM: u32 = 2;
    pub const PROXIMITY_HIGH: u32 = 3;
    pub const POPULATION: u32 = 4;
    pub const SCHOOL_COVERAGE: u32 = 5;
    pub const SCHOOL_GRADING: u32 = 6;
    pub const HOSPITAL_COVERAGE: u32 = 7;
    pub const HOSPITAL_GRADING: u32 = 8;
    pub const HISTORY: u32 = 9;
    pub const IDLE: u32 = 10;
    pub const HISTOGRAM_RESET: u32 = 11;
    pub const POST_POPULATION: u32 = 12;
    pub const SEED_LOW: u32 = 13;
    pub const SEED_MEDIUM: u32 = 14;
    pub const SEED_HIGH: u32 = 15;
    /// The cycle order of §5.1.
    pub const ORDER: [u32; 16] = [0, 13, 1, 14, 2, 15, 3, 11, 4, 12, 5, 6, 7, 8, 9, 10];
}

/// Wealth levels (§2): R$ = 1, R$$ = 2, R$$$ = 3, array indices 0..2.
pub mod wealth {
    pub const LOW: usize = 0;
    pub const MEDIUM: usize = 1;
    pub const HIGH: usize = 2;
    pub const COUNT: usize = 3;
    /// The wealth numbers the interface uses.
    pub const NUMBERS: [u32; COUNT] = [1, 2, 3];
}

/// Grid instance IDs (§2).
pub mod instance {
    pub const PROXIMITY: [u32; 3] = [0x49D5_BC96, 0x49D5_BCA6, 0x49D5_BCB7];
    pub const POPULATION: [u32; 3] = [0x69D5_C2A0, 0x69D5_C2A1, 0x69D5_C2A2];
    pub const AGE: u32 = 0x69D5_C303;
    pub const SCHOOL_COVERAGE: [u32; 7] = [
        0xAA8E_1DC1,
        0x8A8D_F4B6,
        0xCA8E_449E,
        0xAA8D_F4BD,
        0x8A8D_F4C4,
        0xAA8D_F4D4,
        0x69D5_C30D,
    ];
    pub const HOSPITAL_COVERAGE: [u32; 3] = [0xAA8E_1DE4, 0x8A8D_F6FC, 0x69D5_C317];
    pub const SCHOOL_GRADE: u32 = 0x69D5_C324;
    pub const HOSPITAL_GRADE: u32 = 0x69D5_C330;
    pub const EQ: [u32; 3] = [0x69D5_C2D0, 0x69D5_C2D1, 0x69D5_C2D2];
    pub const HQ: [u32; 3] = [0x69D5_C2E0, 0x69D5_C2E1, 0x69D5_C2E2];
    /// `GetAverageEQGrid` / `GetAverageHQGrid` (§6.2).
    pub const AVERAGE_EQ: u32 = 0x69D5_C351;
    pub const AVERAGE_HQ: u32 = 0x69D5_C360;
}

/// The class IDs of the grids of §2 (§11.3: a replacement must hand out game
/// grids of these element types).
pub mod grid_class {
    pub const U8: u32 = 0x49B9_E602;
    pub const U16: u32 = 0x49B9_E604;
    /// The growth developers' population maps (REH §4.7, growth-developer §1).
    pub const I16: u32 = 0x49B9_E605;
    pub const F32: u32 = 0x49B9_E60A;
    /// The temporary proximity grid is constructed with this ID (§2); it is
    /// never saved.
    pub const TEMPORARY: u32 = 0x49B9_E600;
}

/// The proximity grid's tract size (§3): one cell.
pub const PROXIMITY_TRACT_SIZE: i32 = 1;
/// The constructed population and EQ tract sizes (§3), replaced at
/// new-city `Init` by the R$ developer's population-map tract size.
pub const DEFAULT_TRACT_SIZE: i32 = 4;

/// Growth developer types (§4.1 step 1), in wealth order.
pub const DEVELOPERS: [u32; 3] = [0x1010, 0x1020, 0x1030];
/// The demand node held but never read (§4.1 step 1): `GetDemand(0x1000, 0)`.
pub const DEMAND_NODE: (u32, u32) = (0x1000, 0);

/// Budget departments (§4.1 step 9, REH §4.2).
pub mod department {
    pub const GROUP: u32 = 0x6A35_7B7F;
    pub const HEALTH: u32 = 0x0918_8F42;
    pub const HEALTH_SECONDARY: u32 = 0xAA53_8CB3;
    pub const EDUCATION: u32 = 0x0918_8F4C;
    pub const EDUCATION_SECONDARY: u32 = 0x4A53_8CC6;
    pub const NAME_GROUP: u32 = 0x6A3F_F01C;
    pub const HEALTH_NAME: u32 = 0xCA49_3A56;
    pub const HEALTH_SECONDARY_NAME: u32 = 0xCA53_8D98;
    pub const EDUCATION_NAME: u32 = 0x6A49_3A66;
    pub const EDUCATION_SECONDARY_NAME: u32 = 0x8A53_8DAC;
}

/// Pollution type indices passed to `GetPollutionValue` (§5.5 step 1).
pub mod pollution_kind {
    pub const AIR: u32 = 0;
    pub const WATER: u32 = 1;
    pub const GARBAGE: u32 = 2;
}

/// The building map key of a residential wealth level (§5.3 step 1):
/// `GetBuildingMapKey(purpose 1, w)` = `(purpose << 2) + w + 1`.
pub const BUILDING_PURPOSE_RESIDENTIAL: u32 = 1;

/// The death's-door visual effect's name (§5.5 step 2).
pub const BUZZARDS_EFFECT: &str = "buzzards";

/// The occupant auto-budget bool property (§6.6, REH §2).
pub const PROP_AUTO_BUDGET: u32 = 0x6C05_FC4D;

/// [`PROP_AUTO_BUDGET`], as a function so the simulator reads it through one
/// name.
pub fn prop_auto_budget() -> u32 {
    PROP_AUTO_BUDGET
}

/// The number of age histogram buckets (§3, §5.4).
pub const AGE_BUCKETS: usize = 20;

/// A message as `DoMessage` sees it (§4.3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResidentialMessage {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    /// `GetVoid1()`: the occupant of an insert / remove message.
    pub void1: Option<u64>,
}

impl ResidentialMessage {
    pub fn of_type(msg_type: u32) -> ResidentialMessage {
        ResidentialMessage {
            msg_type,
            ..ResidentialMessage::default()
        }
    }
}

/// The shared random stream (§3): the buzzards rolls (§5.5) and the monthly
/// strike rolls (REH §4.9) draw from one `cRZRandom`. The generator itself is
/// [`ResidentialRng`] (re-exported from `simulator`).
pub trait ResidentialRandom {
    /// `RandomDoubleRangeUniform(a, b)` (foundations §6.1).
    fn range_f64(&mut self, a: f64, b: f64) -> f64;
}

/// One education or health department's record: the saved scalars of §9.1
/// plus the two departments it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepartmentBlock {
    /// `0xE918946F` / `0xC91894A8`; loaded but never read (§7.2).
    pub f0: f32,
    pub max_funding: f32,
    pub strike_efficiency: f32,
    /// Compared in **days** (REH §4.9 *[quirk]*).
    pub strike_duration: i32,
    /// Compared with the staff ratio (REH §4.9 *[quirk]*).
    pub min_grade: f32,
    pub strike_flag: u8,
    pub strike_start: i32,
    pub init_funding: f32,
    /// The "a 100.0 field" of §9.1.
    pub hundred: f32,
    /// The strike message type (§7.2): `0x077ACCD2` health, `0x077ACCD0`
    /// education.
    pub id: u32,
    /// The primary and secondary department budgets (host tokens).
    pub department1: Option<u64>,
    pub department2: Option<u64>,
}

impl DepartmentBlock {
    /// The code defaults of §7.2 for one service.
    pub fn code_defaults(id: u32) -> DepartmentBlock {
        DepartmentBlock {
            f0: 0.6,
            max_funding: 1.2,
            strike_efficiency: 0.2,
            strike_duration: 10,
            min_grade: 0.9,
            strike_flag: 0,
            strike_start: 0,
            init_funding: 0.25,
            hundred: 100.0,
            id,
            department1: None,
            department2: None,
        }
    }
}

/// Everything the residential simulator reads from, or does to, the rest of
/// the game (§11.1). Objects are host tokens. Re-entrancy: callbacks must
/// not call back into the residential simulator.
pub trait ResidentialHost: GridProvider<u8> + GridProvider<u16> + GridProvider<f32> {
    // ---- lifecycle (§4.1, §4.2) ----
    /// `Init` step 1: take and `AddRef` the city, the building development
    /// simulator and the occupant manager, each only if not already held.
    /// False if any is missing.
    fn acquire_services(&self) -> bool;
    /// `Init` step 1: the three growth developers through
    /// `BDS.GetDeveloper(0x1010 / 0x1020 / 0x1030)`. All three must be
    /// present.
    fn acquire_developers(&self) -> bool;
    /// `Init` step 1: the demand node `GetDemand(0x1000, 0)` from the demand
    /// simulator. **`Init` fails without it**; the node is only held, never
    /// read.
    fn acquire_demand_node(&self) -> bool;
    /// `Init` step 2: take the message server. False when there is none.
    fn acquire_message_server(&self) -> bool;
    /// `Init` step 3: take the budget simulator.
    fn acquire_budget_simulator(&self) -> bool;
    /// `Init` step 4: whether a simulator exists. `Init` fails without one.
    fn has_simulator(&self) -> bool;
    /// `Init` step 4: `AddAgent(messageTarget, level, "cSC4ResidentialSimulator", 2)`
    /// for each level of [`AGENT_LEVELS`], in that order.
    fn add_agent(&self, level: u32, name: &str, flags: u32);
    /// `Shutdown`: `RemoveAgent(target)` on the simulator.
    fn remove_agent(&self);
    /// Message server slots 5 / 6 with the message-target sub-object.
    fn subscribe(&self, msg_type: u32);
    fn unsubscribe(&self, msg_type: u32);
    /// `Shutdown`: release every service `Init` took, in the reverse order,
    /// and hand every occupant list to the occupant manager (slot 48).
    fn release_services(&self);
    /// `Init` step 5: `SetHistoryDataQueueLength(id, n)`.
    fn set_history_queue_length(&self, series: u32, length: u32);
    /// `Init` step 9: `CreateDepartmentBudget(id, 0x6A357B7F)`, `AddRef`,
    /// `SetDepartmentName(0x6A3FF01C, name)`,
    /// `SetMaxAllowedFundingPercentage(max)` and
    /// `SetFundingPercentage(initial, 0)`. The secondary departments also
    /// create line item 1 (local) and clear its display flag 1 (REH §4.2).
    fn create_department(&self, id: u32, name: u32, max_funding: f32, init_funding: f32) -> Option<u64>;
    /// `Shutdown` / a replaced department: `Release`.
    fn release_department(&self, department: u64);
    /// `SetInstanceID` on a grid created for, or loaded by, the simulator
    /// (§2). Grid references read from a save also get their instance ID set.
    fn set_grid_instance_u8(&self, grid: &Rc<dyn TractGrid<u8>>, instance: u32);
    fn set_grid_instance_u16(&self, grid: &Rc<dyn TractGrid<u16>>, instance: u32);
    fn set_grid_instance_f32(&self, grid: &Rc<dyn TractGrid<f32>>, instance: u32);
    /// `Shutdown` on a grid before the simulator drops it (§4.2).
    fn shutdown_grid_u8(&self, grid: &Rc<dyn TractGrid<u8>>);
    fn shutdown_grid_u16(&self, grid: &Rc<dyn TractGrid<u16>>);
    fn shutdown_grid_f32(&self, grid: &Rc<dyn TractGrid<f32>>);
    /// `AddRef` on a grid handed to a caller (slots 23–25): the new
    /// reference count (§6.3).
    fn grid_add_ref_u16(&self, grid: &Rc<dyn TractGrid<u16>>) -> u32;
    fn grid_add_ref_f32(&self, grid: &Rc<dyn TractGrid<f32>>) -> u32;
    /// `AddRef` / `Release` on a held occupant (§4.1, REH §4.1).
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);

    // ---- resources and tuning (§7) ----
    /// The residential tuning exemplar (type `0x6534284A`, group
    /// `0xE7E2C2DB`, instance `0x69189C84`), if it loads. Fetched on every
    /// call: §7.1 reads it at `Init` and at the first state-4 call of each
    /// cycle, and §7.3 fetches it afresh for every life-expectancy and
    /// workforce query.
    fn tuning(&self) -> Option<Rc<PropertyHolder>>;

    // ---- the monthly pass's inputs (§5.3, §5.5) ----
    /// BDS slot 41 `GetBuildingMap()`: the `uint8` grid of building keys.
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>>;
    /// BDS slot 42 `GetBuildingMapKey(purpose, wealth)` (AL).
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8;
    /// Growth developer slot 15 `GetPopulationMap()` of developer `dev_type`
    /// (`0x1010` / `0x1020` / `0x1030`): a `cISC4SimGrid<int16>` whose 16
    /// bits this simulator reads as unsigned (§5.5 step 3).
    fn developer_population_map(&self, dev_type: u32) -> Option<Rc<dyn TractGrid<i16>>>;
    /// Pollution slot 6 `GetPollutionValue(type, x, z)` (**AX**, the source
    /// grids) at a cell.
    fn pollution_value(&self, kind: u32, cell_x: i32, cell_z: i32) -> u16;
    /// Simulator slot 6 `GetSimDate(year*, month*, day*, dayOfYear*,
    /// weekDay*)`: only the month is used (§5.5 step 5).
    fn sim_month(&self) -> u32;
    /// `cISTETerrain` slot 13 `GetAltitude(x, z)` (ST0) at a world position.
    fn altitude(&self, x: f32, z: f32) -> f32;

    // ---- visual effects (§5.5 step 2) ----
    /// Effects manager slot 7 `CreateVisualEffect(name, effect*&)`: the
    /// created effect, or `None` when it returns no effect (the out value is
    /// what the original tests).
    fn create_visual_effect(&self, name: &str) -> Option<u64>;
    /// `cISC4VisualEffect` slot 8 `SetSourceTransform` with an identity
    /// rotation, this translation and scale 1.
    fn set_effect_transform(&self, effect: u64, translation: [f32; 3]);
    /// Slot 3 `Start(0)`, then slot 2 `Release`.
    fn start_and_release_effect(&self, effect: u64);

    // ---- history (§5.6, §5.7, §8) ----
    /// `RecordHistoryData(id, value)`: create a `cRZMessage2Standard` of type
    /// [`msg::RECORD_HISTORY`] with data1 = `series` and data2 = `value`,
    /// **send** it synchronously and release it. Nothing happens without a
    /// message server.
    fn record_history(&self, series: u32, value: i32);

    // ---- occupants (§6.6, REH §4.1) ----
    /// `QueryInterface(void1, 0x87DDA3A9)` for `cISC4BuildingOccupant`
    /// (§4.3): the building occupant, if it is one.
    fn occupant_as_building(&self, occupant: u64) -> Option<u64>;
    /// A bool property of the occupant's property holder (occupant slot 6);
    /// `None` when absent.
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool>;
    /// Sets a bool property through the occupant's property holder; the
    /// property helper's result (slot 57's EAX).
    fn set_occupant_bool_property(&self, occupant: u64, id: u32, value: bool) -> bool;
    /// How a save refers to an occupant or a department (§9.1).
    fn object_ref(&self, token: u64) -> RefValue;
    /// The token of a reference read with `iid`; the reference the stream
    /// handed over is transferred to the simulator, and when this returns
    /// `None` the host drops it.
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64>;

    // ---- the education and health half (REH) ----
    /// `Init` step 7 and state 0: rebuild the two ordinance-effect lists
    /// (REH §4.3). State 0 clears both first; `Init` does not, but the lists
    /// are empty then.
    fn refresh_ordinance_effects(&self);
    /// Π of the decay multipliers in the education list and in the health
    /// list, each starting at 1.0 for an empty list and multiplying in list
    /// order (REH §4.3, §7.1). Read when the population tuning is read.
    fn ordinance_decay_products(&self) -> (f32, f32);
    /// State 0: the monthly strike check for schools, then hospitals
    /// (REH §4.9). **`rng` is the class's shared `cRZRandom`**: each eligible
    /// non-striking building takes exactly one `RandomDoubleRangeUniform(0.0,
    /// 100.0)` draw from it, in group-list order, even when the chance is 0.
    fn strike_checks(&self, rng: &mut dyn ResidentialRandom);
    /// State 0: move the pending school, then pending hospital, then pending
    /// health-affecting buildings into their group lists (REH §4.1).
    fn distribute_pending_buildings(&self, lists: &mut OccupantLists);
    /// §4.3 `DoBuildingChanged(building, inserted)`: recognise the building,
    /// register it with the budget and append it to (added) or erase it from
    /// (removed) the pending list in `lists` (REH §4.1). The caller sets the
    /// dirty flag afterwards.
    fn building_changed(&self, building: u64, inserted: bool, lists: &mut OccupantLists);
    /// State 5: `UpdateCoverageMap` for the school groups 0..6 in order
    /// (REH §4.4). The simulator zeroes the school-covered counter first and
    /// stores the returned served count for state 9 (§5.7).
    fn update_school_coverage(&self) -> i32;
    /// State 6: for each school group and each building in list order the
    /// auto-budget step, then `ComputeOccupantGrade` with the EQ grids
    /// (REH §4.5, §4.6, §4.10). The grade grid is zeroed first.
    fn grade_schools(&self);
    /// State 7: `UpdateCoverageMap` for the hospital groups 0..2 (REH §4.4);
    /// returns the served count for state 9.
    fn update_hospital_coverage(&self) -> i32;
    /// State 8: as [`ResidentialHost::grade_schools`] for hospitals, with the
    /// HQ grids.
    fn grade_hospitals(&self);
    /// Slots 6–13: the strike flags, `EndStrike`, the strike chances and the
    /// system ratings (REH §4.9, §4.11).
    fn school_is_on_strike(&self) -> bool;
    fn health_is_on_strike(&self) -> bool;
    fn end_school_strike(&self) -> bool;
    fn end_health_strike(&self) -> bool;
    fn chance_of_school_strike(&self) -> f32;
    fn chance_of_health_strike(&self) -> f32;
    fn school_system_rating(&self) -> f32;
    fn health_system_rating(&self) -> f32;
    /// Slots 14 / 15: the system totals; the number of buildings counted
    /// (REH §4.11).
    fn school_system_totals(&self, groups: &[u32]) -> u32;
    fn hospital_system_totals(&self, groups: &[u32]) -> u32;
    /// Slots 19 / 20: the occupant's staff and vehicle funding percentages
    /// (REH §4.2). Slot 19 also sets the dirty flag; a missing budget item
    /// ends the call with it set, a null department without.
    fn set_occupant_funding(&self, occupant: u64, staff: f32, vehicle: f32, is_school: bool) -> bool;
    /// Slot 20: `(staff, vehicle)`; `(1.0, 1.0)` without a staff item.
    fn occupant_funding(&self, occupant: u64, is_school: bool) -> (f32, f32);
    /// Slots 26 / 27: the query-tool record (REH §4.11).
    fn school_query_data(&self, occupant: u64) -> Option<QueryData>;
    fn hospital_query_data(&self, occupant: u64) -> Option<QueryData>;
    /// Slot 28 `EstimateCurrentOccupantCapacity`: `(capacity, staff)`
    /// (REH §4.11).
    fn estimate_occupant_capacity(&self, occupant: u64) -> (u32, u32);
    /// Slot 45 `GetOccupantCoverage`: `(distance at level, radius)`
    /// (REH §4.11).
    fn occupant_coverage(&self, occupant: u64, level: f32) -> Option<(f32, f32)>;
    /// Slots 47 / 49 `GetSchoolBuildings` / `GetHospitalBuildings`: appends
    /// the group lists selected by `groups` (all when `groups` is `None`),
    /// then always the pending list, `AddRef`ing each (REH §4.1).
    fn school_buildings(&self, groups: Option<&[u32]>, out: &mut Vec<u64>);
    fn hospital_buildings(&self, groups: Option<&[u32]>, out: &mut Vec<u64>);
    /// Slot 58 `EstimateIdealFunding` (REH §4.10).
    fn estimate_ideal_funding(&self, occupant: u64) -> Option<f32>;
    /// Slot 61 `GetHealthAffectingBuildings`: appends the health-affecting
    /// list and the other list, each occupant `AddRef`ed.
    fn health_affecting_buildings(&self, out: &mut Vec<u64>);
}

/// The occupant lists of §3 / §9.1, in save order. All are host tokens; the
/// simulator holds one reference per entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OccupantLists {
    /// Buildings added since the last state 0 (REH §4.1).
    pub pending_school: Vec<Option<u64>>,
    pub pending_hospital: Vec<Option<u64>>,
    /// The "other list" of §9.1.
    pub other: Vec<Option<u64>>,
    /// The seven school groups, in coverage-grid order (REH §4.1).
    pub school_groups: [Vec<Option<u64>>; 7],
    /// The three hospital groups (REH §4.1).
    pub hospital_groups: [Vec<Option<u64>>; 3],
    /// Health-affecting buildings (REH §4.1 step 3).
    pub health_affecting: Vec<Option<u64>>,
}

impl OccupantLists {
    /// Every list, in the record's order (§9.1).
    pub fn in_save_order(&mut self) -> Vec<&mut Vec<Option<u64>>> {
        let mut v: Vec<&mut Vec<Option<u64>>> = Vec::with_capacity(14);
        v.push(&mut self.pending_school);
        v.push(&mut self.pending_hospital);
        v.push(&mut self.other);
        for l in self.school_groups.iter_mut() {
            v.push(l);
        }
        for l in self.hospital_groups.iter_mut() {
            v.push(l);
        }
        v.push(&mut self.health_affecting);
        v
    }

    /// The same, immutably.
    pub fn save_order(&self) -> Vec<&Vec<Option<u64>>> {
        let mut v: Vec<&Vec<Option<u64>>> = Vec::with_capacity(14);
        v.push(&self.pending_school);
        v.push(&self.pending_hospital);
        v.push(&self.other);
        for l in &self.school_groups {
            v.push(l);
        }
        for l in &self.hospital_groups {
            v.push(l);
        }
        v.push(&self.health_affecting);
        v
    }
}

/// The query-tool record of REH §4.11 (slots 26 / 27), the fields the spec
/// gives. The leading `cRZString` is not filled by those methods.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QueryData {
    pub capacity: u32,
    pub effective_capacity: u32,
    /// `capacity / divisor` (floor).
    pub capacity_per_staff: u32,
    /// `ceil(effCap / divisor)` for schools, `floor` for hospitals *[quirk]*.
    pub effective_per_staff: u32,
    /// Served population; `0xFFFFFFFF` reads as 0.
    pub served: u32,
    pub grade: f32,
    pub staff_funding: f32,
    pub staff_cost: i64,
    pub vehicle_funding: f32,
    pub vehicle_cost: i64,
}
