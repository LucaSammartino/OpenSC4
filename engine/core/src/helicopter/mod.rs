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

mod settled;
pub use settled::*;

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
    pub runtime: Option<RuntimeOwnership>,
    pub controls: Option<RotorControls>,
    pub engine_enabled: Option<bool>,
    pub landing_latch: Option<bool>,
    pub linear_velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
    pub scaled_rotor_position: Option<Vec3>,
    /// Inputs from the aircraft flag and retained attachment ownership, not guessed native defaults.
    pub player_driven: bool,
    pub spotlight_enabled: Option<bool>,
    pub input_gains: [Option<f32>; 4],
    pub retained_effects_present: [bool; 4],
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

    /// §4.2.3 repeats side effects even for the current phase.
    pub fn set_helicopter_state(&mut self, next: FlightPhase) -> Option<FlightPhaseChange> {
        use crate::automata::{EngineNotification, SoundRequest};
        match next {
            FlightPhase::Phase0 => {
                self.engine_enabled = Some(false);
                self.landing_latch = Some(false);
                for velocity in [&mut self.linear_velocity, &mut self.angular_velocity]
                    .into_iter()
                    .flatten()
                {
                    let squared: f64 = velocity.iter().map(|v| f64::from(*v).powi(2)).sum();
                    if squared > 0.0 {
                        let scale = 0.0 / squared.sqrt();
                        *velocity = velocity.map(|v| (f64::from(v) * scale) as f32);
                    }
                }
            }
            FlightPhase::Phase1 | FlightPhase::Phase4 => self.engine_enabled = Some(true),
            _ => {}
        }
        let sounds = if !self.player_driven {
            Vec::new()
        } else {
            match next {
                FlightPhase::Phase0 => crate::automata::aircraft_actions::stopped_sounds(),
                FlightPhase::Phase4 => vec![SoundRequest::Start(9)],
                _ => Vec::new(),
            }
        };
        let mut engine_notifications = Vec::new();
        // AF4 leaves the pre-PhysicsInit engine flag unknown. Never invent it.
        if let Some(engine) = self.engine_enabled {
            engine_notifications.push(EngineNotification::Base(engine));
            for (offset, operation, argument) in if engine {
                [(0x240, 0x18, 0), (0x22C, 0x1C, 1)]
            } else {
                [(0x240, 0x1C, 1), (0x22C, 0x18, 0)]
            } {
                engine_notifications.push(EngineNotification::Embedded {
                    offset,
                    operation,
                    argument,
                });
            }
        }
        Some(FlightPhaseChange {
            previous: self.phase.replace(next),
            current: next,
            sounds,
            engine_notifications,
        })
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        self.record.write(out)
    }

    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(record) = AircraftPlacementRecord::read(inp) else {
            return false;
        };
        // Read constructs/restores no runtime, phase, or unsaved parameters.
        self.record = record;
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
    pub fn physics_init(&mut self) -> Result<FlightPhaseChange, AircraftUnsupported> {
        if self.runtime.as_ref().and_then(|r| r.completion_word).is_none() {
            return Err(AircraftUnsupported::EffectsAndLifecycle);
        }
        Ok(self
            .set_helicopter_state(FlightPhase::Phase0)
            .expect("setter always returns actions"))
    }
    pub fn init_instance(&mut self) -> Result<(), AllocationFault> {
        self.init_instance_with(&mut CoreAllocator, None, None)
    }
    pub fn init_instance_with(
        &mut self,
        allocator: &mut dyn RuntimeAllocator,
        player: Option<&dyn PlayerParameterSource>,
        ai: Option<&dyn DriveParameterSource>,
    ) -> Result<(), AllocationFault> {
        if let Some(runtime) = &self.runtime {
            if runtime.completion_word.is_some() {
                return Ok(());
            }
            let missing = if !runtime.parameters {
                RuntimeResource::Parameters
            } else {
                RuntimeResource::Transform(TRANSFORM_OFFSETS[runtime.transforms.iter().position(|v| !*v).unwrap()])
            };
            return Err(AllocationFault(missing));
        }
        if !allocator.allocate(RuntimeResource::State) {
            return Err(AllocationFault(RuntimeResource::State));
        }
        self.runtime = Some(RuntimeOwnership::default());
        if !allocator.allocate(RuntimeResource::Parameters) {
            return Err(AllocationFault(RuntimeResource::Parameters));
        }
        self.runtime.as_mut().unwrap().parameters = true;
        self.drive_parameters = DriveParameters::preset0();
        // The copied preset makes the speed fallback's yaw maximum known.
        refresh_player_parameters(&mut self.drive_parameters, &mut self.input_gains, player)
            .expect("preset yaw maximum exists");
        self.drive_parameters.load(ai);
        for (index, offset) in TRANSFORM_OFFSETS.into_iter().enumerate() {
            let resource = RuntimeResource::Transform(offset);
            if !allocator.allocate(resource) {
                return Err(AllocationFault(resource));
            }
            self.runtime.as_mut().unwrap().transforms[index] = true;
        }
        self.runtime.as_mut().unwrap().completion_word = Some(0);
        Ok(())
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
    pub fn ai_control_reset(&mut self, seconds: f32, pitch_factor: f32) -> Result<AngularDemands, AircraftUnsupported> {
        self.controls
            .as_mut()
            .ok_or(AircraftUnsupported::EffectsAndLifecycle)?
            .reset(seconds, pitch_factor, &self.drive_parameters)
    }
    pub fn ai_control_hover(
        &mut self,
        seconds: f32,
        vertical_velocity: f32,
        height: f32,
    ) -> Result<(), AircraftUnsupported> {
        let y = self.base().vectors[0][1];
        self.controls
            .as_mut()
            .ok_or(AircraftUnsupported::EffectsAndLifecycle)?
            .hover(seconds, y, vertical_velocity, height);
        Ok(())
    }
    pub fn ai_control_move_xz(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::GoalControl)
    }
    pub fn ai_control_lift_over_buildings<W: ClearanceWorld>(
        &mut self,
        world: &mut W,
        seconds: f32,
        velocity: Vec3,
    ) -> Result<f32, AircraftUnsupported> {
        if self.controls.is_none() {
            return Err(AircraftUnsupported::EffectsAndLifecycle);
        }
        let height = building_clearance_height(world, self.base().vectors[0], velocity, self.record.home_position[1]);
        self.ai_control_hover(seconds, velocity[1], height)?;
        Ok(height)
    }
    pub fn update_spotlight(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
    pub fn on_player_take_over(
        &mut self,
        provider: Option<&dyn PlayerParameterSource>,
    ) -> Result<(), AircraftUnsupported> {
        refresh_player_parameters(&mut self.drive_parameters, &mut self.input_gains, provider)
    }
    pub fn translate_player_drive_commands(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }
    pub fn do_player_drive_tick(&mut self) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::PlayerControls)
    }

    /// Settled chase decisions only. MoveXZ and the remaining complete AI switch are AF3.
    pub fn pursue_chase_stage(
        &mut self,
        targets: &dyn AircraftTargets,
        seconds: f32,
        opacity: f32,
        fade_duration: f32,
    ) -> Result<Option<ChaseActions>, AircraftUnsupported> {
        let Some(state) = self.base().states.last() else {
            return Ok(None);
        };
        if state.state_type != state_type::CHASE {
            return Err(AircraftUnsupported::GoalControl);
        }
        let target = match &state.extra {
            StateExtra::Chase(reference) if !reference.is_null() => targets.chase_target(reference),
            _ => None,
        };
        let Some(target) = target.filter(|t| t.top_state == Some(state_type::PLAYER_DRIVE)) else {
            self.base_mut().pop_state();
            return Ok(None);
        };
        let actions = chase_actions(self.base().vectors[0], target, seconds, opacity, fade_duration);
        self.spotlight_enabled = Some(true);
        if let Some(position) = actions.teleport {
            self.base_mut().vectors[0] = position;
        }
        if let Some(position) = actions.scaled_rotor_position {
            self.scaled_rotor_position = Some(position);
        }
        Ok(Some(actions))
    }

    pub fn weapon_requests(
        &mut self,
        player: &mut crate::automata::PlayerWeaponRecord,
        effects: &mut dyn crate::automata::AircraftWeaponEffects,
    ) -> crate::automata::WeaponOutcome {
        crate::automata::aircraft_actions::weapon_requests(player, effects, &["helicoptermissle", "ufolaser"])
    }

    pub fn spotlight_activation_stage(
        &self,
        effects: &mut dyn SpotlightEffects,
    ) -> Result<SpotlightStage, AircraftUnsupported> {
        if !effects.present(0) || !effects.present(1) {
            return Ok(SpotlightStage::MissingPair);
        }
        let player_drive_state_present = self
            .base()
            .states
            .iter()
            .any(|s| s.state_type == state_type::PLAYER_DRIVE);
        let environment = effects.environment_mode();
        let enabled = self.spotlight_enabled.ok_or(AircraftUnsupported::PlayerControls)?;
        let phase = self.phase.ok_or(AircraftUnsupported::EffectsAndLifecycle)?;
        let requested = environment == 1 && enabled && phase != FlightPhase::Phase0;
        for index in 0..2 {
            let active = effects.active(index);
            if requested && !active {
                effects.start(index, 0);
            }
            if !requested && active {
                effects.stop(index, 1);
            }
        }
        Ok(SpotlightStage::NeedsGeometry {
            player_drive_state_present,
        })
    }

    /// Post-base Shutdown only (§4.2.5); pre-base scaling remains AF4.
    /// Slots marked present represent retained owning references, never borrowed pointers.
    pub fn shutdown_after_base(&mut self, base_result: bool) -> ShutdownActions {
        let mut actions = Vec::new();
        for (present, offset) in self
            .retained_effects_present
            .iter_mut()
            .zip([0x200, 0x204, 0x224, 0x228])
        {
            if *present {
                actions.push(CleanupAction::StopEffect { offset, argument: 1 });
                *present = false;
                actions.push(CleanupAction::ReleaseRetainedEffect(offset));
            }
        }
        for offset in [0x22C, 0x240] {
            actions.push(CleanupAction::Embedded {
                offset,
                operation: 0x1C,
                argument: Some(1),
            });
            actions.push(CleanupAction::Embedded {
                offset,
                operation: 0x04,
                argument: None,
            });
        }
        if let Some(runtime) = self.runtime.take() {
            for (present, offset) in runtime.transforms.into_iter().zip(TRANSFORM_OFFSETS) {
                if present {
                    actions.push(CleanupAction::Free(RuntimeResource::Transform(offset)));
                }
            }
            if runtime.parameters {
                actions.push(CleanupAction::Free(RuntimeResource::Parameters));
            }
            actions.push(CleanupAction::Free(RuntimeResource::State));
            self.drive_parameters = DriveParameters::default();
        }
        if self.controls.take().is_some() {
            actions.push(CleanupAction::Free(RuntimeResource::Controls));
        }
        self.phase = None;
        ShutdownActions { base_result, actions }
    }
}
