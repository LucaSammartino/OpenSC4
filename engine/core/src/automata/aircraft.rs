// SPDX-License-Identifier: GPL-3.0-or-later
//! Aircraft-owned rules from aircraft.md and automata.md §§6.4, 6.6.
//!
//! The summary-only control laws are explicit errors, not simulated with an
//! invented flight model. See README.md's owner closure table and AF1–AF6.
//! Owners use ordinary exclusive mutation; getters do not borrow a RefCell.

use crate::stream::{InStream, OutStream, RefValue};

use super::codec::{get_vec3, put_vec3};
use super::{state_type, AutomatonBase, SetStateArgs, StateExtra, Vec3};

pub const CLSID_AIRCRAFT: u32 = 0xC96E_75DC;
pub const CLSID_AIRPLANE: u32 = 0xCBEB_4F62;
pub const CLSID_HELICOPTER: u32 = 0x8BA9_8EDA;

/// Explicit spec gaps. No host trait is permitted to supply these owner laws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AircraftUnsupported {
    /// AF1: property identity, player-list selection, rotor entry meanings.
    DriveParameterMeaning,
    /// AF2: forces, rotor integration, attitude and collision response.
    FlightModel,
    /// AF3: AI steering, runway/approach selection and chase distance gates.
    GoalControl,
    /// AF4: initialization failure cleanup, probes and AEMS mapping.
    EffectsAndLifecycle,
    /// AF5: player command mapping, weapons and spotlight cone rules.
    PlayerControls,
    /// AF6: dispatch transitions and aircraft-specific message routing.
    DispatchAndMessages,
}

impl AircraftUnsupported {
    pub const fn question(self) -> &'static str {
        match self {
            Self::DriveParameterMeaning => "AF1",
            Self::FlightModel => "AF2",
            Self::GoalControl => "AF3",
            Self::EffectsAndLifecycle => "AF4",
            Self::PlayerControls => "AF5",
            Self::DispatchAndMessages => "AF6",
        }
    }
}

/// Exemplar data remains foreign. The adapter chooses the property/list;
/// AF1 leaves its property ID and player-list identity open.
pub trait DriveParameterSource {
    fn drive_parameters(&self) -> Option<&[f32]>;
    fn parent(&self) -> Option<&dyn DriveParameterSource>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct AirplaneDriveParameters(pub [f32; 18]);

impl Default for AirplaneDriveParameters {
    fn default() -> Self {
        Self([
            100.0, 1.0, 25.0, 150.0, 0.2618, 0.06545, 0.7854, 0.3927, 0.5236, 0.04363, 10000.0, 10000.0, 0.4, 0.04,
            0.5236, 0.08, 0.68, 0.0,
        ])
    }
}

impl AirplaneDriveParameters {
    /// aircraft.md §3.1: nearest list wins; zero entries use defaults, and
    /// explicit entries 2 and 3 alone get the ×1000 conversion.
    pub fn load(mut source: Option<&dyn DriveParameterSource>) -> Self {
        while let Some(current) = source {
            if let Some(list) = current.drive_parameters() {
                return Self::from_list(Some(list));
            }
            source = current.parent();
        }
        Self::default()
    }

