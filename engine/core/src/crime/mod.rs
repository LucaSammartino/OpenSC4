// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The crime simulator (`docs/specs/crime.md`, `cSC4CrimeSimulator`, class
//! `0x8990C09A`). Section references (`§n`) point at that spec.
//!
//! # Structure
//!
//! - [`CrimeSim`]: the state of §3, the lifecycle of §4, the messages and the
//!   pipeline of §5, the algorithms of §6, the interface slots of §11 (crime
//!   simulator and prop requester) and the save record of §12.
//! - [`CrimeShared`]: what the original keeps **per process** and shares
//!   between all crime simulators: the crime type list (§7) and the
//!   file-local `cRZRandomSuper` (§9). The host creates one and hands it to
//!   every simulator it creates.
//! - [`CrimeType`] and the pure helpers of §6.7 / §7 (mask parsing, lot bits,
//!   candidate selection) in `types.rs`; [`CrimeTuning`] (§8) in
//!   `tuning.rs`; the arithmetic of §6.1–§6.5 as pure functions in
//!   `algo.rs`.
//! - [`CrimeHost`]: everything the simulator reads from, or does to, the rest
//!   of the game (city, residential, demand, police, ordinance, lots, zones,
//!   occupants, props, disasters, messages, agent registration). It has
//!   [`GridProvider<u8>`] as a supertrait: the two grids are created by the
//!   host (in game: the game's own `cSC4SimGrid<uint8>`, simgrid §6.4) and
//!   held as `Rc<dyn TractGrid<u8>>`.
//! - [`CrimeRandom`]: the draws of §9, implemented by [`RzRandomSuper`];
//!   tests script it.
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - Census (§6.1): a bucket outside 0..3 (with the 24-bit product of
//!   foundations §18: EQ ≥ 200, EQ ≤ −50, NaN, ±∞) is ignored (spec
//!   choice); EQ 50 falls in bucket 1 (§6.1, test 2). A wealth level whose
//!   population or EQ grid the host does not supply contributes nothing.
//! - Economic factors (§6.2): the factor is 0 when `B = C` (spec choice). A
//!   demand ID the host does not know (`GetDemand` null, not checked by the
//!   original) is skipped.
//! - Truncations to 32-bit integers (`trunc` of §6) follow the CRT `_ftol`
//!   for finite values below 2⁶³ (the low 32 bits of the 64-bit result);
//!   NaN, ±∞ and larger values give 0, the low half of the "integer
//!   indefinite".
//! - Crimes to commit (§6.4): `rhe` of the `f32` product is saturated to
//!   `i32` (the original idiom is only valid below 2²²); test 6's values
//!   follow §18 (10 × 0.95f → 9.5f → 10). `AddCrimes` makes at
//!   most [`MAX_ATTEMPTS`] attempts per call (the original loops `n` times).
//! - Stamps (§6.6 step 7) only visit tracts inside the crime grid; the
//!   original writes outside its rows for lots outside the grid.
//! - A lot whose `GetLocation` fails reads zone type 0 (bits `0x80000000`).
//!   Building wealth and purpose are compared as signed integers (`≥ 1`);
//!   the shift count of `1 << (v − 1)` is taken modulo 32 like the x86
//!   `shl`.
//! - Without a police grid the police average reads 0; without an ordinance
//!   simulator the ordinance factor is left unchanged (the host returns
//!   `None`).
//! - Average crime (§6.9): the tract corner cell is `(x·S, z·S)` with S the
//!   **tuning** tract size (as in §11.2; spec question); with no zoned tract
//!   the average is 0 (spec choice).
//! - Crime effect (§6.10): arrays shorter than 2 bytes are skipped (spec
//!   choice). The property is read as a byte array (any numeric element
//!   type, low 8 bits).
//! - Tuning (§8): with the exemplar missing, the five defaulted curve kinds
//!   get their code defaults (spec choice); the prop request curve, which
//!   has none, keeps its previous content (empty → 0 in a new simulator). A
//!   curve property that is a scalar or non-numeric is treated as absent; a
//!   present array without a complete point gives an empty curve, which
//!   evaluates to 0 (foundations §4.1, §4.3).
//! - Crime types (§7): the display name is the LTEXT string named by the
//!   three `u32` of `0x8A416B2E` when the host has it, otherwise the string
//!   property `0xAA12FCB0`, otherwise empty. A mask property that is not
//!   numeric is treated as missing (all bits); numeric elements are reduced
//!   to their low 8 bits. A missing or short icon key fills with 0.
//! - `SelectProp` (§11.6): "the incident's lot is `lot`" compares lot
//!   identities, so a null lot (a cell without a lot) matches incidents that
//!   have no lot (possible only after a load), as a pointer comparison
//!   would.
//! - Save (§12): the two unused `u32` are written as 0 (spec choice) and
//!   ignored on read. A reference to a core object where a lot is expected
//!   reads as no lot; an unresolvable grid reference reads as no grid (`Init`
//!   then creates a new one). On a stream error `Read` returns false and
//!   leaves the simulator unchanged except `loadedFromSave` (lots already
//!   read are released).
//! - Arithmetic follows foundations §18 (24-bit x87): every float operation
//!   is one `f32` rounding; an integer or `double` operand enters unrounded
//!   (`f64` operation rounded to `f32` once, [`mul_f32`]). The distance of
//!   §11.3 is `sqrt((dx² + dy²) + dz²)` in that order ([choice], the
//!   original's vector routine is not specified). The generator's own
//!   double arithmetic (§9) belongs to `RzRandomSuper`.
//! - The process-wide part (shutdown callback freeing the static type list,
//!   §4.1 step 2) is the plugin's: [`CrimeShared`] lives as long as the host
//!   keeps it.

mod algo;
mod simulator;
mod tuning;
mod types;

pub use algo::{
    census_bucket, crimes_to_commit, criminal_base_term, decay_value, ftol32, jobless_factor, mul_f32, rhe_i32,
    PROP_SCALE,
};
pub use simulator::{
    CrimeData, CrimeEvent, CrimeMessage, CrimeSim, Incident, CRIME_RECORD_VERSION, MAX_ATTEMPTS, STATE_IDLE,
};
pub use tuning::{shipped_tuning, CrimeTuning};
pub use types::{
    choose_candidate, condition_bits, mask_of, parse_mask, purpose_bits, shipped_type_exemplars, wealth_bits,
    zone_bits, CrimeType, LotBits, ANY_BITS, NONE_BIT, NO_PURPOSE_BIT,
};

use std::cell::RefCell;
use std::rc::Rc;

pub use crate::grid::{GridProvider, TractGrid};
use crate::property::PropertyHolder;
pub use crate::rng::RzRandomSuper;
use crate::rng::SeedSource;
use crate::stream::ResKey;

/// Class and interface IDs (§1).
pub const CLSID_CRIME_SIMULATOR: u32 = 0x8990_C09A;
pub const IID_CRIME_SIMULATOR: u32 = 0x6765_9555;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_PROP_REQUESTER: u32 = 0x89C4_98BE;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// The class slot 15 result (`GetSimulatorType`, §1).
pub const SIMULATOR_TYPE: u32 = 0x2756_7222;
/// The interface a lot reference is read with (§12).
pub const IID_LOT: u32 = 0xE818_FCED;

/// Agent registration (§4.1 step 4).
pub const AGENT_NAME: &str = "cSC4CrimeSimulator";
pub const AGENT_FLAGS: u32 = 2;
pub const AGENT_LEVELS: [u32; 3] = [4, 6, 10];

/// Grid instance IDs (§2).
pub const INSTANCE_CRIME_GRID: u32 = 0x89D5_B895;
pub const INSTANCE_EFFECT_GRID: u32 = 0x2A5B_A1B3;

/// The tuning exemplar (§8) and the crime type group (§7).
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC8E8_F6C5,
};
pub const CRIME_TYPE_GROUP: u32 = 0xEA12_F32C;

