// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4ResidentialSimulator`: the state (§3), `AddRef` / `Release` (§1),
//! the lifecycle (§4), the agent cadence and `DoMessage` (§4.3), the daily
//! state machine (§5.1–§5.2), the proximity maps (§5.3), the histogram
//! (§5.4), `DoPopulationChange` (§5.5), `PostPopulationChange` (§5.6), the
//! history of state 9 (§5.7), the queries (§6, REH §4.8), the strike flags
//! and building lists the frame owns (REH §4.1, §4.9) and the interface slots
//! (§10).
//!
//! No `RefCell` borrow is held across a host callback (foundations §19.3
//! rule 4): every grid is an `Rc` cloned out of the state before the host is
//! called, the education and health half (`education_health.rs`) reads
//! what it needs before each host call, and the
//! accumulators are written back with a short borrow at the end of a tract.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::{GridProvider, ScanIterator, TractGrid, MAX_TRACTS};
use crate::rng::{RzRandom, SeedSource, CLOCK_SEED};
use crate::x87::{ftol32, mul_i64_f32, rhe, u32_to_float};

use super::education_health::{OrdinanceEffect, Service};
use super::geometry::{copy_proximity, distance_pass, distance_to_convergence, seed_proximity};
use super::tuning::{le_curve, new_ages_by_wealth, pct, workforce_curve, PopulationTuning, SimulatorTuning};
use super::{
    group, history, instance, msg, state as st, wealth, CreatedEffect, DepartmentBlock, OccupantLists, ResidentialHost,
    ResidentialMessage, ResidentialRandom, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, AGE_BUCKETS,
    BUILDING_PURPOSE_RESIDENTIAL, BUZZARDS_EFFECT, DEFAULT_TRACT_SIZE, DEFAULT_WORK_BUDGET, DEVELOPERS, MAX_QUOTIENT,
    OCCUPANT_TYPE_BUILDING, PROP_AUTO_BUDGET, PROXIMITY_TRACT_SIZE,
};

/// The grids of §2. The temporary proximity grid is created in states 13–15
/// and freed in state 3 (§5.3); it is never saved.
#[derive(Clone, Default)]
pub struct Grids {
    pub proximity: [Option<Rc<dyn TractGrid<u8>>>; 3],
    pub population: [Option<Rc<dyn TractGrid<u16>>>; 3],
    pub age: Option<Rc<dyn TractGrid<u8>>>,
    pub school_coverage: [Option<Rc<dyn TractGrid<u8>>>; 7],
    pub hospital_coverage: [Option<Rc<dyn TractGrid<u8>>>; 3],
    pub school_grade: Option<Rc<dyn TractGrid<f32>>>,
    pub hospital_grade: Option<Rc<dyn TractGrid<f32>>>,
    pub eq: [Option<Rc<dyn TractGrid<f32>>>; 3],
    pub hq: [Option<Rc<dyn TractGrid<f32>>>; 3],
    /// Exists only during states 13 → 3 (§2, §5.3); not saved.
    pub temporary: Option<Rc<dyn TractGrid<u8>>>,
}

/// The three EQ or HQ grids (R$, R$$, R$$$).
type QuotientGrids = [Option<Rc<dyn TractGrid<f32>>>; 3];
/// The three population grids.
type PopulationGrids = [Option<Rc<dyn TractGrid<u16>>>; 3];

/// Every field of §3 (persisted or not), the grids and the occupant lists.
#[derive(Clone)]
pub struct ResidentialState {
    pub initialized: bool,
    /// The new-city part of `Init` has run (§4.1 step 6); `Read` sets it.
    pub city_started: bool,
    /// The GZCOM reference count (§1): 0 at construction.
    pub ref_count: u32,
    /// `Release` destroyed the object (§1).
    pub destroyed: bool,
    /// A message server is held (§3 "services"; `Init` step 2 takes it and
    /// subscribes only when this is false; `Shutdown` releases it). Not saved.
    pub message_server_held: bool,
    /// The proximity tract size (§3): 1.
    pub proximity_size: i32,
    /// The population tract size P (§2). After a load only `Init` (for a grid
    /// it recreates) and `Write` read it; state 4's S is the R$ population
    /// grid's own size (EH14).
    pub population_size: i32,
    /// The EQ tract size E (§2).
    pub eq_size: i32,
    pub grids: Grids,
    /// The daily state machine (§5.1).
    pub state: u32,
    /// Set by funding and building changes and by an overrunning month;
    /// never read (§3 *[inferred]*). Saved as `dirty ∨ state ≠ 10`.
    pub dirty: bool,
    /// Σ population of the last completed pass (`GetPopulation`, slot 16).
    pub population: i32,
    /// The sum being built by the current pass (§5.5).
    pub running_population: i32,
    /// Σ LE × tract population of the current pass; reset by every `Init`.
    pub le_sum: u64,
    /// The age histogram (§5.4–§5.6); zeroed at construction (§9.3 [choice]).
    pub age_count: [i32; AGE_BUCKETS],
    pub age_eq_sum: [i32; AGE_BUCKETS],
    /// The coverage counters filled by the coverage states (REH §4.4), used
    /// by state 9 (§5.7).
    pub school_covered: i32,
    pub hospital_covered: i32,
    /// The two unused floats (§3): never written except by `Read`.
    pub unused_float1: f32,
    pub unused_float2: f32,
    /// The tract scan of state 4 (§5.5); reset by every `Init`.
    pub iterator: ScanIterator,
    /// The population tuning was loaded for this pass (§7.1).
    pub tuning_loaded: bool,
    pub population_tuning: PopulationTuning,
    pub simulator_tuning: SimulatorTuning,
    pub health: DepartmentBlock,
    pub education: DepartmentBlock,
    pub lists: OccupantLists,
    /// School / hospital global auto-budget (slots 52–55).
    pub school_auto_budget: bool,
    pub hospital_auto_budget: bool,
    /// The ordinance effect lists (REH §2, §4.3): rebuilt by `Init` step 7
    /// and every state 0; not saved.
    pub school_effects: Vec<OrdinanceEffect>,
    pub health_effects: Vec<OrdinanceEffect>,
    /// The grade totals (REH §2): reset by state 0 and accumulated by the
    /// EQ/HQ boost (REH §4.6); never read and not saved (RS13).
    pub school_grade_total: f32,
    pub hospital_grade_total: f32,
}

impl Default for ResidentialState {
    /// A constructed object (§3 "Initial").
    fn default() -> ResidentialState {
        ResidentialState {
            initialized: false,
            city_started: false,
            ref_count: 0,
            destroyed: false,
            message_server_held: false,
            proximity_size: PROXIMITY_TRACT_SIZE,
            population_size: DEFAULT_TRACT_SIZE,
            eq_size: DEFAULT_TRACT_SIZE,
            grids: Grids::default(),
            state: 0,
            dirty: true,
            population: 0,
            running_population: 0,
            le_sum: 0,
            age_count: [0; AGE_BUCKETS],
            age_eq_sum: [0; AGE_BUCKETS],
            school_covered: 0,
            hospital_covered: 0,
            unused_float1: 0.0,
            unused_float2: 0.0,
            iterator: ScanIterator::default(),
            tuning_loaded: false,
            population_tuning: PopulationTuning::default(),
            simulator_tuning: SimulatorTuning::default(),
            health: DepartmentBlock::code_defaults(msg::HEALTH_STRIKE),
            education: DepartmentBlock::code_defaults(msg::SCHOOL_STRIKE),
            lists: OccupantLists::default(),
            school_auto_budget: false,
            hospital_auto_budget: false,
            school_effects: Vec::new(),
            health_effects: Vec::new(),
            school_grade_total: 0.0,
            hospital_grade_total: 0.0,
        }
    }
}

/// How a compiled slot returns its result (§10.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotReturn {
    /// Only AL is meaningful (zero-extended into EAX).
    Al,
    /// EAX (a count, an integer or a pointer).
    Eax,
    /// ST0 (a float).
    St0,
    /// An `SC4Percentage` by value through a hidden first stack argument;
    /// EAX = that pointer.
    HiddenPtr,
    /// Nothing meaningful.
    None,
}