    pub fn from_list(list: Option<&[f32]>) -> Self {
        let Some(list) = list else {
            return Self::default();
        };
        let mut values = Self::default().0;
        for (i, value) in values.iter_mut().enumerate() {
            // Literal reading for a short list: absent entries act as zero.
            // The spec gives no malformed/short-list rule; tracked in AF7.
            if let Some(&given) = list.get(i).filter(|&&given| given != 0.0) {
                *value = if i == 2 || i == 3 { given * 1000.0 } else { given };
            }
        }
        for (derived, parent, factor) in [
            (5, 4, 0.25),
            (7, 6, 0.5),
            (9, 8, 1.0 / 12.0),
            (11, 10, 1.0),
            (13, 12, 0.1),
        ] {
            if list.get(derived).copied().unwrap_or(0.0) == 0.0 {
                values[derived] = values[parent] * factor;
            }
        }
        Self(values)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AircraftRecord {
    pub base: AutomatonBase,
}

impl AircraftRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.error() == 0 && out.put_u16(1) && self.base.write(out) && out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<Self> {
        if inp.error() != 0 || inp.get_u16()? != 1 {
            return None;
        }
        let base = AutomatonBase::read(inp)?;
        (inp.error() == 0).then_some(Self { base })
    }
}

/// Both concrete owners have this layout, without a traffic record. It is
/// shared storage/codec, not a second implementation of the automaton base.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AircraftPlacementRecord {
    pub aircraft: AircraftRecord,
    pub home_position: Vec3,
    /// Runtime only; set on Read, never written.
    pub position_loaded: bool,
    initialized: bool,
}

impl AircraftPlacementRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        // The concrete writers always emit 1, even if Read accepted 0.
        out.error() == 0
            && out.put_u16(1)
            && self.aircraft.write(out)
            && put_vec3(out, &self.home_position)
            && out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<Self> {
        if inp.error() != 0 || inp.get_u16()? > 1 {
            return None;
        }
        let aircraft = AircraftRecord::read(inp)?;
        let home_position = get_vec3(inp)?;
        (inp.error() == 0).then_some(Self {
            aircraft,
            home_position,
            position_loaded: true,
            initialized: false,
        })
    }

    /// Read is city-independent. Only the first Init captures a new home;
    /// loaded owners move to the saved placement instead (automata §6.4).
    pub fn init_placement(&mut self) {
        if self.initialized {
            return;
        }
        if self.position_loaded {
            self.aircraft.base.vectors[0] = self.home_position;
        } else {
            self.home_position = self.aircraft.base.vectors[0];
        }
        self.initialized = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AircraftBounds {
    pub min_x: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_z: f32,
}

/// Renderer, terrain, occupant and simulator queries stay foreign.
pub trait AircraftWorld {
    fn bounds(&self) -> AircraftBounds;
    fn terrain_height(&self, x: f32, z: f32) -> f32;
    fn water_height(&self, x: f32, z: f32) -> Option<f32>;
    fn building_height(&self, x: f32, z: f32) -> Option<f32>;
    fn prop_height(&self, x: f32, z: f32) -> Option<f32>;
}

/// aircraft.md §2: surface height raised by buildings and props.
pub fn aircraft_altitude(world: &dyn AircraftWorld, x: f32, z: f32) -> f32 {
    let mut height = world.terrain_height(x, z);
    for top in [
        world.water_height(x, z),
        world.building_height(x, z),
        world.prop_height(x, z),
    ]
    .into_iter()
    .flatten()
    {
        height = height.max(top);
    }
    height
}

/// aircraft.md §§2,6: horizontal bounds have an outward 1000-unit margin.
pub fn clip_aircraft_to_world(mut point: Vec3, bounds: AircraftBounds) -> Vec3 {
    point[0] = point[0].clamp(bounds.min_x - 1000.0, bounds.max_x + 1000.0);
    point[2] = point[2].clamp(bounds.min_z - 1000.0, bounds.max_z + 1000.0);
    point
}

/// This phase is independent of the goal stack (automata §6.6). The spec
/// does not name the three values; do not infer ground/cruise meanings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FlightPhase {
    Phase0 = 0,
    Phase1 = 1,
    Phase3 = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightPhaseChange {
    pub previous: Option<FlightPhase>,
    pub current: FlightPhase,
}

/// Effect names are established in §3.2, but attachment, probes, cleanup,
/// sounds and failure policy are not (AF4). This is a resource declaration,
/// not a claim that native Init is complete.
pub fn airplane_effect_names(fire_plane: bool) -> [&'static str; 6] {
    let (still, moving) = if fire_plane {
        ("fx_00_fireplanepropstill", "fx_00_fireplaneprop")
    } else {
        ("fx_00_planepropstill", "fx_00_planeprop")
    };
    [
        still,
        moving,
        "fx_00_skywriting",
        "fx_00_waterdrop",
        "aircraftindicate",
        "jetbomb",
    ]
}

#[derive(Clone, Debug, Default)]
pub struct Airplane {
    pub record: AircraftPlacementRecord,
    pub drive_parameters: AirplaneDriveParameters,
    pub player_parameters: AirplaneDriveParameters,
    phase: Option<FlightPhase>,
    ammunition: Option<u32>,
    loiter_elapsed_ms: Option<f32>,
}

impl Airplane {
    pub const fn class_id(&self) -> u32 {
        CLSID_AIRPLANE
    }

    pub fn base(&self) -> &AutomatonBase {
        &self.record.aircraft.base
    }
    pub fn base_mut(&mut self) -> &mut AutomatonBase {
        &mut self.record.aircraft.base
    }
    pub fn phase(&self) -> Option<FlightPhase> {
        self.phase
    }
    pub fn ammunition(&self) -> Option<u32> {
        self.ammunition
    }

    /// Parameter and placement portion of Init. AF4 blocks native Init closure.
    pub fn init_data(&mut self, ai: Option<&dyn DriveParameterSource>, player: Option<&dyn DriveParameterSource>) {
        self.drive_parameters = AirplaneDriveParameters::load(ai);
        self.player_parameters = AirplaneDriveParameters::load(player);
        self.record.init_placement();
    }

    /// The phase change is returned for an eventual AEMS adapter. No group
    /// mapping is guessed here (AF4). Repeated phases do not refill ammo.
    pub fn set_airplane_state(&mut self, next: FlightPhase) -> Option<FlightPhaseChange> {
        if self.phase == Some(next) {
            return None;
        }
        let previous = self.phase.replace(next);
        if next == FlightPhase::Phase1 {
            self.ammunition = Some(3);
        }
        Some(FlightPhaseChange {
            previous,
            current: next,
        })
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        self.record.write(out)
    }

    /// Atomic owner replacement; transient state is rebuilt by Init, not saved.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(record) = AircraftPlacementRecord::read(inp) else {
            return false;
        };
        *self = Self {
            record,
            ..Self::default()
        };
        true
    }