/// The ordinance property crime asks for (§5).
pub const ORDINANCE_CRIME_FACTOR: u32 = 0x28ED_0380;
/// `GetRequesterSatisfactionId` (§11.6).
pub const PROP_SATISFACTION_ID: u32 = 0x49A1_E996;
/// The fire disaster manager, its interface and the arson cause (§6.8).
pub const DISASTER_FIRE: u32 = 0x621C_DA33;
pub const IID_DISASTER_MANAGER_FIRE: u32 = 0x49B4_3F57;
pub const ARSON_CAUSE: u32 = 2;
/// The constant XORed with the local wall-clock seconds to seed the
/// generator (§9).
pub const RNG_SEED_XOR: u32 = 0x6C09_C3F6;

/// `GetNearestCrimeSim/Prop` filters and interfaces (§11.3).
pub const CRIME_SIM_TYPE: u32 = 0x496E_5783;
pub const CRIME_SIM_GROUP: u32 = 0x4102;
pub const IID_CRIME_AUTOMATON: u32 = 0xA9B4_0F05;
pub const CRIME_PROP_TYPE: u32 = 0xA823_821E;
pub const IID_PROP_OCCUPANT: u32 = 0xE979_3A65;

/// Demand IDs summed per wealth by `CalculateEconomicFactors` (§6.2).
pub const DEMAND_IDS: [&[u32]; 3] = [
    &[0x4100, 0x4200, 0x4300, 0x3110, 0x3120],
    &[0x4400, 0x3320, 0x3130],
    &[0x3330],
];