/// One row of the compiled slot table: (slot, method, callee-pop bytes,
/// return channel).
pub type SlotRow = (u32, &'static str, u32, SlotReturn);

/// `cISC4ResidentialSimulator` (§10.1): 60 interface slots + 4 class slots.
/// The pop and return channel are as the **replacement** provides them; the
/// "Replacement values" note of §10.1 governs the slots whose original return
/// is garbage (19, 20, 28, 47, 49 → true; 21/22 → the max pointer; 23–25 → 1;
/// 29, 34, 42 → the value zero-extended in EAX).
pub const SLOTS: [SlotRow; 64] = {
    use SlotReturn::*;
    [
        (0, "QueryInterface", 8, Al),
        (1, "AddRef", 0, Eax),
        (2, "Release", 0, Eax),
        (3, "Init", 0, Al),
        (4, "Shutdown", 0, Al),
        (5, "GetProximityMap", 4, Eax),
        (6, "SchoolIsOnStrike", 0, Al),
        (7, "HealthIsOnStrike", 0, Al),
        (8, "EndSchoolStrike", 0, Al),
        (9, "EndHealthStrike", 0, Al),
        (10, "ChanceOfSchoolStrike", 4, HiddenPtr),
        (11, "ChanceOfHealthStrike", 4, HiddenPtr),
        (12, "GetSchoolSystemRating", 4, HiddenPtr),
        (13, "GetHealthSystemRating", 4, HiddenPtr),
        (14, "GetSchoolSystemTotals", 8, Eax),
        (15, "GetHospitalSystemTotals", 8, Eax),
        (16, "GetPopulation", 0, Eax),
        (17, "GetTotalCityEducationUpkeepCost", 0, Eax),
        (18, "GetTotalCityHealthUpkeepCost", 0, Eax),
        (19, "SetOccupantFundingPercentages", 16, Al),
        (20, "GetOccupantFundingPercentages", 16, Al),
        (21, "GetAverageEQGrid", 12, Eax),
        (22, "GetAverageHQGrid", 12, Eax),
        (23, "GetEQGrids", 12, Eax),
        (24, "GetHQGrids", 12, Eax),
        (25, "GetPopulationGrids", 12, Eax),
        (26, "GetSchoolQueryData", 8, Al),
        (27, "GetHospitalQueryData", 8, Al),
        (28, "EstimateCurrentOccupantCapacity", 12, Al),
        (29, "GetCellLifeExpectancy", 8, Al),
        (30, "GetCellWorkforcePercent", 12, HiddenPtr),
        (31, "GetGlobalWorkforcePercent", 4, HiddenPtr),
        (32, "GetGlobalEQ", 0, St0),
        (33, "GetGlobalHQ", 0, St0),
        (34, "GetGlobalLE", 0, Al),
        (35, "GetCellEQ", 8, St0),
        (36, "GetCellHQ", 8, St0),
        (37, "GetCellEQByWealth", 12, St0),
        (38, "GetCellHQByWealth", 12, St0),
        (39, "GetSchoolAverageGradeMap", 0, Eax),
        (40, "GetHospitalAverageGradeMap", 0, Eax),
        (41, "GetAverageAgeMap", 0, Eax),
        (42, "GetAverageNewAgeByWealth", 4, Al),
        (43, "GetEQMinAndMaxCellCoords", 24, Al),
        (44, "GetHQMinAndMaxCellCoords", 24, Al),
        (45, "GetOccupantCoverage", 16, Al),
        (46, "GetSchoolBuildingCount", 0, Eax),
        (47, "GetSchoolBuildings", 8, Al),
        (48, "GetHospitalBuildingCount", 0, Eax),
        (49, "GetHospitalBuildings", 8, Al),
        (50, "GetMaxEQ", 0, Eax),
        (51, "GetMaxHQ", 0, Eax),
        (52, "GetGlobalAutoBudgetForSchools", 0, Al),
        (53, "SetGlobalAutoBudgetForSchools", 4, Al),
        (54, "GetGlobalAutoBudgetForHospitals", 0, Al),
        (55, "SetGlobalAutoBudgetForHospitals", 4, Al),
        (56, "GetAutoBudget", 4, Al),
        (57, "SetAutoBudget", 8, Eax),
        (58, "EstimateIdealFunding", 8, Al),
        (59, "ToggleTractTracking", 8, None),
        (60, "scalar deleting destructor", 4, Eax),
        (61, "GetHealthAffectingBuildings", 4, None),
        (62, "TrackTracts", 0, None),
        (63, "OutputTractInfo", 8, None),
    ]
};

/// EAX of slots 23–25 in a replacement (§6.3 *[choice]*).
pub const GRID_TRIPLE_RESULT: u32 = 1;

/// Whether `QueryInterface` answers `iid` (§1; unknown IIDs → false).
pub fn query_interface(iid: u32) -> bool {
    matches!(
        iid,
        super::IID_UNKNOWN | super::IID_RESIDENTIAL_SIMULATOR | super::IID_MESSAGE_TARGET2 | super::IID_SERIALIZABLE
    )
}

/// The histogram bucket of an age (§5.5 step 11): `(age − 20) ÷ 3`, signed
/// and truncated toward zero; a bucket that is ≥ 20 **read as unsigned**
/// becomes 19. So ages 18..22 → 0, 23..25 → 1, …, 74..76 → 18, and ages
/// 0..17 and ≥ 77 → 19 *[quirk]*.
pub fn age_bucket(age: i32) -> usize {
    let b = age.wrapping_sub(20) / 3;
    if (b as u32) >= AGE_BUCKETS as u32 {
        AGE_BUCKETS - 1
    } else {
        b as usize
    }
}

/// The class's shared `cRZRandom` (§3): buzzards (§5.5) and the monthly
/// strike rolls (REH §4.9) draw from one stream. RG6 (answered, no original
/// behaviour): every draw of the class is a double draw, which uses only the
/// generator's `mul` word (`+0xBC`).
pub struct ResidentialRng(pub RzRandom);

impl ResidentialRng {
    /// The original's clock-seeded generator (§3, foundations §6.2).
    pub fn new(seeds: &dyn SeedSource) -> ResidentialRng {
        ResidentialRng(RzRandom::new(CLOCK_SEED, seeds))
    }

    /// The session-seeded generator (foundations §6.3 *[choice]*): a member
    /// generator seeded from the wall clock uses the owning class's GZCLSID as
    /// its constant, so `derive_seed(session, 0x4990C013)`.
    pub fn from_session(session: u32, seeds: &dyn SeedSource) -> ResidentialRng {
        ResidentialRng(RzRandom::new(
            crate::seeding::seed(session, crate::seeding::keys::RESIDENTIAL),
            seeds,
        ))
    }
}

impl ResidentialRandom for ResidentialRng {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        self.0.range_f64(a, b)
    }
}

/// `min(cap, v)` as the original's `cap < v ? cap : v`: a NaN operand is
/// stored rather than capped (§5.5 step 4, REH §4.5 *[inferred]*).
pub(super) fn min_cap(cap: f32, v: f32) -> f32 {
    if cap < v {
        cap
    } else {
        v
    }
}

/// `int × float`: one `f32` rounding of the exact product (foundations §18.3
/// point 1: the integer operand enters unrounded).
pub(super) fn mul_i(i: i64, f: f32) -> f32 {
    mul_i64_f32(i, f)
}

/// `float(uint32 v) × f`: a non-negative `int32` enters exactly; a negative
/// one first has 2³² added, which rounds to 24 bits ([`u32_to_float`]).
pub(super) fn mul_u32(v: i32, f: f32) -> f32 {
    if v >= 0 {
        mul_i64_f32(i64::from(v), f)
    } else {
        (u32_to_float(v) * f64::from(f)) as f32
    }
}

/// `float + int` (`fiadd`): the integer enters unrounded, one rounding.
fn add_i(v: f32, i: i64) -> f32 {
    (f64::from(v) + i as f64) as f32
}

/// `float ÷ int`: one `f32` rounding (the integer divisor enters unrounded).
fn div_i(v: f32, d: i64) -> f32 {
    (f64::from(v) / d as f64) as f32
}

/// `float × double`: one `f32` rounding (the double constant enters
/// unrounded).
fn mul_d(v: f32, d: f64) -> f32 {
    (f64::from(v) * d) as f32
}

/// `x / S` with an unsigned division (REH §4.8: the coordinates divide as
/// unsigned); S ≤ 0 gives 0 ([choice]).
fn unsigned_div_tract(v: i32, s: i32) -> i32 {
    if s <= 0 {
        0
    } else {
        ((v as u32) / (s as u32)) as i32
    }
}

/// The residential simulator.
pub struct ResidentialSim {
    pub(super) host: Rc<dyn ResidentialHost>,
    pub(super) token: Cell<u64>,
    pub(super) s: RefCell<ResidentialState>,
    pub(super) rng: RefCell<Box<dyn ResidentialRandom>>,
    work_budget: Cell<u32>,
}