    pub fn pursue_goal(&mut self) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        match self.base().states.last().map(|s| s.state_type) {
            Some(state_type::CHASE) => {
                self.base_mut().pop_state();
                Ok(AircraftGoalOutcome::Popped)
            }
            Some(58) => Ok(AircraftGoalOutcome::NoAircraftHandler), // CROP_DUST is name-only.
            Some(50 | 51 | 53 | 57) => Err(AircraftUnsupported::GoalControl),
            _ => Ok(AircraftGoalOutcome::NoAircraftHandler),
        }
    }

    /// The airplane's documented LOITER push uses an infinite stack timeout;
    /// the separate two-second timer and ammo decrement end it (§6.6).
    pub fn begin_loiter(&mut self) {
        self.base_mut().push_state(53, SetStateArgs::default());
        self.loiter_elapsed_ms = Some(0.0);
    }

    pub fn tick_loiter(&mut self, dt_ms: f32) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        if self.base().states.last().map(|s| s.state_type) != Some(53) {
            return Ok(AircraftGoalOutcome::NoAircraftHandler);
        }
        let Some(elapsed) = self.loiter_elapsed_ms else {
            // AF3: reconstructing an unsaved timer for a loaded LOITER is
            // not specified; do not silently treat it as a newly begun one.
            return Err(AircraftUnsupported::GoalControl);
        };
        let next = elapsed + dt_ms;
        // Literal "after a 2.0 s timer": strictly greater; boundary is AF3.
        if next <= 2000.0 {
            self.loiter_elapsed_ms = Some(next);
            return Ok(AircraftGoalOutcome::Waiting);
        }
        let Some(ammunition) = self.ammunition.and_then(|n| n.checked_sub(1)) else {
            return Err(AircraftUnsupported::GoalControl); // AF3: empty/unknown ammo.
        };
        self.ammunition = Some(ammunition);
        self.loiter_elapsed_ms = None;
        self.base_mut().pop_state();
        Ok(AircraftGoalOutcome::Popped)
    }

    /// AF2 explicitly leaves CalculateForces/UpdatePhysics open. No mutation
    /// and no fake movement on failure.
    pub fn update_physics(&mut self, _dt_ms: f32, _world: &dyn AircraftWorld) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AircraftGoalOutcome {
    Popped,
    Waiting,
    NoAircraftHandler,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChaseTarget {
    pub position: Vec3,
    pub orientation: Vec3,
    pub speed: f32,
    pub top_state: Option<u32>,
}

pub trait AircraftTargets {
    fn chase_target(&self, target: &RefValue) -> Option<ChaseTarget>;
}

/// Established geometry only. Distance gates, teleport destination and the
/// hover control law remain AF3; this is not a completed chase tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelicopterChaseGeometry {
    pub lead_position: Vec3,
    pub spotlight_target: Vec3,
    pub hover_position: Vec3,
}