/// Messages (§5, §10).
pub mod msg {
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const SIM_IDLE: u32 = 0x6695_681A;
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    pub const ORDINANCE_UPDATED: u32 = 0xC2C0_9554;
    pub const EXEMPLAR_CHANGED: u32 = 0xC53D_10AA;
    /// The subscriptions of `Init`, in order (§4.1 step 5).
    pub const SUBSCRIBE: [u32; 4] = [OCCUPANT_INSERTED, OCCUPANT_REMOVED, ORDINANCE_UPDATED, EXEMPLAR_CHANGED];
    /// The unsubscriptions of `Shutdown`, in order (§4.2).
    pub const UNSUBSCRIBE: [u32; 4] = [ORDINANCE_UPDATED, EXEMPLAR_CHANGED, OCCUPANT_INSERTED, OCCUPANT_REMOVED];
    /// Posted per incident: data1 = type ID, IGZUnknown = the lot, string =
    /// the name (§10).
    pub const CRIME_COMMITTED: u32 = 0x0A15_9D95;
    /// Posted at pipeline state 5: void1 = IGZUnknown = the simulator (§10).
    pub const CRIME_MAP_UPDATED: u32 = 0x88EA_4DFF;
}

/// Properties (§7, §8).
pub mod prop {
    pub const TRACT_SIZE: u32 = 0x47E2_C200;
    pub const PROP_REQUEST_CURVE: u32 = 0x49A3_4301;
    /// EQ → base criminality, R$, R$$, R$$$.
    pub const EQ_CURVES: [u32; 3] = [0x8A0C_BAA4, 0x8A0C_BAA5, 0x8A0C_BAA6];
    /// Jobless ratio → criminality, R$, R$$, R$$$.
    pub const JOBLESS_CURVES: [u32; 3] = [0x8A0C_BAA7, 0x8A0C_BAA8, 0x8A0C_BAA9];
    pub const CRIMES_CURVE: u32 = 0x8A0C_BAAA;
    pub const ATTENUATION_CURVE: u32 = 0x8A0C_BAAB;
    pub const DECAY: u32 = 0x8A0C_BAAC;
    pub const REOFFEND: u32 = 0x8A0C_BAAD;
    /// Building crime effect `{magnitude, radius in cells}` (§6.10).
    pub const CRIME_EFFECT: u32 = 0xCA5B_9306;
    /// A prop's crime incident type (§8, read by the prop manager).
    pub const PROP_CRIME_INCIDENT: u32 = 0x8AA8_64D5;
    /// Preview occupants (foundations §5.2).
    pub const PREVIEW_OCCUPANT: u32 = 0x0292_44DB;
    /// Crime type exemplars (§7).
    pub const TYPE_NAME_KEY: u32 = 0x8A41_6B2E;
    pub const TYPE_NAME: u32 = 0xAA12_FCB0;
    pub const TYPE_ICON: u32 = 0xAA12_FCB1;
    pub const TYPE_ZONE_MASK: u32 = 0xAA12_FCB2;
    pub const TYPE_WEALTH_MASK: u32 = 0xAA12_FCB3;
    pub const TYPE_PURPOSE_MASK: u32 = 0xAA12_FCB4;
    pub const TYPE_SEVERITY: u32 = 0xAA12_FCB5;
    pub const TYPE_ARSON: u32 = 0xAA12_FCB6;
    pub const TYPE_CONDITION_MASK: u32 = 0xAA12_FCB7;
    pub const TYPE_WEIGHT: u32 = 0xAA12_FCB8;
}

