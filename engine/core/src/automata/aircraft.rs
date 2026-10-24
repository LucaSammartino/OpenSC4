// SPDX-License-Identifier: GPL-3.0-or-later
//! Aircraft-owned rules from aircraft.md and automata.md §§6.4, 6.6.
//!
//! The summary-only control laws are explicit errors, not simulated with an
//! invented flight model. See README.md's owner closure table and AF1–AF8.
//! Owners use ordinary exclusive mutation; getters do not borrow a RefCell.

use crate::stream::{InStream, OutStream, RefValue};

use super::codec::{get_vec3, put_vec3};
use super::{AutomatonBase, Vec3};

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
    /// AF8: station unit counts exceeding the available-units property's width.
    StationUnitCount,
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
            Self::StationUnitCount => "AF8",
        }
    }
}

/// Imported group data remains foreign: this is the named `airplane` record,
/// not a numeric exemplar property. A present record contains 18 floats;
/// missing named fields have already become zero. Player selection is AF5.
pub trait DriveParameterSource {
    fn drive_parameters(&self) -> Option<&[f32]>;
    fn parent(&self) -> Option<&dyn DriveParameterSource>;
}

/// The property boundary for aircraft.md §8 test 3. Station membership and
/// inter-station call numbering remain the foreign dispatch source's job.
pub trait AirStationProperties {
    fn is_building_occupant(&self) -> bool;
    fn property_u32(&self, id: u32) -> Option<u32>;
    fn set_property_u32(&mut self, id: u32, value: u32);
}

/// Initialize only the specified fire-plane available-unit property (§5).
/// Returns the registered unit count, or None for an ineligible station.
pub fn initialize_fire_plane_station_units(
    station: &mut dyn AirStationProperties,
) -> Result<Option<u32>, AircraftUnsupported> {
    if !station.is_building_occupant() {
        return Ok(None);
    }
    let count = station.property_u32(0x0C0A_FDD7).unwrap_or(0);
    if (count as i32) <= 0 {
        return Ok(None);
    }
    let mask = (1u32 << (count % 32)).wrapping_sub(1);
    station.set_property_u32(0xCC0A_FDDB, mask);
    Ok(Some(count))
}

pub use super::airplane::{Airplane, AirplaneDriveParameters};

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
    /// Absolute world-space top, not a size to add to terrain height.
    fn building_height(&self, x: f32, z: f32) -> Option<f32>;
    /// Absolute world-space top, not a size to add to terrain height.
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
pub struct FlightPhase(pub u32);

#[allow(non_upper_case_globals)]
impl FlightPhase {
    pub const Phase0: Self = Self(0);
    pub const Phase1: Self = Self(1);
    pub const Phase3: Self = Self(3);
    pub const Phase4: Self = Self(4);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlightPhaseChange {
    pub previous: Option<FlightPhase>,
    pub current: FlightPhase,
    pub sounds: Vec<super::SoundRequest>,
    pub engine_notifications: Vec<super::EngineNotification>,
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
    pub fn for_target(target: ChaseTarget, dt_seconds: f32) -> Self {
        let mut lead_position = target.position;
        for (value, direction) in lead_position.iter_mut().zip(target.orientation) {
            // aircraft §4.2.2: the caller has already converted to seconds.
            *value += direction * (target.speed * 1000.0 * dt_seconds);
        }
        let mut hover_position = lead_position;
        hover_position[1] += 20.0;
        Self {
            lead_position,
            spotlight_target: lead_position,
            hover_position,
        }
    }
}