impl HelicopterChaseGeometry {
    pub fn for_target(target: ChaseTarget, dt_ms: f32) -> Self {
        let mut lead_position = target.position;
        for (value, direction) in lead_position.iter_mut().zip(target.orientation) {
            // Literal automata §6.6 formula. Frame dt is milliseconds; do
            // not silently replace its documented ×1000 by ÷1000 (AF3).
            *value += direction * target.speed * 1000.0 * dt_ms;
        }
        let mut hover_position = target.position;
        hover_position[1] += 20.0;
        Self {
            lead_position,
            spotlight_target: target.position,
            hover_position,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Helicopter {
    pub record: AircraftPlacementRecord,
    phase: Option<FlightPhase>,
}

impl Helicopter {
    pub const fn class_id(&self) -> u32 {
        CLSID_HELICOPTER
    }
    pub fn base(&self) -> &AutomatonBase {
        &self.record.aircraft.base
    }
    pub fn base_mut(&mut self) -> &mut AutomatonBase {
        &mut self.record.aircraft.base
    }
    pub fn phase(&self) -> Option<FlightPhase> {
        self.phase
    }

    pub fn set_helicopter_state(&mut self, next: FlightPhase) -> Option<FlightPhaseChange> {
        if self.phase == Some(next) {
            return None;
        }
        Some(FlightPhaseChange {
            previous: self.phase.replace(next),
            current: next,
        })
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        self.record.write(out)
    }

    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(record) = AircraftPlacementRecord::read(inp) else {
            return false;
        };
        *self = Self {
            record,
            ..Self::default()
        };
        true
    }

    /// Performs the settled chase rejection rules only. A valid target
    /// requires AF3's missing movement/gates and is never treated as moved.
    pub fn pursue_goal(&mut self, targets: &dyn AircraftTargets) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        match self.base().states.last() {
            Some(state) if state.state_type == state_type::CHASE => {
                let target = match &state.extra {
                    StateExtra::Chase(reference) if !reference.is_null() => targets.chase_target(reference),
                    _ => None,
                };
                if target
                    .map(|t| t.top_state != Some(state_type::PLAYER_DRIVE))
                    .unwrap_or(true)
                {
                    self.base_mut().pop_state();
                    Ok(AircraftGoalOutcome::Popped)
                } else {
                    Err(AircraftUnsupported::GoalControl)
                }
            }
            Some(state) if matches!(state.state_type, 0 | 50 | 51 | 53..=56) => Err(AircraftUnsupported::GoalControl),
            _ => Ok(AircraftGoalOutcome::NoAircraftHandler),
        }
    }

    pub fn load_drive_parameters(
        &mut self,
        _source: Option<&dyn DriveParameterSource>,
    ) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::DriveParameterMeaning)
    }

    pub fn next_frame(&mut self, _dt_ms: f32, _world: &dyn AircraftWorld) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }

    /// The only specified LOITER push for helicopter states 54/55/56.
    pub fn push_mission_loiter(&mut self) -> bool {
        self.base_mut().push_state(
            53,
            SetStateArgs {
                timeout_ms: 6000.0,
                ..SetStateArgs::default()
            },
        )
    }
}