/// A lot as the simulator holds it: the host identity and the class ID a
/// save writes for its reference (§12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LotRef {
    pub token: u64,
    pub class_id: u32,
}

/// The building on a lot (`GetBuilding` → `GetBuildingProfile`, §6.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingInfo {
    /// Host identity of the building occupant (for `IgniteOccupant`).
    pub token: u64,
    pub purpose: i32,
    pub wealth: i32,
}

/// One demand's active value, maximum and minimum (`cISC4Demand` slots
/// `+0x30`, `+0x38`, `+0x3C`, §6.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DemandValues {
    pub value: f32,
    pub max: f32,
    pub min: f32,
}

/// Which query `GetNearestCrimeSim` / `GetNearestCrimeProp` makes (§11.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NearestKind {
    Sim,
    Prop,
}

impl NearestKind {
    /// The bounding-box filter of §11.3: sims have type `0x496E5783` and are
    /// in occupant group `0x4102`; props have type `0xA823821E` and the
    /// property `0x8AA864D5`. The plugin's filter object may call this.
    pub fn filter_accepts(self, o: &NearbyOccupant) -> bool {
        match self {
            NearestKind::Sim => o.occupant_type == CRIME_SIM_TYPE && o.in_crime_group,
            NearestKind::Prop => o.occupant_type == CRIME_PROP_TYPE && o.has_incident_property,
        }
    }

    /// Whether a filtered occupant becomes the candidate: the QI succeeds
    /// (sims `0xA9B40F05`; props `0xE9793A65` with `GetState() = 0`).
    pub fn qualifies(self, o: &NearbyOccupant) -> bool {
        match self {
            NearestKind::Sim => o.answers_iid,
            NearestKind::Prop => o.answers_iid && o.prop_state == 0,
        }
    }
}

/// One occupant returned by the occupant manager's bounding-box query
/// (§11.3), with the facts the filter and the QI step need.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NearbyOccupant {
    pub token: u64,
    /// `GetType()`.
    pub occupant_type: u32,
    /// `IsOccupantGroup(0x4102)`.
    pub in_crime_group: bool,
    /// Has property `0x8AA864D5`.
    pub has_incident_property: bool,
    /// `GetPosition()`.
    pub position: [f32; 3],
    /// The QI for the kind's interface succeeds.
    pub answers_iid: bool,
    /// `cISC4PropOccupant::GetState()` (props only).
    pub prop_state: u32,
}