impl ResidentialSim {
    /// A constructed (not initialised) simulator, reference count 0 (§1).
    /// `rng` is the class's own shared `cRZRandom` (§3);
    /// [`DEFAULT_WORK_BUDGET`] replaces the 5 ms CPU-cycle slicer
    /// (§5.2 [choice]).
    pub fn new(host: Rc<dyn ResidentialHost>, token: u64, rng: Box<dyn ResidentialRandom>) -> Rc<ResidentialSim> {
        Rc::new(ResidentialSim {
            host,
            token: Cell::new(token),
            s: RefCell::new(ResidentialState::default()),
            rng: RefCell::new(rng),
            work_budget: Cell::new(DEFAULT_WORK_BUDGET),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn host(&self) -> &Rc<dyn ResidentialHost> {
        &self.host
    }

    /// A snapshot of the state (tests, diagnostics).
    pub fn snapshot(&self) -> ResidentialState {
        self.s.borrow().clone()
    }

    /// Changes the state directly (hosts restoring state, tests, vectors).
    /// `f` runs on a copy that is written back afterwards.
    pub fn with_state<R>(&self, f: impl FnOnce(&mut ResidentialState) -> R) -> R {
        let mut st = self.s.borrow().clone();
        let r = f(&mut st);
        *self.s.borrow_mut() = st;
        r
    }

    /// [choice] The work budget that replaces the 5 ms CPU-cycle slicer
    /// (§5.2): one unit is one distance pass (states 1, 2) or one tract
    /// (state 4).
    pub fn work_budget(&self) -> u32 {
        self.work_budget.get()
    }

    pub fn set_work_budget(&self, n: u32) {
        self.work_budget.set(n);
    }

    /// The class's shared generator (tests, a host driving the strike rolls
    /// outside state 0). Borrows the generator for the duration of `f`.
    pub fn with_rng<R>(&self, f: impl FnOnce(&mut dyn ResidentialRandom) -> R) -> R {
        f(&mut **self.rng.borrow_mut())
    }

    // ---- identity (§1) ----

    /// Slot 1 `AddRef`: increments the count and returns it.
    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release`: if the count is > 1, decrements it and returns the
    /// new count; otherwise calls its own `Shutdown`, destroys the object
    /// (here: marks it destroyed) and returns 0.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        let mut s = self.s.borrow_mut();
        s.ref_count = 0;
        s.destroyed = true;
        0
    }

    // ---- lifecycle (§4) ----

    /// `Init` (§4.1). Returns false if the demand node, the message server or
    /// the simulator is missing.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        // Step 1: services, developers and the demand node; only the demand
        // node is tested.
        h.acquire_services();
        h.acquire_developers();
        if !h.acquire_demand_node() {
            return false;
        }
        // Step 2: if no message server is held, take it and subscribe both
        // occupant messages.
        if !self.s.borrow().message_server_held {
            if !h.acquire_message_server() {
                return false;
            }
            self.s.borrow_mut().message_server_held = true;
            for &m in &msg::SUBSCRIPTIONS {
                h.subscribe(m);
            }
        }
        // Step 3: the budget simulator.
        h.acquire_budget_simulator();
        // Step 4: the simulator and the agent registration.
        if !h.has_simulator() {
            return false;
        }
        for &level in &AGENT_LEVELS {
            h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
        }
        // Step 5: the history queue lengths, interleaved per g, then
        // 0x4A8A3FE2 and 0x4A8A3FE5.
        for g in 0..history::AGE_GROUPS as u32 {
            h.set_history_queue_length(history::AGE_GROUP_POPULATION + g, history::QUEUE_LENGTH);
            h.set_history_queue_length(history::AGE_GROUP_EQ + g, history::QUEUE_LENGTH);
        }
        h.set_history_queue_length(history::HOSPITAL_COVERAGE, history::QUEUE_LENGTH);
        h.set_history_queue_length(history::SCHOOL_COVERAGE, history::QUEUE_LENGTH);

        // Step 6: the new-city part. [choice] Without the R$ developer's
        // population map P and E keep their constructed 4.
        if !self.s.borrow().city_started {
            let p = h
                .developer_population_map(DEVELOPERS[wealth::LOW])
                .map(|m| m.geometry().tract_size);
            let mut s = self.s.borrow_mut();
            if let Some(p) = p {
                s.population_size = p;
                s.eq_size = p;
            }
            s.population = 0;
            s.running_population = 0;
            s.state = st::STRIKES_AND_LISTS;
            s.city_started = true;
        }
        // Step 7: the simulator tuning and the ordinance-effect lists, when
        // not yet initialised.
        if !self.s.borrow().initialized {
            self.read_simulator_tuning();
            self.read_ordinance_effects(false);
        }
        // Step 8: create every absent grid of §2.
        self.create_grids();
        // Step 9: the departments, when absent.
        self.create_departments();
        // Step 10 (always): the iterator rectangle (the population R$ grid's
        // tract range, an empty range widened to one tract; the step is
        // kept) and Reset(0, 0); the population tuning; leSum := 0;
        // initialised := true.
        let counts = self.s.borrow().grids.population[wealth::LOW]
            .as_ref()
            .map(|g| (g.count_x(), g.count_z()))
            .unwrap_or((0, 0));
        {
            let mut s = self.s.borrow_mut();
            let rect = ScanIterator::for_counts(counts.0, counts.1);
            let it = &mut s.iterator;
            (it.x0, it.z0, it.x1, it.z1) = (rect.x0, rect.z0, rect.x1, rect.z1);
            it.reset(0, 0);
        }
        self.read_population_tuning();
        let mut s = self.s.borrow_mut();
        s.le_sum = 0;
        s.initialized = true;
        true
    }

    /// `Shutdown` (§4.2): always returns true.
    pub fn shutdown(&self) -> bool {
        if !self.s.borrow().initialized {
            return true;
        }
        let h = &*self.host;
        self.s.borrow_mut().initialized = false;
        h.remove_agent();
        for &m in &msg::SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        // The message server, the budget simulator and the four departments.
        self.s.borrow_mut().message_server_held = false;
        h.release_message_server_and_budget();
        let departments = {
            let mut s = self.s.borrow_mut();
            [
                s.health.department1.take(),
                s.health.department2.take(),
                s.education.department1.take(),
                s.education.department2.take(),
            ]
        };
        for d in departments.into_iter().flatten() {
            h.release_department(d);
        }
        // Every grid: Shutdown, Release and forget it.
        let mut grids = std::mem::take(&mut self.s.borrow_mut().grids);
        let temporary = grids.temporary.take();
        for g in grids.proximity.iter().flatten() {
            h.shutdown_grid_u8(g);
        }
        for g in grids.population.iter().flatten() {
            h.shutdown_grid_u16(g);
        }
        if let Some(g) = &grids.age {
            h.shutdown_grid_u8(g);
        }
        for g in grids
            .school_coverage
            .iter()
            .chain(grids.hospital_coverage.iter())
            .flatten()
        {
            h.shutdown_grid_u8(g);
        }
        for g in [&grids.school_grade, &grids.hospital_grade].into_iter().flatten() {
            h.shutdown_grid_f32(g);
        }
        for g in grids.eq.iter().chain(grids.hq.iter()).flatten() {
            h.shutdown_grid_f32(g);
        }
        drop(grids);
        // The demand node, the temporary grid (released, not shut down), the
        // city, the BDS, the occupant manager and the developers.
        drop(temporary);
        h.release_services();
        // Hand every occupant list to the occupant manager (slot 48,
        // [inferred]: releases the list's references) and clear it.
        let lists = std::mem::take(&mut self.s.borrow_mut().lists);
        for occ in lists.save_order().into_iter().flatten().flatten() {
            h.release(*occ);
        }
        true
    }

    fn create_grids(&self) {
        let h = &*self.host;
        let (p, e, prox) = {
            let s = self.s.borrow();
            (s.population_size, s.eq_size, s.proximity_size)
        };
        let mut g = std::mem::take(&mut self.s.borrow_mut().grids);
        let u8_grid = |id: u32, size: i32| GridProvider::<u8>::create_grid(h, id, size);
        let f32_grid = |id: u32, size: i32| GridProvider::<f32>::create_grid(h, id, size);
        for (i, slot) in g.proximity.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = u8_grid(instance::PROXIMITY[i], prox);
            }
        }
        for (i, slot) in g.population.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = GridProvider::<u16>::create_grid(h, instance::POPULATION[i], p);
            }
        }
        if g.age.is_none() {
            g.age = u8_grid(instance::AGE, p);
        }
        // The coverage vectors: if the first entry is null, every entry is
        // created (§4.1 step 8).
        if g.school_coverage[0].is_none() {
            for (i, slot) in g.school_coverage.iter_mut().enumerate() {
                *slot = u8_grid(instance::SCHOOL_COVERAGE[i], p);
            }
        }
        if g.hospital_coverage[0].is_none() {
            for (i, slot) in g.hospital_coverage.iter_mut().enumerate() {
                *slot = u8_grid(instance::HOSPITAL_COVERAGE[i], p);
            }
        }
        if g.school_grade.is_none() {
            g.school_grade = f32_grid(instance::SCHOOL_GRADE, p);
        }
        if g.hospital_grade.is_none() {
            g.hospital_grade = f32_grid(instance::HOSPITAL_GRADE, p);
        }
        for (i, slot) in g.eq.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = f32_grid(instance::EQ[i], e);
            }
        }
        for (i, slot) in g.hq.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = f32_grid(instance::HQ[i], e);
            }
        }
        self.s.borrow_mut().grids = g;
    }

    fn create_departments(&self) {
        use super::department as dept;
        let h = &*self.host;
        let (health, education) = {
            let s = self.s.borrow();
            (s.health, s.education)
        };
        // Health, then education; each block holds its primary and its
        // secondary department (REH §4.2). The secondary starts at 100 %.
        let make = |present: Option<u64>, id: u32, name: u32, max: f32, init: f32| match present {
            Some(d) => Some(d),
            None => h.create_department(id, name, max, init),
        };
        let h1 = make(
            health.department1,
            dept::HEALTH,
            dept::HEALTH_NAME,
            health.max_funding,
            health.init_funding,
        );
        let h2 = make(
            health.department2,
            dept::HEALTH_SECONDARY,
            dept::HEALTH_SECONDARY_NAME,
            health.max_funding,
            1.0,
        );
        let e1 = make(
            education.department1,
            dept::EDUCATION,
            dept::EDUCATION_NAME,
            education.max_funding,
            education.init_funding,
        );
        let e2 = make(
            education.department2,
            dept::EDUCATION_SECONDARY,
            dept::EDUCATION_SECONDARY_NAME,
            education.max_funding,
            1.0,
        );
        let mut s = self.s.borrow_mut();
        s.health.department1 = h1;
        s.health.department2 = h2;
        s.education.department1 = e1;
        s.education.department2 = e2;
    }

    /// §7.2: the simulator tuning, whose unconditional defaults overwrite the
    /// department blocks' scalars (including the strike flags and start days),
    /// so strikes do not survive save and load (§9.3 *[quirk]*).
    fn read_simulator_tuning(&self) {
        let exemplar = self.host.tuning();
        let (mut health, mut education) = {
            let s = self.s.borrow();
            (s.health, s.education)
        };
        let tuning = SimulatorTuning::read(exemplar.as_deref(), &mut health, &mut education);
        let mut s = self.s.borrow_mut();
        s.health = health;
        s.education = education;
        s.simulator_tuning = tuning;
    }

    /// §7.1: the population tuning, with the two decays scaled by the
    /// ordinance products (REH §4.3); tuningLoaded := true.
    fn read_population_tuning(&self) {
        let exemplar = self.host.tuning();
        let (edu, health) = self.ordinance_decay_products();
        let previous = self.s.borrow().population_tuning.pollution_curves.clone();
        let t = PopulationTuning::read(exemplar.as_deref(), edu, health, &previous);
        let mut s = self.s.borrow_mut();
        s.population_tuning = t;
        s.tuning_loaded = true;
    }

    // ---- messages and cadence (§4.3) ----

    /// `DoMessage` (§4.3). Returns the step's "finished" value: a NewDay step
    /// returns false only when a time-sliced state runs out of budget.
    pub fn do_message(&self, m: &ResidentialMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_DAY => self.do_daily_simulation(),
            msg::SIM_NEW_MONTH => {
                let mut s = self.s.borrow_mut();
                if s.state == st::IDLE {
                    s.state = st::STRIKES_AND_LISTS;
                    s.dirty = false;
                } else {
                    s.dirty = true;
                }
                true
            }
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if m.data3 == OCCUPANT_TYPE_BUILDING {
                    if let Some(building) = m.void1.and_then(|occ| self.host.occupant_as_building(occ)) {
                        self.do_building_changed(building, m.msg_type == msg::OCCUPANT_INSERTED);
                    }
                }
                true
            }
            // SimBegin, SimIdle and anything else: nothing.
            _ => true,
        }
    }

    /// One step of the daily state machine (§5.1). At most one state per
    /// call; returns false only when a time-sliced state (1, 2, 4) runs out
    /// of its work budget (§5.2).
    pub fn do_daily_simulation(&self) -> bool {
        let mut budget = self.work_budget.get();
        let state = self.s.borrow().state;
        let next = match state {
            st::STRIKES_AND_LISTS => {
                self.state_0();
                st::SEED_LOW
            }
            st::SEED_LOW => {
                self.seed_temporary(wealth::LOW);
                st::PROXIMITY_LOW
            }
            st::PROXIMITY_LOW => {
                if !self.proximity_passes(wealth::LOW, &mut budget) {
                    return false;
                }
                st::SEED_MEDIUM
            }
            st::SEED_MEDIUM => {
                self.seed_temporary(wealth::MEDIUM);
                st::PROXIMITY_MEDIUM
            }
            st::PROXIMITY_MEDIUM => {
                if !self.proximity_passes(wealth::MEDIUM, &mut budget) {
                    return false;
                }
                st::SEED_HIGH
            }
            st::SEED_HIGH => {
                // Seeds R$$$ and runs the full transform at once (§5.3).
                self.seed_temporary(wealth::HIGH);
                if let Some(t) = self.s.borrow().grids.temporary.clone() {
                    distance_to_convergence(&*t, t.count_x(), t.count_z());
                }
                st::PROXIMITY_HIGH
            }
            st::PROXIMITY_HIGH => {
                self.finish_proximity();
                st::HISTOGRAM_RESET
            }
            st::HISTOGRAM_RESET => {
                self.state_11();
                st::POPULATION
            }
            st::POPULATION => {
                if !self.state_4(&mut budget) {
                    return false;
                }
                st::POST_POPULATION
            }
            st::POST_POPULATION => {
                self.s.borrow_mut().iterator.reset(0, 0);
                self.post_population_change();
                st::SCHOOL_COVERAGE
            }
            st::SCHOOL_COVERAGE => {
                self.coverage_state(true);
                st::SCHOOL_GRADING
            }
            st::SCHOOL_GRADING => {
                self.grading_state(true);
                st::HOSPITAL_COVERAGE
            }
            st::HOSPITAL_COVERAGE => {
                self.coverage_state(false);
                st::HOSPITAL_GRADING
            }
            st::HOSPITAL_GRADING => {
                self.grading_state(false);
                st::HISTORY
            }
            st::HISTORY => {
                self.state_9();
                st::IDLE
            }
            // State 10 waits for NewMonth; [choice] an unknown state (a
            // hostile save) waits too.
            _ => return true,
        };
        self.s.borrow_mut().state = next;
        true
    }

    /// State 0 (§5.1, REH §3): the grade totals, the strike checks (schools,
    /// then hospitals) and the ordinance-effect refresh (`education_health.rs`),
    /// then the pending school and hospital lists, then the pending
    /// health-affecting list (§5.8).
    fn state_0(&self) {
        let h = &*self.host;
        self.education_health_state_0();
        let mut lists = self.s.borrow().lists.clone();
        distribute(h, &mut lists.pending_school, &mut lists.school_groups, &group::SCHOOL);
        distribute(
            h,
            &mut lists.pending_hospital,
            &mut lists.hospital_groups,
            &group::HOSPITAL,
        );
        // The other list is the pending health-affecting list (§5.8, RS8):
        // every entry is appended to the health-affecting list, in order, and
        // the pending list is cleared.
        let moved = std::mem::take(&mut lists.other);
        lists.health_affecting.extend(moved);
        self.s.borrow_mut().lists = lists;
    }

    /// States 13/14/15 (§5.3 step 1): create the temporary grid if absent and
    /// seed it for wealth `w`. R$$ and R$$$ reuse the R$ grid. *[compat]*
    /// (RG23): the seed overwrites only the z = 0 edge, so every other cell
    /// keeps the previous wealth's result (0 off the edge, as on a fresh grid).
    fn seed_temporary(&self, w: usize) {
        let h = &*self.host;
        let key = h.building_map_key(BUILDING_PURPOSE_RESIDENTIAL, wealth::NUMBERS[w]);
        let map = h.building_map();
        let (temp, prox) = {
            let s = self.s.borrow();
            (s.grids.temporary.clone(), s.proximity_size)
        };
        let temp = match temp {
            Some(t) => t,
            None => {
                let Some(t) = GridProvider::<u8>::create_grid(h, 0, prox) else {
                    return;
                };
                self.s.borrow_mut().grids.temporary = Some(t.clone());
                t
            }
        };
        // *[compat]* (§5.3 step 1, RG24): with no building map the seed writes
        // nothing, not even the edge, so a fresh grid stays all 0 and a
        // reused one keeps the previous wealth's result: every proximity grid
        // is then 0. The grid is still created first, as state 13 does.
        if let Some(map) = map {
            seed_proximity(&*temp, &*map, key);
        }
    }

    /// States 1 and 2 (§5.1–§5.3): distance passes on the temporary grid
    /// until one reports no change, one pass per work unit, then the copy
    /// into proximity grid `w`. False when the budget ran out first.
    fn proximity_passes(&self, w: usize, budget: &mut u32) -> bool {
        let (temp, prox) = {
            let s = self.s.borrow();
            (s.grids.temporary.clone(), s.grids.proximity[w].clone())
        };
        let Some(temp) = temp else {
            // No temporary grid (the host could not create one): nothing to
            // transform or copy.
            return true;
        };
        let (cx, cz) = (temp.count_x(), temp.count_z());
        loop {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            if !distance_pass(&*temp, cx, cz) {
                break;
            }
        }
        if let Some(p) = prox {
            copy_proximity(&*p, &*temp);
        }
        true
    }

    /// State 3 (§5.3): one pass (no change), copy into proximity R$$$, then
    /// `Shutdown` and release the temporary grid.
    fn finish_proximity(&self) {
        let (temp, prox) = {
            let mut s = self.s.borrow_mut();
            (s.grids.temporary.take(), s.grids.proximity[wealth::HIGH].clone())
        };
        if let Some(t) = temp {
            distance_pass(&*t, t.count_x(), t.count_z());
            if let Some(p) = prox {
                copy_proximity(&*p, &*t);
            }
            self.host.shutdown_grid_u8(&t);
        }
    }

    /// State 11 (§5.4): reset the histogram, leSum, the running population
    /// and the tuning-loaded flag.
    fn state_11(&self) {
        let mut s = self.s.borrow_mut();
        s.age_count = [0; AGE_BUCKETS];
        s.age_eq_sum = [0; AGE_BUCKETS];
        s.le_sum = 0;
        s.running_population = 0;
        s.tuning_loaded = false;
    }

    /// State 4 (§5.5): `DoPopulationChange` for every tract of the scanning
    /// iterator, one tract per work unit. False when the budget ran out.
    fn state_4(&self, budget: &mut u32) -> bool {
        if !self.s.borrow().tuning_loaded {
            self.read_population_tuning();
        }
        loop {
            if *budget == 0 {
                return false;
            }
            let next = self.s.borrow_mut().iterator.next_cell();
            let Some((x, z)) = next else {
                return true;
            };
            *budget -= 1;
            self.do_population_change(x, z);
        }
    }

    /// §5.5 step 1: `hqLoss = ((HQdecay + air(p0)) + water(p1)) + garbage(p2)`
    /// with `p_q` the low byte of the tract's average source pollution.
    pub fn hq_loss(&self, t: &PopulationTuning, s_size: i32, x: i32, z: i32) -> f32 {
        let h = &*self.host;
        let mut p = [0u32; 3];
        let cells = s_size.clamp(0, MAX_TRACTS);
        // S·S is a 32-bit product, the sum a wrapping u32 and the division
        // unsigned (RS3).
        let denom = (s_size.max(0) as u32).wrapping_mul(s_size.max(0) as u32);
        for (q, slot) in p.iter_mut().enumerate() {
            let mut sum = 0u32;
            for i in 0..cells {
                for j in 0..cells {
                    let cx = s_size.wrapping_mul(x).wrapping_add(i);
                    let cz = s_size.wrapping_mul(z).wrapping_add(j);
                    sum = sum.wrapping_add(u32::from(h.pollution_value(q as u32, cx, cz)));
                }
            }
            // The caller keeps only the low byte [quirk]. [choice] A zero
            // divisor (S ≤ 0, or S·S wrapping to 0) gives 0.
            *slot = sum.checked_div(denom).unwrap_or(0) & 0xFF;
        }
        let mut loss = t.hq_decay;
        for (curve, index) in t.pollution_curves.iter().zip(p) {
            loss += curve.eval(index as f32);
        }
        loss
    }

    /// `DoPopulationChange(T, x, z)` for one tract (§5.5, REH §4.7), with the
    /// population tuning currently loaded. Internal in the original; public
    /// for tests and vectors. Newcomer arithmetic uses invocation precision,
    /// with float grid stores in the order the spec gives (foundations §18.4).
    pub fn do_population_change(&self, x: i32, z: i32) {
        let h = &*self.host;
        // Clone the grids and the scalars this tract needs, so no state
        // borrow is held across a host callback.
        let (s_size, t, eq, hq, pop, age_grid) = {
            let s = self.s.borrow();
            (
                // §5.5, EH14: S is the R$ population grid's own
                // `GetTractSize()`, not P. [choice] P without that grid
                // (`Init` always creates it).
                s.grids.population[wealth::LOW]
                    .as_ref()
                    .map_or(s.population_size, |g| g.geometry().tract_size),
                s.population_tuning.clone(),
                s.grids.eq.clone(),
                s.grids.hq.clone(),
                s.grids.population.clone(),
                s.grids.age.clone(),
            )
        };
        let get_f = |g: &Option<Rc<dyn TractGrid<f32>>>| g.as_ref().map_or(0.0, |g| g.get(x, z));
        let set_f = |g: &Option<Rc<dyn TractGrid<f32>>>, v: f32| {
            if let Some(g) = g {
                g.set(x, z, v);
            }
        };
        let get_p = |w: usize| pop[w].as_ref().map_or(0, |g| g.get(x, z));

        // Step 1: pollution → hqLoss.
        let hq_loss = self.hq_loss(&t, s_size, x, z);

        // Step 2: decay and buzzards, one flag per tract.
        let mut buzzard = false;
        for w in 0..wealth::COUNT {
            let e = get_f(&eq[w]) - t.eq_decay;
            set_f(&eq[w], if e < 0.0 { 0.0 } else { e });
            let q = get_f(&hq[w]) - hq_loss;
            let hq_after = if q < 0.0 { 0.0 } else { q };
            set_f(&hq[w], hq_after);
            // HQ below the door (strict), no effect yet in this tract, and
            // the current population (before this pass's update) non-zero.
            if hq_after < t.deaths_door_hq && !buzzard && get_p(w) != 0 {
                let r = self.rng.borrow_mut().range_f64(0.0, 100.0);
                if r < f64::from(t.effect_chance) {
                    // RG1 (REH §4.7 step 3): the game creates the effect. No
                    // effect: nothing else happens, and a later wealth level
                    // of the same tract may roll again. A host that cannot
                    // show effects: treated as created [choice], so both
                    // draws and the flag, as the original with shipped data.
                    match h.create_visual_effect(BUZZARDS_EFFECT) {
                        CreatedEffect::Failed => {}
                        CreatedEffect::NotShown => {
                            let delta = f64::from(t.effect_delta);
                            let _ = self.rng.borrow_mut().range_f64(-delta, delta);
                            buzzard = true;
                        }
                        CreatedEffect::Effect(effect) => {
                            let geom = eq[w].as_ref().map(|g| g.geometry()).unwrap_or_default();
                            // wx = float(width × x) + width × 0.5, each an f32 step.
                            let wx = mul_i(i64::from(x), geom.width_x) + mul_d(geom.width_x, 0.5);
                            let wz = mul_i(i64::from(z), geom.width_z) + mul_d(geom.width_z, 0.5);
                            let base = h.altitude(wx, wz) + t.effect_height;
                            let delta = f64::from(t.effect_delta);
                            let dy = self.rng.borrow_mut().range_f64(-delta, delta);
                            let y = (f64::from(base) + dy) as f32;
                            h.set_effect_transform(effect, [wx, y, wz]);
                            h.start_and_release_effect(effect);
                            buzzard = true;
                        }
                    }
                }
            }
        }

        // Step 3: read all three new populations before any write; the
        // int16 grid's 16 bits are used as unsigned.
        let new_pop: [u16; wealth::COUNT] = std::array::from_fn(|w| {
            h.developer_population_map(DEVELOPERS[w])
                .map_or(0, |m| m.get(x, z) as u16)
        });

        // Step 4: the population and the newcomers' EQ/HQ blend.
        let mut arrived = false;
        let mut total = 0i32;
        let mut old_sum = 0i32;
        let mut nc_age_sum = 0i32;
        for w in 0..wealth::COUNT {
            let new_w = new_pop[w];
            let old_w = get_p(w);
            total = total.wrapping_add(i32::from(new_w));
            old_sum = old_sum.wrapping_add(i32::from(old_w));
            if let Some(g) = &pop[w] {
                g.set(x, z, new_w);
            }
            if old_w < new_w {
                // delta := int16(new − old) (REH §4.7 step 4).
                let nc = new_w.wrapping_sub(old_w) as i16;
                let blend = |g: &Option<Rc<dyn TractGrid<f32>>>, bonus: f32| {
                    let pc = h.invocation_precision();
                    let q_old = f64::from(get_f(g));
                    let m = if q_old > 0.0 { 2 } else { 1 };
                    let avg = pc.round(pc.round(q_old + f64::from(bonus)) / f64::from(m));
                    let num = pc.round(pc.round(f64::from(nc) * avg) + pc.round(f64::from(old_w) * q_old));
                    let q = pc.round(num / f64::from(new_w));
                    // Compare the register result, then store float32 (REH §4.7).
                    set_f(
                        g,
                        if f64::from(MAX_QUOTIENT) < q {
                            MAX_QUOTIENT as f32
                        } else {
                            q as f32
                        },
                    );
                };
                blend(&eq[w], t.new_eq[w]);
                blend(&hq[w], t.new_hq[w]);
                nc_age_sum = nc_age_sum.wrapping_add(i32::from(nc).wrapping_mul(t.new_age[w]));
                arrived = true;
            }
            if new_w == 0 {
                set_f(&eq[w], 0.0);
                set_f(&hq[w], 0.0);
            }
        }

        // Step 5: ageing. The month is read per tract with total ≠ 0, nothing
        // latched (RS2).
        let mut age = i32::from(age_grid.as_ref().map_or(0, |g| g.get(x, z)));
        if total != 0 {
            if h.sim_month() == 12 {
                age = (age + 1) & 0xFF;
            }
        } else {
            age = 0;
        }

        // Step 6: the newcomers' age, signed 32-bit and truncated *[quirk:
        // the old populations of every level]*.
        if arrived && total != 0 {
            age = old_sum.wrapping_mul(age).wrapping_add(nc_age_sum).wrapping_div(total);
        }

        // Step 7: life expectancy, after the EQ/HQ updates.
        let le = i32::from(self.get_cell_life_expectancy(s_size.wrapping_mul(x), s_size.wrapping_mul(z)));

        // Step 8: the generational reset.
        if age > le {
            for w in 0..wealth::COUNT {
                set_f(&eq[w], get_f(&eq[w]) * t.eq_reset);
                set_f(&hq[w], get_f(&hq[w]) * t.hq_reset);
            }
            age = t.reset_age;
        }

        // Step 9: the low byte of the age.
        if let Some(g) = &age_grid {
            g.set(x, z, (age & 0xFF) as u8);
        }

        // Steps 10 and 11: leSum += LE × total (32-bit product, sign-extended
        // to 64 bits); the histogram with the post-reset age and EQ.
        let b = age_bucket(age);
        let mut dcount = 0i32;
        let mut deqsum = 0i32;
        for w in 0..wealth::COUNT {
            let pw = i32::from(new_pop[w]);
            dcount = dcount.wrapping_add(pw);
            deqsum = deqsum.wrapping_add(ftol32(f64::from(get_f(&eq[w]))).wrapping_mul(pw));
        }
        let mut s = self.s.borrow_mut();
        s.running_population = s.running_population.wrapping_add(total);
        s.le_sum = s.le_sum.wrapping_add(i64::from(le.wrapping_mul(total)) as u64);
        s.age_count[b] = s.age_count[b].wrapping_add(dcount);
        s.age_eq_sum[b] = s.age_eq_sum[b].wrapping_add(deqsum);
    }

    /// State 12 (§5.6): `PostPopulationChange` (the iterator was reset by the
    /// caller).
    fn post_population_change(&self) {
        let (population, le_sum, age_count, age_eq_sum, curves) = {
            let mut s = self.s.borrow_mut();
            // Step 1: population := running population.
            s.population = s.running_population;
            (
                s.population,
                s.le_sum,
                s.age_count,
                s.age_eq_sum,
                s.simulator_tuning.age_group_curves.clone(),
            )
        };
        // Step 2: the population-weighted average life expectancy: an unsigned
        // 64-bit division by the population sign-extended to 64 bits, guarded
        // by `≠ 0` (RS16), so a negative population divides by 2⁶⁴ − |p|.
        let avg_le = if population != 0 {
            (le_sum / i64::from(population) as u64) as i32
        } else {
            0
        };
        self.host.record_history(history::AVERAGE_LIFE_EXPECTANCY, avg_le);
        // Step 3: the per-bucket average EQ, both counters read as unsigned.
        let avg: [f32; AGE_BUCKETS] = std::array::from_fn(|b| {
            if age_count[b] != 0 {
                (u32_to_float(age_eq_sum[b]) / u32_to_float(age_count[b])) as f32
            } else {
                0.0
            }
        });
        // Step 4: the age groups.
        for (g, curve) in curves.iter().enumerate() {
            let mut pop_sum = 0i32;
            let mut eq_sum = 0i32;
            for b in 0..AGE_BUCKETS {
                if age_count[b] == 0 {
                    continue;
                }
                // n := trunc((curve(float(21 + 3b)) × float(ageCount[b])) × 0.01).
                let cv = curve.eval((21 + 3 * b) as f32);
                let n = ftol32(f64::from(pct(mul_u32(age_count[b], cv))));
                pop_sum = pop_sum.wrapping_add(n);
                // eqSum := trunc(float(n) × avg_b + float(eqSum)).
                eq_sum = ftol32(f64::from(add_i(mul_i(i64::from(n), avg[b]), i64::from(eq_sum))));
            }
            let g = g as u32;
            self.host.record_history(history::AGE_GROUP_POPULATION + g, pop_sum);
            let group_eq = if pop_sum > 0 { eq_sum.wrapping_div(pop_sum) } else { 0 };
            self.host.record_history(history::AGE_GROUP_EQ + g, group_eq);
        }
    }

    /// State 9 (§5.7, REH §4.12): the history.
    fn state_9(&self) {
        let h = &*self.host;
        // In this order: global EQ, global HQ, population.
        let geq = self.get_global_eq();
        let ghq = self.get_global_hq();
        let (population, school, hospital) = {
            let s = self.s.borrow();
            (s.population, s.school_covered, s.hospital_covered)
        };
        h.record_history(history::GLOBAL_EQ, ftol32(f64::from(geq)));
        h.record_history(history::GLOBAL_HQ, ftol32(f64::from(ghq)));
        h.record_history(history::POPULATION, population);
        if population > 0 {
            // h is computed first, then school, then hospital (§5.7 [quirk]:
            // the two branches write 0x4A8A3FE5 / 0x4A8A3FE2 in opposite
            // orders).
            let pop = f64::from(population);
            let hfrac = (f64::from(hospital) / pop) as f32;
            let sfrac = (f64::from(school) / pop) as f32;
            h.record_history(history::SCHOOL_COVERAGE, rhe(mul_d(sfrac, 1000.0)));
            h.record_history(history::HOSPITAL_COVERAGE, rhe(mul_d(hfrac, 1000.0)));
        } else {
            h.record_history(history::HOSPITAL_COVERAGE, 0);
            h.record_history(history::SCHOOL_COVERAGE, 0);
        }
    }

    // ---- population-half queries (§6) ----

    /// Slot 16 `GetPopulation`.
    pub fn get_population(&self) -> i32 {
        self.s.borrow().population
    }

    /// Slot 5 `GetProximityMap(w)`: w = 1/2/3 → the proximity grid; anything
    /// else → `None`. No `AddRef`.
    pub fn get_proximity_map(&self, w: u32) -> Option<Rc<dyn TractGrid<u8>>> {
        let idx = match w {
            1 => wealth::LOW,
            2 => wealth::MEDIUM,
            3 => wealth::HIGH,
            _ => return None,
        };
        self.s.borrow().grids.proximity[idx].clone()
    }

    /// Slots 39 / 40 / 41: the grade maps and the age grid (no `AddRef`).
    pub fn school_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.s.borrow().grids.school_grade.clone()
    }
    pub fn hospital_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.s.borrow().grids.hospital_grade.clone()
    }
    pub fn average_age_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.s.borrow().grids.age.clone()
    }

    /// `GetTractQuotient(values[3], pops[3], x, z)` (§6.1, REH §4.8): the
    /// population sum is kept in 16 bits *[quirk]*.
    pub fn tract_quotient(
        values: &[Option<Rc<dyn TractGrid<f32>>>; 3],
        pops: &[Option<Rc<dyn TractGrid<u16>>>; 3],
        x: i32,
        z: i32,
    ) -> f32 {
        let p: [u16; 3] = std::array::from_fn(|w| pops[w].as_ref().map_or(0, |g| g.get(x, z)));
        let sum16 = p[0].wrapping_add(p[1]).wrapping_add(p[2]);
        if sum16 == 0 {
            return 0.0;
        }
        let v: [f32; 3] = std::array::from_fn(|w| values[w].as_ref().map_or(0.0, |g| g.get(x, z)));
        // s := q2·p2; s := q1·p1 + s; result := (q0·p0 + s) / float(sum16)
        // (float addition is commutative, so `+=` is the same rounding).
        let mut s = mul_i(i64::from(p[2]), v[2]);
        s += mul_i(i64::from(p[1]), v[1]);
        s += mul_i(i64::from(p[0]), v[0]);
        div_i(s, i64::from(sum16))
    }

    fn quotient_grids(&self, is_eq: bool) -> (QuotientGrids, PopulationGrids) {
        let s = self.s.borrow();
        let q = if is_eq { s.grids.eq.clone() } else { s.grids.hq.clone() };
        (q, s.grids.population.clone())
    }

    /// The tract size the cell queries divide by (REH §4.8, AQ2, AQ3): the
    /// R$ grid's own `GetTractSize()`, asked on every call — the R$ **EQ**
    /// grid for slots 35, 37 and 43, the R$ **HQ** grid for slots 36, 38 and
    /// 44 (never an EQ grid's, and not the member E). The grids keep their
    /// saved geometry through a load, so these queries follow the grids; E
    /// differs from them only in a hand-made save. [choice] Without an R$
    /// grid (the original calls through null) the member E.
    fn quotient_tract_size(&self, q: &QuotientGrids) -> i32 {
        match q[0].as_ref() {
            Some(g) => g.geometry().tract_size,
            None => self.s.borrow().eq_size,
        }
    }

    /// Slots 35 / 36 `GetCellEQ` / `GetCellHQ(x, z)` (REH §4.8): the quotient
    /// at tract (x ÷ T, z ÷ T), unsigned division, T the R$ EQ (slot 35) or
    /// R$ HQ (slot 36) grid's own tract size (AQ2, AQ3).
    pub fn get_cell_eq(&self, x: i32, z: i32) -> f32 {
        self.cell_quotient(true, x, z)
    }
    pub fn get_cell_hq(&self, x: i32, z: i32) -> f32 {
        self.cell_quotient(false, x, z)
    }

    fn cell_quotient(&self, is_eq: bool, x: i32, z: i32) -> f32 {
        let (q, pop) = self.quotient_grids(is_eq);
        let t = self.quotient_tract_size(&q);
        Self::tract_quotient(&q, &pop, unsigned_div_tract(x, t), unsigned_div_tract(z, t))
    }

    /// Slots 37 / 38 `GetCellEQByWealth` / `GetCellHQByWealth`: grid
    /// `w − 1` at tract (x ÷ T, z ÷ T), T the **R$** EQ (37) or HQ (38)
    /// grid's tract size whatever the wealth (REH §4.8, AQ2, AQ3). The wealth
    /// byte is decremented, so wealth 0 or > 3 reads out of bounds in the
    /// original *[quirk]*; FreeCity4 returns 0.0 ([choice]).
    pub fn get_cell_eq_by_wealth(&self, x: i32, z: i32, w: u8) -> f32 {
        self.cell_by_wealth(true, x, z, w)
    }
    pub fn get_cell_hq_by_wealth(&self, x: i32, z: i32, w: u8) -> f32 {
        self.cell_by_wealth(false, x, z, w)
    }

    fn cell_by_wealth(&self, is_eq: bool, x: i32, z: i32, w: u8) -> f32 {
        let idx = usize::from(w.wrapping_sub(1));
        if idx >= wealth::COUNT {
            return 0.0;
        }
        let (q, _) = self.quotient_grids(is_eq);
        let t = self.quotient_tract_size(&q);
        q[idx]
            .as_ref()
            .map_or(0.0, |g| g.get(unsigned_div_tract(x, t), unsigned_div_tract(z, t)))
    }

    /// Slots 32 / 33 `GetGlobalEQ` / `GetGlobalHQ` (REH §4.8).
    pub fn get_global_eq(&self) -> f32 {
        self.global_quotient(true)
    }
    pub fn get_global_hq(&self) -> f32 {
        self.global_quotient(false)
    }

    fn global_quotient(&self, is_eq: bool) -> f32 {
        self.global_quotient_register(is_eq) as f32
    }

    /// REH §4.8: final quotient in the register, before a caller's float store.
    pub fn global_quotient_register(&self, is_eq: bool) -> f64 {
        let pc = self.host.invocation_precision();
        let population = self.s.borrow().population;
        if population == 0 {
            return 0.0;
        }
        let (q, pop) = self.quotient_grids(is_eq);
        let (cx, cz) = q[0].as_ref().map_or((0, 0), |g| (g.count_x(), g.count_z()));
        let mut sum = 0.0f32;
        for x in 0..cx.clamp(0, MAX_TRACTS) {
            for z in 0..cz.clamp(0, MAX_TRACTS) {
                // a := Q0·p0; b := Q1·p1 + a; c := Q2·p2 + b; sum := c + sum.
                let p = |w: usize| f64::from(pop[w].as_ref().map_or(0, |g| g.get(x, z)));
                let v = |w: usize| f64::from(q[w].as_ref().map_or(0.0, |g| g.get(x, z)));
                let a = pc.round(p(0) * v(0)) as f32;
                let b = pc.round(pc.round(p(1) * v(1)) + f64::from(a)) as f32;
                let c = pc.round(pc.round(p(2) * v(2)) + f64::from(b));
                sum = pc.round(c + f64::from(sum)) as f32;
            }
        }
        pc.round(f64::from(sum) / f64::from(population))
    }

    /// Slots 21 / 22 `GetAverageEQGrid` / `GetAverageHQGrid` (§6.2): a **new**
    /// float grid per call, owned by the caller, plus `(min, max)`. Each tract
    /// := `float(trunc(quotient))`; min starts at 255.0 and max at 0.0, each
    /// updated with every tract, populated or not (REH §4.8, RS6). No grid
    /// (the host could not create one) → `None` with the initial values.
    /// *[compat]* (REH §4.8, AQ2, AQ3): the new grid's tract size is the
    /// member E for both slots — the only queries that read E.
    pub fn average_quotient_grid(&self, is_eq: bool) -> (Option<Rc<dyn TractGrid<f32>>>, f32, f32) {
        let e = self.s.borrow().eq_size;
        let id = if is_eq {
            instance::AVERAGE_EQ
        } else {
            instance::AVERAGE_HQ
        };
        let Some(grid) = GridProvider::<f32>::create_grid(&*self.host, id, e) else {
            return (None, 255.0, 0.0);
        };
        let (q, pop) = self.quotient_grids(is_eq);
        let mut min = 255.0f32;
        let mut max = 0.0f32;
        for x in 0..grid.count_x().clamp(0, MAX_TRACTS) {
            for z in 0..grid.count_z().clamp(0, MAX_TRACTS) {
                let v = ftol32(f64::from(Self::tract_quotient(&q, &pop, x, z))) as f32;
                grid.set(x, z, v);
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
            }
        }
        (Some(grid), min, max)
    }

    /// Slots 23 / 24 / 25 `GetEQGrids` / `GetHQGrids` / `GetPopulationGrids`
    /// (§6.3): the three grids written to the three out-references, each
    /// `AddRef`ed — here, the `Rc` clones the caller now holds. EAX is
    /// [`GRID_TRIPLE_RESULT`].
    pub fn get_eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.s.borrow().grids.eq.clone()
    }
    pub fn get_hq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.s.borrow().grids.hq.clone()
    }
    pub fn get_population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3] {
        self.s.borrow().grids.population.clone()
    }

    /// Slot 29 `GetCellLifeExpectancy(x, z)` (§6.4): the HQ→LE curve at
    /// `GetCellHQ`, truncated, AL. No exemplar → 0.
    pub fn get_cell_life_expectancy(&self, x: i32, z: i32) -> u8 {
        let Some(curve) = le_curve(self.host.tuning().as_deref()) else {
            return 0;
        };
        (ftol32(f64::from(curve.eval(self.get_cell_hq(x, z)))) & 0xFF) as u8
    }

    /// Slot 34 `GetGlobalLE()`: the same curve at `GetGlobalHQ`, AL.
    pub fn get_global_le(&self) -> u8 {
        let Some(curve) = le_curve(self.host.tuning().as_deref()) else {
            return 0;
        };
        (ftol32(f64::from(curve.eval(self.get_global_hq()))) & 0xFF) as u8
    }

    /// Slot 30 `GetCellWorkforcePercent(x, z)` (§6.4): the LE→workforce curve
    /// at `float(slot 29)`, × 0.01, written through the hidden pointer. No
    /// exemplar → 0.0 *[inferred]*.
    pub fn get_cell_workforce_percent(&self, x: i32, z: i32) -> f32 {
        let Some(curve) = workforce_curve(self.host.tuning().as_deref()) else {
            return 0.0;
        };
        let le = self.get_cell_life_expectancy(x, z);
        pct(curve.eval(f32::from(le)))
    }

    /// Slot 31 `GetGlobalWorkforcePercent()`: the same with slot 34.
    pub fn get_global_workforce_percent(&self) -> f32 {
        let Some(curve) = workforce_curve(self.host.tuning().as_deref()) else {
            return 0.0;
        };
        let le = self.get_global_le();
        pct(curve.eval(f32::from(le)))
    }

    /// Slot 42 `GetAverageNewAgeByWealth(w)` (§6.5): `ages[(w − 1) mod 256]`
    /// in AL. w ∉ {1, 2, 3} returns 0 ([choice]; the original reads out of
    /// bounds).
    pub fn get_average_new_age_by_wealth(&self, w: u8) -> u8 {
        let ages = new_ages_by_wealth(self.host.tuning().as_deref());
        ages.get(usize::from(w.wrapping_sub(1))).copied().unwrap_or(0)
    }

    /// Slots 50 / 51 `GetMaxEQ` / `GetMaxHQ`: 200.
    pub fn max_quotient(&self) -> i32 {
        MAX_QUOTIENT
    }

    /// Slots 17 / 18: always 0.
    pub fn total_city_upkeep_cost(&self) -> i32 {
        0
    }

    /// Slots 52–55: the global auto-budget flags; the setters return the
    /// value.
    pub fn global_auto_budget_schools(&self) -> bool {
        self.s.borrow().school_auto_budget
    }
    pub fn set_global_auto_budget_schools(&self, v: bool) -> bool {
        self.s.borrow_mut().school_auto_budget = v;
        v
    }
    pub fn global_auto_budget_hospitals(&self) -> bool {
        self.s.borrow().hospital_auto_budget
    }
    pub fn set_global_auto_budget_hospitals(&self, v: bool) -> bool {
        self.s.borrow_mut().hospital_auto_budget = v;
        v
    }

    /// Slots 56 / 57 `GetAutoBudget` / `SetAutoBudget`: the occupant's bool
    /// property `0x6C05FC4D`, default false (§6.6).
    pub fn get_auto_budget(&self, occupant: u64) -> bool {
        self.host
            .occupant_bool_property(occupant, PROP_AUTO_BUDGET)
            .unwrap_or(false)
    }
    pub fn set_auto_budget(&self, occupant: u64, v: bool) -> bool {
        self.host.set_occupant_bool_property(occupant, PROP_AUTO_BUDGET, v)
    }

    /// Slots 43 / 44 `GetEQMinAndMaxCellCoords` / `GetHQMinAndMaxCellCoords`
    /// (REH §4.8): `Some((minX, minZ, maxX, maxZ, min, max))`, or `None` when
    /// the population is 0 (all six outputs 0, AL = 0). [choice] When no
    /// tract is populated (the counter disagrees with the grids) the
    /// coordinates are those of tract (0, 0) instead of stack garbage. The
    /// coordinates use the R$ EQ (43) or R$ HQ (44) grid's own tract size
    /// (AQ2, AQ3).
    pub fn quotient_min_max(&self, is_eq: bool) -> Option<(u32, u32, u32, u32, f32, f32)> {
        if self.s.borrow().population == 0 {
            return None;
        }
        let (q, pop) = self.quotient_grids(is_eq);
        let e = self.quotient_tract_size(&q);
        let (cx, cz) = q[0].as_ref().map_or((0, 0), |g| (g.count_x(), g.count_z()));
        let mut min = 200.0f32;
        let mut max = -200.0f32;
        let (mut min_t, mut max_t) = ((0i32, 0i32), (0i32, 0i32));
        for x in 0..cx.clamp(0, MAX_TRACTS) {
            for z in 0..cz.clamp(0, MAX_TRACTS) {
                if (0..wealth::COUNT).all(|w| pop[w].as_ref().map_or(0, |g| g.get(x, z)) == 0) {
                    continue;
                }
                let v = Self::tract_quotient(&q, &pop, x, z);
                if v < min {
                    min = v;
                    min_t = (x, z);
                }
                if v > max {
                    max = v;
                    max_t = (x, z);
                }
            }
        }
        // Coordinates out := T/2 + T × tract (signed /2), T the R$ grid's.
        let coord = |t: i32| (e / 2).wrapping_add(e.wrapping_mul(t)) as u32;
        Some((coord(min_t.0), coord(min_t.1), coord(max_t.0), coord(max_t.1), min, max))
    }

    // ---- strikes (slots 6–11, REH §4.9) ----

    /// Slots 6 / 7: the strike flags.
    pub fn school_is_on_strike(&self) -> bool {
        self.s.borrow().education.strike_flag != 0
    }
    pub fn health_is_on_strike(&self) -> bool {
        self.s.borrow().health.strike_flag != 0
    }

    /// Slots 8 / 9 `EndSchoolStrike` / `EndHealthStrike`: if the service is
    /// on strike, `EndStrike(dept, today − start)` — flag := 0, start := 0,
    /// post the end-of-strike message (data2 = 0, data3 = the length) — and
    /// true; otherwise false. The buildings' strike days are not cleared.
    pub fn end_school_strike(&self) -> bool {
        self.end_strike(true)
    }
    pub fn end_health_strike(&self) -> bool {
        self.end_strike(false)
    }

    pub(super) fn end_strike(&self, school: bool) -> bool {
        if !(if school {
            self.school_is_on_strike()
        } else {
            self.health_is_on_strike()
        }) {
            return false;
        }
        let today = self.host.sim_day_number();
        let ended = {
            let mut s = self.s.borrow_mut();
            let block = if school { &mut s.education } else { &mut s.health };
            block.end_strike(today).map(|length| (block.id, length))
        };
        match ended {
            Some((msg_type, length)) => {
                self.host.post_strike_message(msg_type, None, 0, length);
                true
            }
            None => false,
        }
    }

    /// Slots 10 / 11 `ChanceOfSchoolStrike` / `ChanceOfHealthStrike`
    /// (hidden pointer): 1.0 on strike, otherwise the combined chance of the
    /// group lists (REH §4.9).
    pub fn chance_of_school_strike(&self) -> f32 {
        self.chance_of_strike(true)
    }
    pub fn chance_of_health_strike(&self) -> f32 {
        self.chance_of_strike(false)
    }

    fn chance_of_strike(&self, school: bool) -> f32 {
        if if school {
            self.school_is_on_strike()
        } else {
            self.health_is_on_strike()
        } {
            return 1.0;
        }
        self.combined_strike_chance(Service::of(school))
    }

    // ---- education / health slots (REH) ----
    //
    // Slots 12–15, 19, 20, 26–28, 45 and 58 are in `education_health.rs`.

    // ---- building lists (slots 46–49, 61; REH §4.1) ----

    /// Slots 46 / 48: Σ group list sizes + the pending list's size.
    pub fn school_building_count(&self) -> i32 {
        let s = self.s.borrow();
        list_count(&s.lists.school_groups, &s.lists.pending_school)
    }
    pub fn hospital_building_count(&self) -> i32 {
        let s = self.s.borrow();
        list_count(&s.lists.hospital_groups, &s.lists.pending_hospital)
    }

    /// Slots 47 / 49 `GetSchoolBuildings` / `GetHospitalBuildings(out,
    /// groups)`: the group lists selected by `groups` (all when `None`, none
    /// for an empty slice), then **always** the pending list, each occupant
    /// `AddRef`ed for the caller. A replacement returns true.
    pub fn school_buildings(&self, groups: Option<&[u32]>) -> Vec<u64> {
        let lists = self.s.borrow().lists.clone();
        self.buildings(&lists.school_groups, &group::SCHOOL, &lists.pending_school, groups)
    }
    pub fn hospital_buildings(&self, groups: Option<&[u32]>) -> Vec<u64> {
        let lists = self.s.borrow().lists.clone();
        self.buildings(
            &lists.hospital_groups,
            &group::HOSPITAL,
            &lists.pending_hospital,
            groups,
        )
    }

    fn buildings(
        &self,
        lists: &[Vec<Option<u64>>],
        ids: &[u32],
        pending: &[Option<u64>],
        groups: Option<&[u32]>,
    ) -> Vec<u64> {
        let selected = lists
            .iter()
            .zip(ids)
            .filter(|(_, id)| groups.map_or(true, |g| g.contains(id)))
            .flat_map(|(l, _)| l.iter());
        let out: Vec<u64> = selected.chain(pending.iter()).flatten().copied().collect();
        for &occ in &out {
            self.host.add_ref(occ);
        }
        out
    }

    /// Slot 61 `GetHealthAffectingBuildings(out)`: the health-affecting list
    /// then the other list, each occupant `AddRef`ed.
    pub fn health_affecting_buildings(&self) -> Vec<u64> {
        let out: Vec<u64> = {
            let s = self.s.borrow();
            s.lists
                .health_affecting
                .iter()
                .chain(s.lists.other.iter())
                .flatten()
                .copied()
                .collect()
        };
        for &occ in &out {
            self.host.add_ref(occ);
        }
        out
    }
}

/// The state-0 distribution of one service (REH §4.1): each pending building
/// goes to the first group, in table order, that `IsOccupantGroup` accepts;
/// then the pending list is empty. A building in no group is dropped
/// **without** `Release` *[quirk]* (it leaks one reference); a null entry is
/// dropped ([choice]: the original would dereference it).
fn distribute(h: &dyn ResidentialHost, pending: &mut Vec<Option<u64>>, groups: &mut [Vec<Option<u64>>], ids: &[u32]) {
    for occ in std::mem::take(pending).into_iter().flatten() {
        if let Some(i) = ids.iter().position(|&g| h.is_occupant_group(occ, g)) {
            groups[i].push(Some(occ));
        }
    }
}

/// Slots 46 / 48: Σ list sizes + pending size, as the `int32` EAX.
fn list_count(groups: &[Vec<Option<u64>>], pending: &[Option<u64>]) -> i32 {
    groups.iter().map(Vec::len).sum::<usize>().wrapping_add(pending.len()) as i32
}
