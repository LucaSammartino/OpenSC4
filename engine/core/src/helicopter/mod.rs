// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4Helicopter: aircraft.md §§2/4/8 and automata.md §§6.4/6.6.
//! See README.md for owner closure and the unsettled AF2–AF6 contracts.

use crate::automata::{
    aircraft_altitude, clip_aircraft_to_world, state_type, AircraftGoalOutcome, AircraftPlacementRecord,
    AircraftTargets, AircraftUnsupported, AircraftWorld, AutomatonBase, FlightPhase, FlightPhaseChange, SetStateArgs,
    StateExtra, Vec3, CLSID_HELICOPTER,
};
use crate::stream::{InStream, OutStream};

pub use crate::automata::HelicopterChaseGeometry;

/// Import order from aircraft.md §4.1, not an exemplar-property list.
pub const DRIVE_FIELDS: [&str; 13] = [
    "Mass",
    "Power",
    "SpeedMultiplier",
    "MaxSpeed",
    "DragFactor",
    "RotationalDragFactor",
    "BankMax",
    "BankChangeRate",
    "PitchMax",
    "PitchChangeRate",
    "YawRateMax",
    "BankMultiplier",
    "PitchMultiplier",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum DriveField {
    Mass,
    Power,
    SpeedMultiplier,
    MaxSpeed,
    DragFactor,
    RotationalDragFactor,
    BankMax,
    BankChangeRate,
    PitchMax,
    PitchChangeRate,
    YawRateMax,
    BankMultiplier,
    PitchMultiplier,
}

/// A present named Lua record has exactly thirteen binary32 fields. Missing
/// names import as zero; a present empty record still stops inheritance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveRecord(pub [f32; 13]);

impl DriveRecord {
    pub fn import(mut field: impl FnMut(&str) -> Option<f32>) -> Self {
        Self(DRIVE_FIELDS.map(|name| field(name).unwrap_or(0.0)))
    }
}

/// Foreign group lookup only; all conversion and overlay rules stay here.
/// Typed records exclude unsupported short raw buffers (aircraft.md §3.1).
pub trait DriveParameterSource {
    fn helicopter_record(&self) -> Option<&DriveRecord>;
    fn parent(&self) -> Option<&dyn DriveParameterSource>;
}

/// Existing rotor preset. AF4 leaves its construction defaults unspecified,
/// so unknown fields remain None until supplied or overlaid by a nonzero value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DriveParameters(pub [Option<f32>; 13]);

impl DriveParameters {
    pub fn from_preset(values: [f32; 13]) -> Self {
        Self(values.map(Some))
    }

    pub fn get(&self, field: DriveField) -> Option<f32> {
        self.0[field as usize]
    }

    pub fn overlay(&mut self, record: &DriveRecord) {
        for (index, &input) in record.0.iter().enumerate() {
            // Literal binary32 operations, in the order specified in §4.1.
            let converted = match index {
                3 => (input * 1000.0_f32) * f32::from_bits(0x4055_5555),
                6..=10 => input * f32::from_bits(0x440F_3D4D),
                _ => input,
            };
            // Both signed zeros preserve the destination, after conversion.
            // Deliberately no finite/range validation, including NaNs.
            if converted != 0.0 {
                self.0[index] = Some(converted);
            }
        }
    }

    pub fn load(&mut self, mut source: Option<&dyn DriveParameterSource>) {
        // §3.1 explicitly has no cycle detection. Do not substitute
        // field-by-field inheritance or a made-up cycle fallback.
        while let Some(group) = source {
            if let Some(record) = group.helicopter_record() {
                self.overlay(record);
                return;
            }
            source = group.parent();
        }
    }
}

/// Core-only owner. Shared automaton and aircraft records are reused verbatim.
/// Getters use ordinary references, never RefCell borrowing.
#[derive(Clone, Debug, Default)]
pub struct Helicopter {
    pub record: AircraftPlacementRecord,
    pub drive_parameters: DriveParameters,
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

    /// The settled placement step, not complete native Init (AF4).
    pub fn init_placement(&mut self) {
        self.record.init_placement();
    }

    pub fn get_altitude(&self, world: &dyn AircraftWorld, x: f32, z: f32) -> f32 {
        aircraft_altitude(world, x, z)
    }

    pub fn clip_to_world(&self, world: &dyn AircraftWorld, position: Vec3) -> Vec3 {
        clip_aircraft_to_world(position, world.bounds())
    }

    /// Phase metadata only. Native rotor sound mapping is open in AF4.
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
        // Atomic owner replacement. Unsaved runtime defaults stay unknown
        // (AF4), rather than inventing a phase or rotor preset after loading.
        *self = Self {
            record,
            ..Self::default()
        };
        true
    }

    pub fn load_drive_parameters(
        &mut self,
        source: Option<&dyn DriveParameterSource>,
    ) -> Result<(), AircraftUnsupported> {
        self.drive_parameters.load(source);
        Ok(())
    }

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
                    // AF3: geometry is exposed, but distance gates and AI
                    // controls must not be delegated to a foreign flight model.
                    Err(AircraftUnsupported::GoalControl)
                }
            }
            Some(state) if matches!(state.state_type, 0 | 50 | 51 | 53..=56) => Err(AircraftUnsupported::GoalControl),
            Some(_) => {
                // automata.md §6.2: PopState only for every unlisted type,
                // including 58; no fall-through to 50.
                self.base_mut().pop_state();
                Ok(AircraftGoalOutcome::Popped)
            }
            None => Ok(AircraftGoalOutcome::NoAircraftHandler),
        }
    }

    /// Known push used by missions 54/55/56. Claim/search/goal transitions
    /// remain AF3; this helper does not claim a mission succeeded.
    pub fn push_mission_loiter(&mut self) -> bool {
        self.base_mut().push_state(
            53,
            SetStateArgs {
                timeout_ms: 6000.0,
                ..SetStateArgs::default()
            },
        )
    }

    // Explicit spec gaps. These signatures are core error boundaries, not
    // guessed native argument/slot layouts. No speculative partial mutation.
    pub fn physics_init(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::EffectsAndLifecycle)
    }
    pub fn init_instance(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::EffectsAndLifecycle)
    }
    pub fn update_rotor_sounds(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::EffectsAndLifecycle)
    }
    pub fn next_frame(&mut self, _dt_ms: f32, _world: &dyn AircraftWorld) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
    pub fn yaw_pitch_roll_slide_real(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
    pub fn move_xz_real(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
    pub fn move_y_real(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
    pub fn ai_control_reset(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::GoalControl)
    }
    pub fn ai_control_hover(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::GoalControl)
    }
    pub fn ai_control_move_xz(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::GoalControl)
    }
    pub fn ai_control_lift_over_buildings(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::GoalControl)
    }
    pub fn update_spotlight(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
    pub fn on_player_take_over(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
    pub fn translate_player_drive_commands(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
    pub fn do_player_drive_tick(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
}