/// What the crime simulator reads from, and does to, the rest of the game.
/// Re-entrancy: the simulator holds no borrow across a callback, but
/// callbacks should not call back into the simulator.
pub trait CrimeHost: GridProvider<u8> {
    // ---- lifecycle glue (§4) ----
    /// `cISC4Simulator::AddAgent` (slot `+0x4C`) once per level of
    /// `levels`, in order, on the `cIGZMessageTarget2` sub-object.
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32);
    /// `RemoveAgent(target)` from all levels (slot `+0x50`).
    fn remove_agent(&self);
    /// Holds the message server (first time) and `AddNotification` for each
    /// message type, in order.
    fn subscribe(&self, msg_types: &[u32]);
    /// `RemoveNotification` for each type, in order, then releases the
    /// message server.
    fn unsubscribe(&self, msg_types: &[u32]);
    /// Holds the city, the occupant manager (`+0x88`), the ordinance
    /// simulator (`+0xFC`) and the police simulator (`+0x104`).
    fn acquire_services(&self);
    /// Releases what [`CrimeHost::acquire_services`] holds (police,
    /// ordinance, occupant manager, city).
    fn release_services(&self);
    /// `cISC4PropDeveloper::AddPropRequester` (slot `+0x14`) with the
    /// prop-requester sub-object (§11.6).
    fn add_prop_requester(&self);
    /// `Shutdown()` on a grid before the simulator drops it (§4.2).
    fn shutdown_grid(&self, grid: &Rc<dyn TractGrid<u8>>);

    // ---- resources (§7, §8) ----
    /// The tuning exemplar [`TUNING_KEY`], if it exists.
    fn tuning(&self) -> Option<Rc<PropertyHolder>>;
    /// Every exemplar of type `0x6534284A`, group `0xEA12F32C`, as
    /// (instance, properties), in the resource manager's key-list order.
    fn crime_type_exemplars(&self) -> Vec<(u32, Rc<PropertyHolder>)>;
    /// The LTEXT string with this key, if any.
    fn ltext(&self, key: ResKey) -> Option<Vec<u8>>;

    // ---- residential and demand (§6.1, §6.2) ----
    /// `cISC4ResidentialSimulator::GetPopulationGrids` (R$, R$$, R$$$).
    fn population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3];
    /// `cISC4ResidentialSimulator::GetEQGrids` (R$, R$$, R$$$).
    fn eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3];
    /// `cISC4DemandSimulator::GetDemand(id, 0)` and its active value,
    /// maximum and minimum; `None` if there is no such demand.
    fn demand(&self, id: u32) -> Option<DemandValues>;

    // ---- police and ordinances (§6.4, §6.6) ----
    /// `cISC4PoliceSimulator::GetPolicePowerGrid` (slot `+0x14`).
    fn police_grid(&self) -> Option<Rc<dyn TractGrid<u8>>>;
    /// `GetJailPopulation` (slot `+0x78`).
    fn jail_population(&self) -> u32;
    /// `ResetUnjailedCriminals` (slot `+0x6C`; has side effects).
    fn reset_unjailed_criminals(&self) -> i32;
    /// `ProcessMonthlyCrimeStats` (slot `+0x98`).
    fn process_monthly_crime_stats(&self);
    /// `cISC4OrdinanceSimulator::GetCumulativeFactor(0x28ED0380, 0)`;
    /// `None` without an ordinance simulator.
    fn ordinance_crime_factor(&self) -> Option<f32>;

    // ---- lots and zones (§6.6–§6.9, §11) ----
    /// `cISC4LotManager::GetRandomLot()` (borrowed; its own generator).
    fn random_lot(&self) -> Option<LotRef>;
    /// `GetLot(cellX, cellZ, false)` (borrowed).
    fn lot_at(&self, cell_x: i32, cell_z: i32) -> Option<LotRef>;
    /// `GetBoundingRect`: inclusive cells `[x1, z1, x2, z2]`.
    fn lot_bounding_rect(&self, lot: u64) -> [i32; 4];
    /// `GetLocation`, or `None` if it fails.
    fn lot_location(&self, lot: u64) -> Option<(i32, i32)>;
    /// `GetCondition` (byte).
    fn lot_condition(&self, lot: u64) -> u8;
    /// `GetBuilding` and its profile; `None` without a building.
    fn lot_building(&self, lot: u64) -> Option<BuildingInfo>;
    /// `city.PositionToCell(lot.GetCenter())`.
    fn lot_center_cell(&self, lot: u64) -> (i32, i32);
    /// `cISC4ZoneManager::GetZoneType(x, z)`.
    fn zone_type(&self, cell_x: i32, cell_z: i32) -> u32;
    /// `AddRef` / `Release` on a held object (lots of incidents).
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);

    // ---- occupants and props (§6.10, §11.3, §11.6) ----
    /// Whether the occupant (as a property holder) has the property.
    fn occupant_has_property(&self, occupant: u64, id: u32) -> bool;
    /// A property of the occupant as bytes (`RefUint8` data), if present.
    fn occupant_property_bytes(&self, occupant: u64, id: u32) -> Option<Vec<u8>>;
    /// `GetPosition()` (world units).
    fn occupant_position(&self, occupant: u64) -> [f32; 3];
    /// `GetOccupantsByBBox` over the box `min`..`max`, in the manager's
    /// order. The host may pre-filter with [`NearestKind::filter_accepts`].
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], kind: NearestKind) -> Vec<NearbyOccupant>;
    /// `GetPropProfile(prop).crimeIncidentID` (prop manager slot `+0x34`).
    fn prop_crime_incident_id(&self, prop: u32) -> u32;

    // ---- arson (§6.8) ----
    /// Whether the city's disaster layer (`cISC4City +0x120`) exists.
    fn disaster_layer_present(&self) -> bool;
    /// `GetDisasterManager(0x621CDA33)`, QI `0x49B43F57`,
    /// `IgniteOccupant(building)`, `SetCause(cause)`, release.
    fn ignite(&self, building: u64, cause: u32);

    // ---- messages (§10) ----
    /// Posts `0x0A159D95`: data1 = type ID, IGZUnknown = the lot, string =
    /// a copy of the name.
    fn post_crime_committed(&self, type_id: u32, lot: LotRef, name: &[u8]);
    /// Posts `0x88EA4DFF`: void1 = IGZUnknown = the simulator.
    fn post_crime_map_updated(&self);
}

