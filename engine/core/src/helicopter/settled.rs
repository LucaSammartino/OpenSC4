// SPDX-License-Identifier: GPL-3.0-or-later
//! Isolated settled rotor stages; no stage claims a complete AF2/AF3/AF5 tick.

use super::{DriveField, DriveParameters};
use crate::automata::{aircraft_altitude, AircraftUnsupported, AircraftWorld, ChaseTarget, Vec3};

pub const PRESET0_BITS: [u32; 13] = [
    0x3FC00000, 0x43700000, 0x3F333333, 0x43630000, 0x40800000, 0x3F800000, 0x43960000, 0x43C80000, 0x43960000,
    0x43C80000, 0x43C80000, 0x3F800000, 0x3F800000,
];

impl DriveParameters {
    pub fn preset0() -> Self {
        Self::from_preset(PRESET0_BITS.map(f32::from_bits))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotorControls {
    pub lateral: f32,
    pub longitudinal: f32,
    pub collective: f32,
    pub yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngularDemands {
    pub bank: f32,
    pub pitch: f32,
}

impl RotorControls {
    pub fn reset(
        &mut self,
        seconds: f32,
        pitch_factor: f32,
        parameters: &DriveParameters,
    ) -> Result<AngularDemands, AircraftUnsupported> {
        let known = |field| parameters.get(field).ok_or(AircraftUnsupported::EffectsAndLifecycle);
        let bank_rate = known(DriveField::BankChangeRate)?;
        let bank_max = known(DriveField::BankMax)?;
        let pitch_rate = known(DriveField::PitchChangeRate)?;
        let pitch_max = known(DriveField::PitchMax)?;
        let step = 100.0 * f64::from(seconds);
        for (control, target) in [
            (&mut self.lateral, 0.0),
            (&mut self.longitudinal, 0.0),
            (&mut self.collective, -100.0),
            (&mut self.yaw, 0.0),
        ] {
            let old = f64::from(*control);
            *control = (old + limit(target - old, -step, step)) as f32;
        }
        let factor = f64::from(f32::from_bits(0x3CCCCCCD));
        Ok(AngularDemands {
            bank: limit(
                f64::from(self.lateral) * f64::from(bank_rate) * factor,
                -f64::from(bank_max),
                f64::from(bank_max),
            ) as f32,
            pitch: limit(
                f64::from(pitch_factor) * f64::from(pitch_rate) * -f64::from(self.longitudinal) * factor,
                -f64::from(pitch_max),
                f64::from(pitch_max),
            ) as f32,
        })
    }

    pub fn hover(&mut self, seconds: f32, y: f32, vertical_velocity: f32, height: f32) {
        let old = f64::from(self.collective);
        let u = f64::from(vertical_velocity) * f64::from(f32::from_bits(0x3E99999A));
        let desired =
            ((f64::from(height) - (f64::from(y) + u * 0.5)) * f64::from(f32::from_bits(0x3DCCCCCD)) - u) * 0.5;
        // Preserve the native add-old/subtract-old order before limiting.
        let increment = (desired + old) - old;
        let step = 100.0 * f64::from(seconds);
        self.collective = limit(old + limit(increment, -step, step), -150.0, 100.0) as f32;
    }
}

fn limit(value: f64, low: f64, high: f64) -> f64 {
    // Literal ordered comparisons, no extra range/non-finite policy (AF2/AF10).
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

pub trait ClearanceWorld: AircraftWorld {
    /// Acquire the auxiliary provider once before either sample (§4.2.4).
    fn acquire_auxiliary_height(&mut self);
    fn accepts_terrain_point(&mut self, x: f32, z: f32) -> bool;
    fn auxiliary_height(&mut self, mode: u32, x: f32, z: f32) -> f32;
}

pub fn building_clearance_height<W: ClearanceWorld>(world: &mut W, position: Vec3, velocity: Vec3, home_y: f32) -> f32 {
    world.acquire_auxiliary_height();
    let predicted = std::array::from_fn(|i| {
        (f64::from(position[i]) + f64::from(f32::from_bits(0x3E99999A)) * f64::from(velocity[i])) as f32
    });
    let mut base = home_y;
    for sample in [position, predicted] {
        let mut height = aircraft_altitude(world, sample[0], sample[2]);
        if world.accepts_terrain_point(sample[0], sample[2]) {
            height = height.max(world.auxiliary_height(1, sample[0], sample[2]));
        }
        base = base.max(height);
    }
    let mut height = base + 20.0;
    if position[1] > height {
        height -= (position[1] - height).min(50.0);
    }
    height
}

/// Only the five named player inputs used by §4.3.1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerTakeoverRecord {
    pub speed: f32,
    pub gains: [f32; 4],
}

pub trait PlayerParameterSource {
    fn direct_record(&self) -> Option<&PlayerTakeoverRecord>;
    fn script_record(&self) -> Option<&PlayerTakeoverRecord>;
}

pub fn refresh_player_parameters(
    parameters: &mut DriveParameters,
    gains: &mut [Option<f32>; 4],
    provider: Option<&dyn PlayerParameterSource>,
) -> Result<(), AircraftUnsupported> {
    let Some(record) = provider.and_then(|p| p.direct_record().or_else(|| p.script_record())) else {
        return Ok(());
    };
    let speed = (record.speed * 1000.0) * f32::from_bits(0x40555555);
    parameters.0[DriveField::MaxSpeed as usize] = if speed == 0.0 {
        Some(
            parameters
                .get(DriveField::YawRateMax)
                .ok_or(AircraftUnsupported::EffectsAndLifecycle)?,
        )
    } else {
        Some(speed)
    };
    for (destination, value) in gains.iter_mut().zip(record.gains) {
        if value != 0.0 {
            *destination = Some(value);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChaseHeightControl {
    BuildingClearance,
    Hover(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChaseActions {
    pub startup_requested: bool,
    pub spotlight_enabled: bool,
    pub aim: Vec3,
    pub teleport: Option<Vec3>,
    pub scaled_rotor_position: Option<Vec3>,
    pub opacity_request: Option<(f32, f32)>,
    pub horizontal_vector: Vec3,
    pub speed_argument: f32,
    pub seconds: f32,
    pub height_control: ChaseHeightControl,
}

pub fn chase_actions(
    position: Vec3,
    target: ChaseTarget,
    seconds: f32,
    opacity: f32,
    fade_duration: f32,
) -> ChaseActions {
    let lead = super::HelicopterChaseGeometry::for_target(target, seconds).lead_position;
    let vector = [lead[0] - position[0], 0.0, lead[2] - position[2]];
    let distance = (f64::from(vector[0]).powi(2) + f64::from(vector[2]).powi(2)).sqrt();
    // §4.2.2 explicitly normalizes before branching, including at distance zero.
    let direction: Vec3 = vector.map(|v| (f64::from(v) / distance) as f32);
    let teleport = (distance > 192.0 && opacity == 0.0).then(|| {
        [
            lead[0] - direction[0] * 128.0,
            position[1],
            lead[2] - direction[2] * 128.0,
        ]
    });
    let opacity_request = if distance > 192.0 && opacity == 0.0 {
        Some((1.0, fade_duration))
    } else if distance > 192.0 && opacity == 1.0 {
        Some((0.0, fade_duration))
    } else {
        None
    };
    ChaseActions {
        startup_requested: true,
        spotlight_enabled: true,
        aim: lead,
        scaled_rotor_position: teleport.map(|p| p.map(|v| v * f32::from_bits(0x40555555))),
        teleport,
        opacity_request,
        horizontal_vector: vector,
        speed_argument: 2.0,
        seconds,
        height_control: if distance > 32.0 {
            ChaseHeightControl::BuildingClearance
        } else {
            ChaseHeightControl::Hover(lead[1] + 20.0)
        },
    }
}

pub fn helicopter_physics_seconds(
    milliseconds: f32,
    multiplier: f32,
    player_driven: bool,
    chase_target_contains_player_drive: bool,
) -> f32 {
    crate::automata::airplane::physics_seconds(
        milliseconds,
        (!player_driven && !chase_target_contains_player_drive).then_some(multiplier),
    )
}

pub trait SpotlightEffects {
    fn present(&self, index: usize) -> bool;
    fn environment_mode(&mut self) -> u32;
    fn active(&mut self, index: usize) -> bool;
    fn start(&mut self, index: usize, argument: u32);
    fn stop(&mut self, index: usize, argument: u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotlightStage {
    MissingPair,
    /// AF5 geometry must follow even when activation was not requested.
    NeedsGeometry {
        player_drive_state_present: bool,
    },
}

pub const TRANSFORM_OFFSETS: [u32; 10] = [0x94, 0x90, 0xA8, 0xAC, 0xB0, 0x98, 0x9C, 0xB4, 0xA0, 0xA4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeResource {
    State,
    Parameters,
    Transform(u32),
    Controls,
}

impl RuntimeResource {
    pub fn allocation_size(self) -> Option<usize> {
        match self {
            Self::State => Some(0x1D0),
            Self::Parameters => Some(0x7C),
            Self::Transform(_) => Some(0x94),
            Self::Controls => None, // AF4.
        }
    }
}

pub trait RuntimeAllocator {
    fn allocate(&mut self, resource: RuntimeResource) -> bool;
}

pub struct CoreAllocator;
impl RuntimeAllocator for CoreAllocator {
    fn allocate(&mut self, _: RuntimeResource) -> bool {
        true
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeOwnership {
    pub parameters: bool,
    pub transforms: [bool; 10],
    pub state_word: u32,
    pub completion_word: Option<u32>,
}

/// Original allocations have no clean false/rollback path. This diagnostic
/// describes an unusable partial runtime, never a successful inert rotor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationFault(pub RuntimeResource);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupAction {
    StopEffect {
        offset: u32,
        argument: u32,
    },
    ReleaseRetainedEffect(u32),
    Embedded {
        offset: u32,
        operation: u32,
        argument: Option<u32>,
    },
    Free(RuntimeResource),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownActions {
    pub base_result: bool,
    pub actions: Vec<CleanupAction>,
}
