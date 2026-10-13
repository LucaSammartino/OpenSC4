// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4ResidentialSimulator`: the state (§3), the lifecycle (§4), the
//! agent cadence and `DoMessage` (§4.3), the daily state machine
//! (§5.1–§5.2), the proximity maps (§5.3), the histogram (§5.4),
//! `DoPopulationChange` (§5.5), `PostPopulationChange` (§5.6), the history
//! of state 9 (§5.7), the population-half queries (§6) and the interface
//! slots (§10).
//!
//! No `RefCell` borrow is held across a host callback (foundations §19.3
//! rule 4): every grid is an `Rc` cloned out of the state before the host is
//! called, and the accumulators are written back with a short borrow at the
//! end of a tract.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::{GridProvider, ScanIterator, TractGrid, MAX_TRACTS};
use crate::rng::{derive_seed, RzRandom, SeedSource, CLOCK_SEED};
use crate::x87::{ftol32, rhe};

use super::geometry::{copy_proximity, distance_pass, distance_to_convergence, seed_proximity};
use super::tuning::{le_curve, new_ages_by_wealth, workforce_curve, PopulationTuning, SimulatorTuning};
use super::{
    history, instance, msg, state as st, wealth, DepartmentBlock, OccupantLists, QueryData, ResidentialHost,
    ResidentialMessage, ResidentialRandom, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, BUILDING_PURPOSE_RESIDENTIAL,
    BUZZARDS_EFFECT, CLSID_RESIDENTIAL_SIMULATOR, DEFAULT_TRACT_SIZE, DEFAULT_WORK_BUDGET, DEVELOPERS, MAX_QUOTIENT,
    OCCUPANT_TYPE_BUILDING, PROXIMITY_TRACT_SIZE,
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

/// Every field of §3 (persisted or not), the grids and the occupant lists.
#[derive(Clone)]
pub struct ResidentialState {
    pub initialized: bool,
    /// The new-city part of `Init` has run (§4.1 step 6); `Read` sets it.
    pub city_started: bool,
    pub loaded: bool,
    pub ref_count: u32,
    pub destroyed: bool,
    /// The proximity tract size (§3): 1.
    pub proximity_size: i32,
    /// The population tract size P (§2).
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
    pub age_count: [i32; 20],
    pub age_eq_sum: [i32; 20],
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
}

impl Default for ResidentialState {
    /// A constructed object (§3 "Initial").
    fn default() -> ResidentialState {
        ResidentialState {
            initialized: false,
            city_started: false,
            loaded: false,
            ref_count: 0,
            destroyed: false,
            proximity_size: PROXIMITY_TRACT_SIZE,
            population_size: DEFAULT_TRACT_SIZE,
            eq_size: DEFAULT_TRACT_SIZE,
            grids: Grids::default(),
            state: 0,
            dirty: true,
            population: 0,
            running_population: 0,
            le_sum: 0,
            age_count: [0; 20],
            age_eq_sum: [0; 20],
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

/// Whether `QueryInterface` answers `iid` (§1; unknown IIDs → false).
pub fn query_interface(iid: u32) -> bool {
    matches!(
        iid,
        crate::residential::IID_UNKNOWN
            | crate::residential::IID_RESIDENTIAL_SIMULATOR
            | crate::residential::IID_MESSAGE_TARGET2
            | crate::residential::IID_SERIALIZABLE
    )
}

/// The class's shared `cRZRandom` (§3): buzzards (§5.5) and the monthly
/// strike rolls (REH §4.9) draw from one stream.
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
        ResidentialRng(RzRandom::new(derive_seed(session, CLSID_RESIDENTIAL_SIMULATOR), seeds))
    }
}

impl ResidentialRandom for ResidentialRng {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        self.0.range_f64(a, b)
    }
}

/// `min(cap, v)` as the original's `cap < v ? cap : v`: a NaN operand is
/// stored rather than capped (§5.5 step 4, REH §4.5 *[inferred]*).
fn min_cap(cap: f32, v: f32) -> f32 {
    if cap < v {
        cap
    } else {
        v
    }
}

/// `int × float`: one `f32` rounding of the exact product (foundations §18.3
/// point 1: the integer operand enters unrounded).
fn mul_i(i: i64, f: f32) -> f32 {
    (i as f64 * f64::from(f)) as f32
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

/// The residential simulator.
pub struct ResidentialSim {
    host: Rc<dyn ResidentialHost>,
    token: Cell<u64>,
    s: RefCell<ResidentialState>,
    rng: RefCell<Box<dyn ResidentialRandom>>,
    work_budget: Cell<u32>,
}

impl ResidentialSim {
    /// A constructed (not initialised) simulator. `rng` is the class's own
    /// shared `cRZRandom` (§3); [`DEFAULT_WORK_BUDGET`] replaces the 5 ms
    /// CPU-cycle slicer (§5.2 [choice]).
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

    /// A snapshot of the state (tests, diagnostics).
    pub fn snapshot(&self) -> ResidentialState {
        self.s.borrow().clone()
    }

    /// [choice] The work budget that replaces the 5 ms CPU-cycle slicer
    /// (§5.2): one unit is one distance pass (states 1–3) or one tract
    /// (state 4).
    pub fn work_budget(&self) -> u32 {
        self.work_budget.get()
    }

    pub fn set_work_budget(&self, n: u32) {
        self.work_budget.set(n);
    }

    /// The class's shared generator, for a host that drives the strike rolls
    /// itself (REH §4.9). Borrows the generator for the duration of `f`.
    pub fn with_rng<R>(&self, f: impl FnOnce(&mut dyn ResidentialRandom) -> R) -> R {
        f(&mut **self.rng.borrow_mut())
    }

    // ---- lifecycle (§4) ----

    /// `Init` (§4.1). Returns false if the demand node, the message server or
    /// the simulator is missing.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        // Step 1: services, developers and the demand node.
        if !h.acquire_services() || !h.acquire_developers() || !h.acquire_demand_node() {
            return false;
        }
        // Step 2: the message server, then subscribe both occupant messages.
        if !h.acquire_message_server() {
            return false;
        }
        for &m in &msg::SUBSCRIPTIONS {
            h.subscribe(m);
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
        // Step 5: the history queue lengths, interleaved per g (§4.1).
        for g in 0..history::AGE_GROUPS as u32 {
            h.set_history_queue_length(history::AGE_GROUP_POPULATION + g, history::QUEUE_LENGTH);
            h.set_history_queue_length(history::AGE_GROUP_EQ + g, history::QUEUE_LENGTH);
        }
        h.set_history_queue_length(history::SCHOOL_COVERAGE, history::QUEUE_LENGTH);
        h.set_history_queue_length(history::HOSPITAL_COVERAGE, history::QUEUE_LENGTH);

        // Step 6: the new-city part.
        {
            let mut s = self.s.borrow_mut();
            if !s.city_started {
                let p = h
                    .developer_population_map(DEVELOPERS[wealth::LOW])
                    .map(|m| m.geometry().tract_size)
                    .unwrap_or(DEFAULT_TRACT_SIZE);
                s.population_size = p;
                s.eq_size = p;
                s.population = 0;
                s.running_population = 0;
                s.state = 0;
                s.city_started = true;
            }
        }
        // Step 7: the simulator tuning and the ordinance-effect lists, when
        // not yet initialised.
        {
            let initialised = self.s.borrow().initialized;
            if !initialised {
                self.read_simulator_tuning();
                h.refresh_ordinance_effects();
            }
        }
        // Step 8: create every absent grid of §2.
        self.create_grids();
        // Step 9: the departments, when absent.
        self.create_departments();
        // Step 10 (always): the iterator rectangle, the population tuning,
        // leSum := 0, initialised := true.
        {
            let (cx, cz) = {
                let s = self.s.borrow();
                s.grids
                    .population[wealth::LOW]
                    .as_ref()
                    .map(|g| (g.count_x(), g.count_z()))
                    .unwrap_or((0, 0))
            };
            let mut s = self.s.borrow_mut();
            s.iterator = ScanIterator::for_counts(cx, cz);
            s.iterator.reset(0, 0);
            s.le_sum = 0;
            s.initialized = true;
        }
        self.read_population_tuning();
        true
    }

    /// `Shutdown` (§4.2): always returns true.
    pub fn shutdown(&self) -> bool {
        let initialised = self.s.borrow().initialized;
        if !initialised {
            return true;
        }
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            s.initialized = false;
        }
        h.remove_agent();
        for &m in &msg::SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        // Shutdown, release and forget every grid.
        let grids = std::mem::take(&mut self.s.borrow_mut().grids);
        for g in grids.proximity.iter().flatten() {
            h.shutdown_grid_u8(g);
        }
        for g in grids.population.iter().flatten() {
            h.shutdown_grid_u16(g);
        }
        if let Some(g) = &grids.age {
            h.shutdown_grid_u8(g);
        }
        for g in grids.school_coverage.iter().flatten() {
            h.shutdown_grid_u8(g);
        }
        for g in grids.hospital_coverage.iter().flatten() {
            h.shutdown_grid_u8(g);
        }
        for g in [&grids.school_grade, &grids.hospital_grade].into_iter().flatten() {
            h.shutdown_grid_f32(g);
        }
        for g in grids.eq.iter().flatten() {
            h.shutdown_grid_f32(g);
        }
        for g in grids.hq.iter().flatten() {
            h.shutdown_grid_f32(g);
        }
        if let Some(g) = &grids.temporary {
            h.shutdown_grid_u8(g);
        }
        // Hand every occupant list to the occupant manager and clear it.
        let released: Vec<u64> = {
            let mut s = self.s.borrow_mut();
            for list in s.lists.in_save_order() {
                for occ in list.iter().flatten() {
                    h.release(*occ);
                }
                list.clear();
            }
            let mut out = Vec::new();
            out.extend(s.health.department1.take());
            out.extend(s.health.department2.take());
            out.extend(s.education.department1.take());
            out.extend(s.education.department2.take());
            out
        };
        for d in released {
            h.release_department(d);
        }
        h.release_services();
        true
    }

    fn create_grids(&self) {
        let h = &*self.host;
        let (p, e, prox) = {
            let s = self.s.borrow();
            (s.population_size, s.eq_size, s.proximity_size)
        };
        let mut s = self.s.borrow_mut();
        let g = &mut s.grids;
        for (i, slot) in g.proximity.iter_mut().enumerate() {
            if slot.is_none() {
                if let Some(gr) = GridProvider::<u8>::create_grid(h, instance::PROXIMITY[i], prox) {
                    *slot = Some(gr);
                }
            }
        }
        for (i, slot) in g.population.iter_mut().enumerate() {
            if slot.is_none() {
                if let Some(gr) = GridProvider::<u16>::create_grid(h, instance::POPULATION[i], p) {
                    *slot = Some(gr);
                }
            }
        }
        if g.age.is_none() {
            g.age = GridProvider::<u8>::create_grid(h, instance::AGE, p);
        }
        // The coverage vectors: if the first entry is null, every entry is
        // created (§4.1 step 8).
        if g.school_coverage[0].is_none() {
            for (i, slot) in g.school_coverage.iter_mut().enumerate() {
                if slot.is_none() {
                    *slot = GridProvider::<u8>::create_grid(h, instance::SCHOOL_COVERAGE[i], p);
                }
            }
        }
        if g.hospital_coverage[0].is_none() {
            for (i, slot) in g.hospital_coverage.iter_mut().enumerate() {
                if slot.is_none() {
                    *slot = GridProvider::<u8>::create_grid(h, instance::HOSPITAL_COVERAGE[i], p);
                }
            }
        }
        if g.school_grade.is_none() {
            g.school_grade = GridProvider::<f32>::create_grid(h, instance::SCHOOL_GRADE, p);
        }
        if g.hospital_grade.is_none() {
            g.hospital_grade = GridProvider::<f32>::create_grid(h, instance::HOSPITAL_GRADE, p);
        }
        for (i, slot) in g.eq.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = GridProvider::<f32>::create_grid(h, instance::EQ[i], e);
            }
        }
        for (i, slot) in g.hq.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = GridProvider::<f32>::create_grid(h, instance::HQ[i], e);
            }
        }
    }

    fn create_departments(&self) {
        let h = &*self.host;
        // Read the scalars each block needs, then drop the borrow before the
        // host calls (foundations §19.3 rule 4: never hold a lock across a
        // call into the game).
        let (health_max, health_init, education_max, education_init) = {
            let s = self.s.borrow();
            (
                s.health.max_funding,
                s.health.init_funding,
                s.education.max_funding,
                s.education.init_funding,
            )
        };
        let (health_needs, education_needs) = {
            let s = self.s.borrow();
            (
                (s.health.department1.is_none(), s.health.department2.is_none()),
                (s.education.department1.is_none(), s.education.department2.is_none()),
            )
        };
        use super::department as dept;
        // health, then education; each block holds its primary and secondary
        // department (REH §4.2). The secondary starts at 100 % funding.
        let health1 = health_needs
            .0
            .then(|| h.create_department(dept::HEALTH, dept::HEALTH_NAME, health_max, health_init))
            .flatten();
        let health2 = health_needs
            .1
            .then(|| h.create_department(dept::HEALTH_SECONDARY, dept::HEALTH_SECONDARY_NAME, health_max, 1.0))
            .flatten();
        let education1 = education_needs
            .0
            .then(|| h.create_department(dept::EDUCATION, dept::EDUCATION_NAME, education_max, education_init))
            .flatten();
        let education2 = education_needs
            .1
            .then(|| {
                h.create_department(
                    dept::EDUCATION_SECONDARY,
                    dept::EDUCATION_SECONDARY_NAME,
                    education_max,
                    1.0,
                )
            })
            .flatten();
        let mut s = self.s.borrow_mut();
        if let Some(d) = health1 {
            s.health.department1 = Some(d);
        }
        if let Some(d) = health2 {
            s.health.department2 = Some(d);
        }
        if let Some(d) = education1 {
            s.education.department1 = Some(d);
        }
        if let Some(d) = education2 {
            s.education.department2 = Some(d);
        }
    }

    /// §7.2: the simulator tuning, whose unconditional defaults overwrite the
    /// department blocks' scalars (including the strike flags and start days),
    /// so strikes do not survive save and load (§9.3 *[quirk]*).
    fn read_simulator_tuning(&self) {
        let exemplar = self.host.tuning();
        // Take the blocks out so `SimulatorTuning::read` can borrow both
        // mutably at once (an array literal cannot).
        let (mut health, mut education) = {
            let mut s = self.s.borrow_mut();
            (
                std::mem::replace(&mut s.health, DepartmentBlock::code_defaults(msg::HEALTH_STRIKE)),
                std::mem::replace(&mut s.education, DepartmentBlock::code_defaults(msg::SCHOOL_STRIKE)),
            )
        };
        let tuning = SimulatorTuning::read(exemplar.as_deref(), &mut health, &mut education);
        let mut s = self.s.borrow_mut();
        s.health = health;
        s.education = education;
        s.simulator_tuning = tuning;
    }

    /// §7.1: the population tuning, with the two decays scaled by the
    /// ordinance products (REH §4.3).
    fn read_population_tuning(&self) {
        let exemplar = self.host.tuning();
        let (edu, health) = self.host.ordinance_decay_products();
        let t = PopulationTuning::read(exemplar.as_deref(), edu, health);
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
            msg::SIM_BEGIN | msg::SIM_IDLE => true,
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if m.data3 == OCCUPANT_TYPE_BUILDING {
                    if let Some(occ) = m.void1 {
                        if let Some(building) = self.host.occupant_as_building(occ) {
                            let inserted = m.msg_type == msg::OCCUPANT_INSERTED;
                            let mut lists = std::mem::take(&mut self.s.borrow_mut().lists);
                            self.host.building_changed(building, inserted, &mut lists);
                            {
                                let mut s = self.s.borrow_mut();
                                s.lists = lists;
                                s.dirty = true;
                            }
                        }
                    }
                }
                true
            }
            _ => true,
        }
    }

    /// One step of the daily state machine (§5.1). At most one state per
    /// call; returns false only when a time-sliced state (1, 2, 3, 4) runs
    /// out of its work budget (§5.2).
    pub fn do_daily_simulation(&self) -> bool {
        let mut budget = self.work_budget.get();
        let state = self.s.borrow().state;
        match state {
            st::STRIKES_AND_LISTS => {
                self.state_0();
                self.set_state(st::SEED_LOW);
                true
            }
            st::SEED_LOW => {
                self.seed_proximity(wealth::LOW);
                self.set_state(st::PROXIMITY_LOW);
                true
            }
            st::PROXIMITY_LOW => self.proximity_state(wealth::LOW, st::SEED_MEDIUM, &mut budget),
            st::SEED_MEDIUM => {
                self.seed_proximity(wealth::MEDIUM);
                self.set_state(st::PROXIMITY_MEDIUM);
                true
            }
            st::PROXIMITY_MEDIUM => self.proximity_state(wealth::MEDIUM, st::SEED_HIGH, &mut budget),
            st::SEED_HIGH => {
                // Seeds R$$$ and runs the full transform at once (§5.3).
                self.seed_proximity(wealth::HIGH);
                self.transform_temporary();
                self.set_state(st::PROXIMITY_HIGH);
                true
            }
            st::PROXIMITY_HIGH => {
                // One pass (no change), copy, free the temporary grid.
                self.finish_proximity(wealth::HIGH);
                self.set_state(st::HISTOGRAM_RESET);
                true
            }
            st::HISTOGRAM_RESET => {
                self.state_11();
                self.set_state(st::POPULATION);
                true
            }
            st::POPULATION => self.state_4(&mut budget),
            st::POST_POPULATION => {
                self.post_population_change();
                self.set_state(st::SCHOOL_COVERAGE);
                true
            }
            st::SCHOOL_COVERAGE => {
                self.set_school_covered(0);
                let served = self.host.update_school_coverage();
                self.set_school_covered(served);
                self.set_state(st::SCHOOL_GRADING);
                true
            }
            st::SCHOOL_GRADING => {
                self.zero_grade_grid(true);
                self.host.grade_schools();
                self.set_state(st::HOSPITAL_COVERAGE);
                true
            }
            st::HOSPITAL_COVERAGE => {
                self.set_hospital_covered(0);
                let served = self.host.update_hospital_coverage();
                self.set_hospital_covered(served);
                self.set_state(st::HOSPITAL_GRADING);
                true
            }
            st::HOSPITAL_GRADING => {
                self.zero_grade_grid(false);
                self.host.grade_hospitals();
                self.set_state(st::HISTORY);
                true
            }
            st::HISTORY => {
                self.state_9();
                self.set_state(st::IDLE);
                true
            }
            st::IDLE => true,
            _ => true,
        }
    }

    fn set_state(&self, state: u32) {
        self.s.borrow_mut().state = state;
    }

    /// State 0 (§5.1, REH §3): the strike checks, the ordinance-effect
    /// refresh and the pending building lists.
    fn state_0(&self) {
        let h = &*self.host;
        // The strike rolls draw from the class's shared generator, in
        // group-list order (REH §4.9).
        self.with_rng(|rng| h.strike_checks(rng));
        h.refresh_ordinance_effects();
        let mut lists = std::mem::take(&mut self.s.borrow_mut().lists);
        h.distribute_pending_buildings(&mut lists);
        self.s.borrow_mut().lists = lists;
    }

    /// Create the temporary grid if absent and seed it for wealth `w`
    /// (§5.3 step 1). States 13/14/15 create it only if it does not exist, so
    /// R$$ and R$$$ reuse the R$ grid (the seed overwrites every tract).
    fn seed_proximity(&self, w: usize) {
        let h = &*self.host;
        let key = h.building_map_key(BUILDING_PURPOSE_RESIDENTIAL, wealth::NUMBERS[w]);
        let map = h.building_map();
        let prox = self.s.borrow().proximity_size;
        // Create the temporary grid if absent (states 13/14/15 create it only
        // if it does not exist, so R$$ and R$$$ reuse the R$ grid).
        {
            let mut s = self.s.borrow_mut();
            if s.grids.temporary.is_none() {
                s.grids.temporary = GridProvider::<u8>::create_grid(h, 0, prox);
            }
        }
        let temp = self.s.borrow().grids.temporary.clone();
        let Some(temp) = temp else { return };
        match map {
            Some(map) => seed_proximity(&*temp, &*map, key),
            // No building map: no key tract, so every value is FAR (§5.3).
            None => temp.fill(super::geometry::FAR),
        }
    }

    /// Run the distance passes on the temporary grid to convergence (§5.3
    /// step 2), not time-sliced. Used by state 15.
    fn transform_temporary(&self) {
        let temp = self.s.borrow().grids.temporary.clone();
        if let Some(t) = temp {
            let (cx, cz) = (t.count_x(), t.count_z());
            distance_to_convergence(&*t, cx, cz);
        }
    }

    /// A time-sliced proximity state (1 or 2): run passes until convergence
    /// or the budget runs out, then copy into the wealth's proximity grid and
    /// advance (§5.1, §5.2, §5.3). Returns "finished".
    fn proximity_state(&self, w: usize, next: u32, budget: &mut u32) -> bool {
        let temp = self.s.borrow().grids.temporary.clone();
        let Some(temp) = temp else {
            // No temporary grid: nothing to transform; advance.
            self.set_state(next);
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
        // Copy into proximity grid w (§5.3 step 3).
        let prox = self.s.borrow().grids.proximity[w].clone();
        if let Some(p) = prox {
            copy_proximity(&*p, &*temp);
        }
        self.set_state(next);
        true
    }

    /// State 3 (§5.3): one pass (no change), copy into proximity R$$$, then
    /// `Shutdown` and release the temporary grid.
    fn finish_proximity(&self, w: usize) {
        let h = &*self.host;
        let temp = self.s.borrow().grids.temporary.clone();
        if let Some(t) = &temp {
            let (cx, cz) = (t.count_x(), t.count_z());
            distance_pass(&**t, cx, cz);
            let prox = self.s.borrow().grids.proximity[w].clone();
            if let Some(p) = prox {
                copy_proximity(&*p, &**t);
            }
            h.shutdown_grid_u8(t);
        }
        self.s.borrow_mut().grids.temporary = None;
    }

    /// State 11 (§5.4): reset the histogram, leSum, the running population
    /// and the tuning-loaded flag.
    fn state_11(&self) {
        let mut s = self.s.borrow_mut();
        s.age_count = [0; 20];
        s.age_eq_sum = [0; 20];
        s.le_sum = 0;
        s.running_population = 0;
        s.tuning_loaded = false;
    }

    /// State 4 (§5.5): `DoPopulationChange` for every tract of the scanning
    /// iterator (time-sliced). Returns "finished".
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
                self.set_state(st::POST_POPULATION);
                return true;
            };
            *budget -= 1;
            self.do_population_change(x, z);
        }
    }

    /// `DoPopulationChange(x, z)` for one tract (§5.5). All float steps are
    /// `f32` operations in the order the spec gives (foundations §18).
    fn do_population_change(&self, x: i32, z: i32) {
        let h = &*self.host;
        // Clone the grids and read the scalars this tract needs, so no state
        // borrow is held across a host callback.
        let (p_size, t, eq, hq, pop, age_grid) = {
            let s = self.s.borrow();
            (
                s.population_size,
                s.population_tuning.clone(),
                s.grids.eq.clone(),
                s.grids.hq.clone(),
                s.grids.population.clone(),
                s.grids.age.clone(),
            )
        };
        let s_size = p_size;

        // Step 1: pollution → hqLoss.
        let hq_loss = {
            let mut avg = [0u32; 3];
            let cells = s_size.max(0);
            let denom = (cells * cells) as u32;
            if denom != 0 {
                for (q, slot) in avg.iter_mut().enumerate() {
                    let mut sum = 0u32;
                    for i in 0..cells {
                        for j in 0..cells {
                            let v = h.pollution_value(q as u32, s_size * x + i, s_size * z + j);
                            sum = sum.wrapping_add(u32::from(v));
                        }
                    }
                    *slot = sum / denom;
                }
            }
            // The caller keeps only the low byte [quirk].
            let p = [avg[0] & 0xFF, avg[1] & 0xFF, avg[2] & 0xFF];
            let mut loss = t.hq_decay;
            for (curve, index) in t.pollution_curves.iter().zip(p) {
                loss += curve.eval(index as f32);
            }
            loss
        };

        // Step 2: decay and buzzards, one flag per tract.
        let mut buzzard = false;
        for w in 0..wealth::COUNT {
            let eq_g = &eq[w];
            let hq_g = &hq[w];
            let pop_g = &pop[w];
            // EQ decay.
            if let Some(g) = eq_g {
                let v = g.get(x, z);
                let d = v - t.eq_decay;
                g.set(x, z, if d < 0.0 { 0.0 } else { d });
            }
            // HQ decay.
            let hq_after = if let Some(g) = hq_g {
                let v = g.get(x, z);
                let d = v - hq_loss;
                let r = if d < 0.0 { 0.0 } else { d };
                g.set(x, z, r);
                r
            } else {
                0.0
            };
            // Buzzards: HQ below the door, not yet spawned this tract, and
            // the current population non-zero (before this pass's update).
            let cur_pop = pop_g.as_ref().map(|g| g.get(x, z)).unwrap_or(0);
            if hq_after < t.deaths_door_hq && !buzzard && cur_pop != 0 {
                let r = self.rng.borrow_mut().range_f64(0.0, 100.0);
                if r < f64::from(t.effect_chance) {
                    if let Some(effect) = h.create_visual_effect(BUZZARDS_EFFECT) {
                        // wx = float(width × x) + width × 0.5 (two f32 steps).
                        let width = eq_g
                            .as_ref()
                            .map(|g| g.geometry().width_x)
                            .unwrap_or(0.0);
                        let width_z = eq_g
                            .as_ref()
                            .map(|g| g.geometry().width_z)
                            .unwrap_or(0.0);
                        let wx = mul_i(i64::from(x), width) + mul_d(width, 0.5);
                        let wz = mul_i(i64::from(z), width_z) + mul_d(width_z, 0.5);
                        let alt = h.altitude(wx, wz);
                        let dy = self.rng.borrow_mut().range_f64(
                            -f64::from(t.effect_delta),
                            f64::from(t.effect_delta),
                        );
                        let y = (f64::from(alt + t.effect_height) + dy) as f32;
                        h.set_effect_transform(effect, [wx, y, wz]);
                        h.start_and_release_effect(effect);
                        buzzard = true;
                    }
                    // No effect: nothing else happens, and a later wealth
                    // level of the same tract may roll again (buzzard stays
                    // false).
                }
            }
        }

        // Step 3: read all three new populations before any write.
        let mut new_pop = [0u16; wealth::COUNT];
        for w in 0..wealth::COUNT {
            if let Some(m) = h.developer_population_map(DEVELOPERS[w]) {
                // The grid is int16; this read uses the 16 bits as unsigned.
                new_pop[w] = m.get(x, z) as u16;
            }
        }

        // Step 4: population, newcomers' EQ/HQ blend.
        let mut arrived = false;
        let mut total = 0i32;
        let mut old_sum = 0i64;
        let mut nc_age_sum = 0i64;
        let mut new_sum = 0i64;
        let mut new_populations = [0u16; wealth::COUNT];
        for w in 0..wealth::COUNT {
            let new_w = new_pop[w];
            new_populations[w] = new_w;
            total = total.wrapping_add(i32::from(new_w));
            let pop_g = &pop[w];
            let old_w = pop_g.as_ref().map(|g| g.get(x, z)).unwrap_or(0);
            if let Some(g) = pop_g {
                g.set(x, z, new_w);
            }
            old_sum += i64::from(old_w);
            new_sum += i64::from(new_w);
            let nc_w = if old_w < new_w {
                let nc = new_w - old_w;
                // Blend EQ then HQ for the newcomers (REH §4.7 step 4).
                if let (Some(eq_g), Some(hq_g)) = (&eq[w], &hq[w]) {
                    let eq_old = eq_g.get(x, z);
                    let hq_old = hq_g.get(x, z);
                    let delta = i64::from(nc);
                    let old = i64::from(old_w);
                    let new = i64::from(new_w);
                    let blend = |q_old: f32, bonus: f32| -> f32 {
                        let m = if q_old > 0.0 { 2 } else { 1 };
                        let avg = div_i(q_old + bonus, m);
                        let num = mul_i(delta, avg) + mul_i(old, q_old);
                        min_cap(MAX_QUOTIENT as f32, div_i(num, new))
                    };
                    eq_g.set(x, z, blend(eq_old, t.new_eq[w]));
                    hq_g.set(x, z, blend(hq_old, t.new_hq[w]));
                }
                nc_age_sum += i64::from(nc) * i64::from(t.new_age[w]);
                arrived = true;
                nc
            } else {
                0
            };
            let _ = nc_w;
            if new_w == 0 {
                if let Some(g) = &eq[w] {
                    g.set(x, z, 0.0);
                }
                if let Some(g) = &hq[w] {
                    g.set(x, z, 0.0);
                }
            }
        }

        // Step 5: ageing.
        let mut age = i32::from(age_grid.as_ref().map(|g| g.get(x, z)).unwrap_or(0));
        if total != 0 {
            if h.sim_month() == 12 {
                age = (age + 1) & 0xFF;
            }
        } else {
            age = 0;
        }

        // Step 6: the newcomers' age [quirk: old populations of every level].
        if arrived && new_sum != 0 {
            let num = old_sum * i64::from(age) + nc_age_sum;
            age = (num / new_sum) as i32;
        }

        // Step 7: life expectancy, after the EQ/HQ updates.
        let le = i32::from(self.get_cell_life_expectancy(s_size * x, s_size * z));

        // Step 8: generational reset.
        if age > le {
            for w in 0..wealth::COUNT {
                if let Some(g) = &eq[w] {
                    let v = g.get(x, z);
                    g.set(x, z, v * t.eq_reset);
                }
                if let Some(g) = &hq[w] {
                    let v = g.get(x, z);
                    g.set(x, z, v * t.hq_reset);
                }
            }
            age = t.reset_age;
        }

        // Step 9: store the low byte of the age.
        let age_byte = (age & 0xFF) as u8;
        if let Some(g) = &age_grid {
            g.set(x, z, age_byte);
        }

        // Step 11: the histogram bucket (uses the post-reset age).
        let mut b = (age - 20) / 3;
        if (b as u32) >= 20 {
            b = 19;
        }
        let bi = b as usize;

        // Steps 10 and 11 accumulate; write them back with a short borrow.
        // leSum += LE × total (32-bit product, sign-extended to 64 bits).
        let le_total = (le.wrapping_mul(total)) as i32;
        let mut dcount = 0i32;
        let mut deqsum = 0i32;
        for w in 0..wealth::COUNT {
            let pw = i32::from(new_populations[w]);
            dcount = dcount.wrapping_add(pw);
            let eqv = eq[w].as_ref().map(|g| g.get(x, z)).unwrap_or(0.0);
            deqsum = deqsum.wrapping_add(ftol32(f64::from(eqv)).wrapping_mul(pw));
        }
        let mut s = self.s.borrow_mut();
        s.running_population = s.running_population.wrapping_add(total);
        s.le_sum = s.le_sum.wrapping_add(i64::from(le_total) as u64);
        s.age_count[bi] = s.age_count[bi].wrapping_add(dcount);
        s.age_eq_sum[bi] = s.age_eq_sum[bi].wrapping_add(deqsum);
    }

    /// State 12 (§5.6): `PostPopulationChange`.
    fn post_population_change(&self) {
        // Reset the iterator (§5.1 state 12).
        self.s.borrow_mut().iterator.reset(0, 0);
        let (population, le_sum, age_count, age_eq_sum, curves) = {
            let s = self.s.borrow();
            (
                s.running_population,
                s.le_sum,
                s.age_count,
                s.age_eq_sum,
                s.simulator_tuning.age_group_curves.clone(),
            )
        };
        // Step 1: population := running population.
        self.s.borrow_mut().population = population;
        // Step 2: the population-weighted average life expectancy.
        let avg_le = if population > 0 {
            (le_sum / population as u64) as i32
        } else {
            0
        };
        self.record_history(history::AVERAGE_LIFE_EXPECTANCY, avg_le);
        // Step 3: the per-bucket average EQ.
        let mut avg = [0.0f32; 20];
        for b in 0..20 {
            let (c, e) = (age_count[b] as u32, age_eq_sum[b] as u32);
            avg[b] = if c > 0 {
                (f64::from(e) / f64::from(c)) as f32
            } else {
                0.0
            };
        }
        // Step 4: the age groups.
        for g in 0..history::AGE_GROUPS {
            let curve = &curves[g];
            let mut pop_sum = 0i64;
            let mut eq_sum = 0i64;
            for b in 0..20 {
                if (age_count[b] as u32) > 0 {
                    // n := trunc((curve(21 + 3b) × ageCount[b]) × 0.01).
                    let age_input = (21 + 3 * b) as f32;
                    let cv = curve.eval(age_input);
                    let scaled = mul_i(i64::from(age_count[b] as u32 as i32), cv);
                    let n = ftol32(f64::from(mul_d(scaled, 0.01)));
                    pop_sum += i64::from(n);
                    // eqSum := trunc(float(n) × avg_b + float(eqSum)), an
                    // integer accumulator.
                    let term = mul_i(i64::from(n), avg[b]) + (eq_sum as i32 as f32);
                    eq_sum = i64::from(ftol32(f64::from(term)));
                }
            }
            self.record_history(history::AGE_GROUP_POPULATION + g as u32, pop_sum as i32);
            let group_eq = if pop_sum > 0 {
                (eq_sum / pop_sum) as i32
            } else {
                0
            };
            self.record_history(history::AGE_GROUP_EQ + g as u32, group_eq);
        }
    }

    /// State 9 (§5.7): the history.
    fn state_9(&self) {
        // In this order: global EQ, global HQ, population.
        let geq = self.get_global_eq();
        let ghq = self.get_global_hq();
        let population = self.s.borrow().population;
        self.record_history(history::GLOBAL_EQ, ftol32(f64::from(geq)));
        self.record_history(history::GLOBAL_HQ, ftol32(f64::from(ghq)));
        self.record_history(history::POPULATION, population);
        let (school, hospital) = {
            let s = self.s.borrow();
            (s.school_covered, s.hospital_covered)
        };
        if population > 0 {
            // h is computed first, then school, then hospital (§5.7 [quirk]:
            // the two branches write 0x4A8A3FE2 / 0x4A8A3FE5 in opposite
            // orders).
            let pop = f64::from(population);
            let hq_permille = rhe(mul_d((f64::from(hospital) / pop) as f32, 1000.0));
            let school_permille = rhe(mul_d((f64::from(school) / pop) as f32, 1000.0));
            self.record_history(history::SCHOOL_COVERAGE, school_permille);
            self.record_history(history::HOSPITAL_COVERAGE, hq_permille);
        } else {
            self.record_history(history::HOSPITAL_COVERAGE, 0);
            self.record_history(history::SCHOOL_COVERAGE, 0);
        }
    }

    fn record_history(&self, series: u32, value: i32) {
        self.host.record_history(series, value);
    }

    fn set_school_covered(&self, v: i32) {
        self.s.borrow_mut().school_covered = v;
    }

    fn set_hospital_covered(&self, v: i32) {
        self.s.borrow_mut().hospital_covered = v;
    }

    fn zero_grade_grid(&self, school: bool) {
        let g = if school {
            self.s.borrow().grids.school_grade.clone()
        } else {
            self.s.borrow().grids.hospital_grade.clone()
        };
        if let Some(g) = g {
            g.fill(0.0);
        }
    }

    // ---- queries (§6) ----

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
    fn tract_quotient(values: &[Option<Rc<dyn TractGrid<f32>>>; 3], pops: &[Option<Rc<dyn TractGrid<u16>>>; 3], x: i32, z: i32) -> f32 {
        let p = [
            pops[0].as_ref().map(|g| g.get(x, z)).unwrap_or(0),
            pops[1].as_ref().map(|g| g.get(x, z)).unwrap_or(0),
            pops[2].as_ref().map(|g| g.get(x, z)).unwrap_or(0),
        ];
        // Σp in 16 bits [quirk].
        let sum16 = (p[0] as u32 + p[1] as u32 + p[2] as u32) as u16;
        if sum16 == 0 {
            return 0.0;
        }
        let v = [
            values[0].as_ref().map(|g| g.get(x, z)).unwrap_or(0.0),
            values[1].as_ref().map(|g| g.get(x, z)).unwrap_or(0.0),
            values[2].as_ref().map(|g| g.get(x, z)).unwrap_or(0.0),
        ];
        // s := q2·p2; s := q1·p1 + s; result := (q0·p0 + s) / float(sum16).
        let mut s = mul_i(i64::from(p[2]), v[2]);
        s = mul_i(i64::from(p[1]), v[1]) + s;
        s = mul_i(i64::from(p[0]), v[0]) + s;
        div_i(s, i64::from(sum16))
    }

    /// Slot 35 `GetCellEQ(x, z)` (REH §4.8): the quotient at the EQ tract.
    pub fn get_cell_eq(&self, x: i32, z: i32) -> f32 {
        let (eq, pop, e) = {
            let s = self.s.borrow();
            (s.grids.eq.clone(), s.grids.population.clone(), s.eq_size)
        };
        let (tx, tz) = (unsigned_div_tract(x, e), unsigned_div_tract(z, e));
        Self::tract_quotient(&eq, &pop, tx, tz)
    }

    /// Slot 36 `GetCellHQ(x, z)`.
    pub fn get_cell_hq(&self, x: i32, z: i32) -> f32 {
        let (hq, pop, e) = {
            let s = self.s.borrow();
            (s.grids.hq.clone(), s.grids.population.clone(), s.eq_size)
        };
        let (tx, tz) = (unsigned_div_tract(x, e), unsigned_div_tract(z, e));
        Self::tract_quotient(&hq, &pop, tx, tz)
    }

    /// Slots 37 / 38 `GetCellEQByWealth` / `GetCellHQByWealth`: the wealth
    /// byte decremented, so wealth 0 or > 3 reads out of bounds in the
    /// original *[quirk]*; FreeCity4 returns 0.0 ([choice]).
    pub fn get_cell_eq_by_wealth(&self, x: i32, z: i32, w: u8) -> f32 {
        self.cell_by_wealth(true, x, z, w)
    }
    pub fn get_cell_hq_by_wealth(&self, x: i32, z: i32, w: u8) -> f32 {
        self.cell_by_wealth(false, x, z, w)
    }

    fn cell_by_wealth(&self, is_eq: bool, x: i32, z: i32, w: u8) -> f32 {
        let idx = (w as u32).wrapping_sub(1) as usize;
        if idx >= wealth::COUNT {
            return 0.0;
        }
        let (grids, e) = {
            let s = self.s.borrow();
            (
                if is_eq { s.grids.eq.clone() } else { s.grids.hq.clone() },
                s.eq_size,
            )
        };
        grids[idx]
            .as_ref()
            .map(|g| g.get(unsigned_div_tract(x, e), unsigned_div_tract(z, e)))
            .unwrap_or(0.0)
    }

    /// Slots 32 / 33 `GetGlobalEQ` / `GetGlobalHQ` (REH §4.8).
    pub fn get_global_eq(&self) -> f32 {
        self.global_quotient(true)
    }
    pub fn get_global_hq(&self) -> f32 {
        self.global_quotient(false)
    }

    fn global_quotient(&self, is_eq: bool) -> f32 {
        let population = self.s.borrow().population;
        if population == 0 {
            return 0.0;
        }
        let (grids, pop, counts) = {
            let s = self.s.borrow();
            let g = if is_eq { s.grids.eq.clone() } else { s.grids.hq.clone() };
            let counts = g[0]
                .as_ref()
                .map(|gr| (gr.count_x(), gr.count_z()))
                .unwrap_or((0, 0));
            (g, s.grids.population.clone(), counts)
        };
        let mut sum = 0.0f32;
        for x in 0..counts.0.clamp(0, MAX_TRACTS) {
            for z in 0..counts.1.clamp(0, MAX_TRACTS) {
                // a := Q0·p0; b := Q1·p1 + a; c := Q2·p2 + b; sum := c + sum.
                let p = |w: usize| i64::from(pop[w].as_ref().map(|g| g.get(x, z)).unwrap_or(0));
                let q = |w: usize| grids[w].as_ref().map(|g| g.get(x, z)).unwrap_or(0.0);
                let a = mul_i(p(0), q(0));
                let b = mul_i(p(1), q(1)) + a;
                let c = mul_i(p(2), q(2)) + b;
                sum = c + sum;
            }
        }
        div_i(sum, i64::from(population))
    }

    /// Slots 21 / 22 `GetAverageEQGrid` / `GetAverageHQGrid` (§6.2): a **new**
    /// float grid per call, owned by the caller, plus its min and max. min
    /// starts at 255.0 and max at 0.0, each updated with every tract (REH
    /// §4.8, RS6).
    pub fn average_quotient_grid(&self, is_eq: bool) -> (Option<Rc<dyn TractGrid<f32>>>, f32, f32) {
        let h = &*self.host;
        let e = self.s.borrow().eq_size;
        let id = if is_eq { instance::AVERAGE_EQ } else { instance::AVERAGE_HQ };
        let Some(grid) = GridProvider::<f32>::create_grid(h, id, e) else {
            return (None, 255.0, 0.0);
        };
        let (eq, pop) = {
            let s = self.s.borrow();
            (
                if is_eq { s.grids.eq.clone() } else { s.grids.hq.clone() },
                s.grids.population.clone(),
            )
        };
        let mut min = 255.0f32;
        let mut max = 0.0f32;
        let (cx, cz) = (grid.count_x(), grid.count_z());
        for x in 0..cx.clamp(0, MAX_TRACTS) {
            for z in 0..cz.clamp(0, MAX_TRACTS) {
                let q = Self::tract_quotient(&eq, &pop, x, z);
                let v = ftol32(f64::from(q)) as f32;
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
    /// (§6.3): the three grids, each `AddRef`ed. Returns the third `AddRef`'s
    /// reference count (a replacement returns 1).
    pub fn eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.s.borrow().grids.eq.clone()
    }
    pub fn hq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.s.borrow().grids.hq.clone()
    }
    pub fn population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3] {
        self.s.borrow().grids.population.clone()
    }
    pub fn add_ref_eq_grids(&self) -> u32 {
        let g = self.eq_grids();
        let mut last = 1;
        for gr in g.iter().flatten() {
            last = self.host.grid_add_ref_f32(gr);
        }
        last
    }
    pub fn add_ref_population_grids(&self) -> u32 {
        let g = self.population_grids();
        let mut last = 1;
        for gr in g.iter().flatten() {
            last = self.host.grid_add_ref_u16(gr);
        }
        last
    }

    /// Slot 29 `GetCellLifeExpectancy(x, z)` (§6.4): the HQ→LE curve at
    /// `GetCellHQ`, truncated, AL. No exemplar → 0.
    pub fn get_cell_life_expectancy(&self, x: i32, z: i32) -> u8 {
        let Some(curve) = le_curve(self.host.tuning().as_deref()) else {
            return 0;
        };
        let hq = self.get_cell_hq(x, z);
        (ftol32(f64::from(curve.eval(hq))) & 0xFF) as u8
    }

    /// Slot 34 `GetGlobalLE()`: the same curve at `GetGlobalHQ`, AL.
    pub fn get_global_le(&self) -> u8 {
        let Some(curve) = le_curve(self.host.tuning().as_deref()) else {
            return 0;
        };
        let hq = self.get_global_hq();
        (ftol32(f64::from(curve.eval(hq))) & 0xFF) as u8
    }

    /// Slot 30 `GetCellWorkforcePercent(x, z)` (§6.4): the LE→workforce curve
    /// at `float(slot 29)`, × 0.01. No exemplar → 0.0 *[inferred]*.
    pub fn get_cell_workforce_percent(&self, x: i32, z: i32) -> f32 {
        let Some(curve) = workforce_curve(self.host.tuning().as_deref()) else {
            return 0.0;
        };
        let le = self.get_cell_life_expectancy(x, z);
        let v = curve.eval(le as f32);
        mul_d(v, 0.01)
    }

    /// Slot 31 `GetGlobalWorkforcePercent()`.
    pub fn get_global_workforce_percent(&self) -> f32 {
        let Some(curve) = workforce_curve(self.host.tuning().as_deref()) else {
            return 0.0;
        };
        let le = self.get_global_le();
        let v = curve.eval(le as f32);
        mul_d(v, 0.01)
    }

    /// Slot 42 `GetAverageNewAgeByWealth(w)` (§6.5): `ages[(w − 1) mod 256]`
    /// in AL. w ∉ {1,2,3} returns 0 ([choice]; the original reads out of
    /// bounds).
    pub fn get_average_new_age_by_wealth(&self, w: u8) -> u8 {
        let ages = new_ages_by_wealth(self.host.tuning().as_deref());
        let idx = (w as u32).wrapping_sub(1) & 0xFF;
        match idx {
            0 => ages[0],
            1 => ages[1],
            2 => ages[2],
            _ => 0,
        }
    }

    /// Slots 50 / 51 `GetMaxEQ` / `GetMaxHQ`: 200.
    pub fn max_quotient(&self) -> i32 {
        MAX_QUOTIENT
    }

    /// Slots 52–55: the global auto-budget flags.
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
    /// property `0x6C05FC4D` (§6.6).
    pub fn get_auto_budget(&self, occupant: u64) -> bool {
        self.host
            .occupant_bool_property(occupant, super::prop_auto_budget())
            .unwrap_or(false)
    }
    pub fn set_auto_budget(&self, occupant: u64, v: bool) -> bool {
        self.host
            .set_occupant_bool_property(occupant, super::prop_auto_budget(), v)
    }

    // ---- education / health slots delegated to the host (REH) ----

    pub fn school_is_on_strike(&self) -> bool {
        self.host.school_is_on_strike()
    }
    pub fn health_is_on_strike(&self) -> bool {
        self.host.health_is_on_strike()
    }
    pub fn end_school_strike(&self) -> bool {
        self.host.end_school_strike()
    }
    pub fn end_health_strike(&self) -> bool {
        self.host.end_health_strike()
    }
    pub fn chance_of_school_strike(&self) -> f32 {
        self.host.chance_of_school_strike()
    }
    pub fn chance_of_health_strike(&self) -> f32 {
        self.host.chance_of_health_strike()
    }
    pub fn school_system_rating(&self) -> f32 {
        self.host.school_system_rating()
    }
    pub fn health_system_rating(&self) -> f32 {
        self.host.health_system_rating()
    }
    pub fn school_system_totals(&self, groups: &[u32]) -> u32 {
        self.host.school_system_totals(groups)
    }
    pub fn hospital_system_totals(&self, groups: &[u32]) -> u32 {
        self.host.hospital_system_totals(groups)
    }
    /// Slot 19 (§10.1: also sets the dirty flag; a replacement returns true).
    pub fn set_occupant_funding(&self, occ: u64, staff: f32, vehicle: f32, is_school: bool) -> bool {
        let r = self.host.set_occupant_funding(occ, staff, vehicle, is_school);
        self.s.borrow_mut().dirty = true;
        let _ = r;
        true
    }
    /// Slot 20: `(staff, vehicle)`; a replacement returns true.
    pub fn occupant_funding(&self, occ: u64, is_school: bool) -> (f32, f32) {
        self.host.occupant_funding(occ, is_school)
    }
    pub fn school_query_data(&self, occ: u64) -> Option<QueryData> {
        self.host.school_query_data(occ)
    }
    pub fn hospital_query_data(&self, occ: u64) -> Option<QueryData> {
        self.host.hospital_query_data(occ)
    }
    pub fn estimate_occupant_capacity(&self, occ: u64) -> (u32, u32) {
        self.host.estimate_occupant_capacity(occ)
    }
    pub fn occupant_coverage(&self, occ: u64, level: f32) -> Option<(f32, f32)> {
        self.host.occupant_coverage(occ, level)
    }
    pub fn school_buildings(&self, groups: Option<&[u32]>) -> Vec<u64> {
        let mut out = Vec::new();
        self.host.school_buildings(groups, &mut out);
        out
    }
    pub fn hospital_buildings(&self, groups: Option<&[u32]>) -> Vec<u64> {
        let mut out = Vec::new();
        self.host.hospital_buildings(groups, &mut out);
        out
    }
    pub fn school_building_count(&self) -> i32 {
        self.school_buildings(None).len() as i32
    }
    pub fn hospital_building_count(&self) -> i32 {
        self.hospital_buildings(None).len() as i32
    }
    pub fn estimate_ideal_funding(&self, occ: u64) -> Option<f32> {
        self.host.estimate_ideal_funding(occ)
    }
    pub fn health_affecting_buildings(&self) -> Vec<u64> {
        let mut out = Vec::new();
        self.host.health_affecting_buildings(&mut out);
        out
    }

    /// Slots 43 / 44 `GetEQMinAndMaxCellCoords` / `GetHQMinAndMaxCellCoords`
    /// (REH §4.8): `Some((minX, minZ, maxX, maxZ, min, max))`, or `None` when
    /// the population is 0 (all six outputs 0, AL = 0).
    pub fn quotient_min_max(&self, is_eq: bool) -> Option<(u32, u32, u32, u32, f32, f32)> {
        if self.s.borrow().population == 0 {
            return None;
        }
        let (grids, pop, e, counts) = {
            let s = self.s.borrow();
            let g = if is_eq { s.grids.eq.clone() } else { s.grids.hq.clone() };
            let counts = g[0]
                .as_ref()
                .map(|gr| (gr.count_x(), gr.count_z()))
                .unwrap_or((0, 0));
            (g, s.grids.population.clone(), s.eq_size, counts)
        };
        let mut min = 200.0f32;
        let mut max = -200.0f32;
        let (mut min_x, mut min_z, mut max_x, mut max_z) = (0i32, 0i32, 0i32, 0i32);
        for x in 0..counts.0.clamp(0, MAX_TRACTS) {
            for z in 0..counts.1.clamp(0, MAX_TRACTS) {
                let any = (0..wealth::COUNT).any(|w| pop[w].as_ref().map(|g| g.get(x, z)).unwrap_or(0) != 0);
                if !any {
                    continue;
                }
                let q = Self::tract_quotient(&grids, &pop, x, z);
                if q < min {
                    min = q;
                    min_x = x;
                    min_z = z;
                }
                if q > max {
                    max = q;
                    max_x = x;
                    max_z = z;
                }
            }
        }
        // Coordinates out := S_EQ/2 + S_EQ × tract (signed /2).
        let coord = |t: i32| (e / 2 + e * t) as u32;
        Some((
            coord(min_x),
            coord(min_z),
            coord(max_x),
            coord(max_z),
            min,
            max,
        ))
    }
}

/// `x / S` with an unsigned division (REH §4.8: the wealth byte and the
/// coordinates divide as unsigned); S ≤ 0 gives 0 ([choice]).
fn unsigned_div_tract(v: i32, s: i32) -> i32 {
    if s <= 0 {
        0
    } else {
        ((v as u32) / (s as u32)) as i32
    }
}