/// The draws the crime simulator makes from its generator (§9).
pub trait CrimeRandom {
    /// `RandomDoubleUniform()`.
    fn uniform_f64(&mut self) -> f64;
    /// `RandomDoubleRangeUniform(a, b)`.
    fn range_f64(&mut self, a: f64, b: f64) -> f64;
    /// `RandomUint32Uniform(n)`.
    fn uniform_u32(&mut self, n: u32) -> u32;
}

impl CrimeRandom for RzRandomSuper {
    fn uniform_f64(&mut self) -> f64 {
        RzRandomSuper::uniform_f64(self)
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        RzRandomSuper::range_f64(self, a, b)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        RzRandomSuper::uniform_u32(self, n)
    }
}

/// The seed of the file-local generator: the local wall-clock seconds XOR
/// `0x6C09C3F6` (§9).
pub fn seed_from_clock(local_seconds: u32) -> u32 {
    local_seconds ^ RNG_SEED_XOR
}

/// What the original keeps once per process and shares between all crime
/// simulators: the crime type list (§7, loaded by the first `Init`) and
/// the generator (§9, not saved).
pub struct CrimeShared {
    pub(crate) types: RefCell<Option<Rc<Vec<CrimeType>>>>,
    pub(crate) rng: RefCell<Box<dyn CrimeRandom>>,
}

impl CrimeShared {
    /// Process state with the given generator.
    pub fn new(rng: Box<dyn CrimeRandom>) -> Rc<CrimeShared> {
        Rc::new(CrimeShared {
            types: RefCell::new(None),
            rng: RefCell::new(rng),
        })
    }

    /// Process state with an `RzRandomSuper` seeded with `seed` (normally
    /// [`seed_from_clock`]).
    pub fn seeded(seed: u32, seeds: &dyn SeedSource) -> Rc<CrimeShared> {
        CrimeShared::new(Box::new(RzRandomSuper::new(seed, seeds)))
    }

    /// The loaded crime type list (`None` before the first `Init`).
    pub fn types(&self) -> Option<Rc<Vec<CrimeType>>> {
        self.types.borrow().clone()
    }

    /// Replaces the generator.
    pub fn set_random(&self, rng: Box<dyn CrimeRandom>) {
        *self.rng.borrow_mut() = rng;
    }
}
