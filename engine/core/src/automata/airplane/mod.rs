// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled numerical stages of cSC4Airplane (aircraft.md §§3.1, 3.4.1–3.4.2).
//!
//! Airplane owns its shared state stack and delegates the nested record codec
//! to super::aircraft. These
//! stages expose candidate values without committing an incomplete flight tick:
//! AF2 still blocks attitude, lift frame, surface and collision corrections.
//! f64 intermediates approximate x87 extended arithmetic; conformance uses the
//! spec tolerances rather than claiming bit-exact transcendental results.

use super::{
    state_type, AircraftGoalOutcome, AircraftPlacementRecord, AircraftUnsupported, AircraftWorld, AutomatonBase,
    DriveParameterSource, FlightPhase, FlightPhaseChange, SetStateArgs, Vec3, CLSID_AIRPLANE,
};
use crate::stream::{InStream, OutStream};

#[derive(Clone, Debug, PartialEq)]
pub struct AirplaneDriveParameters(pub [f32; 18]);

impl Default for AirplaneDriveParameters {
    fn default() -> Self {
        Self(
            [
                0x42C80000, 0x3F800000, 0x41C80000, 0x43160000, 0x3E860A92, 0x3D860A92, 0x3F490FDB, 0x3EC90FDB,
                0x3F060A92, 0x3D32B8C3, 0x461C4000, 0x461C4000, 0x3ECCCCCD, 0x3D23D70A, 0x3F060A92, 0x3DA3D70A,
                0x3F2E147B, 0x00000000,
            ]
            .map(f32::from_bits),
        )
    }
}

impl AirplaneDriveParameters {
    /// aircraft.md §3.1: nearest whole record wins, including an all-zero
    /// record. The spec gives no cycle detection: recordless cycles do not
    /// terminate. No property ID or per-field inheritance is inferred.
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
        // §3.1 explicitly excludes short raw records. An empty Lua table is
        // imported as 18 zeros, never as an empty slice; do not zero-pad here.
        assert!(list.len() >= 18, "airplane record requires 18 binary32 fields");
        let mut values = Self::default().0;
        for (i, value) in values.iter_mut().enumerate() {
            let given = if i == 2 || i == 3 { list[i] * 1000.0 } else { list[i] };
            if given != 0.0 {
                *value = given;
            }
        }
        for (derived, parent, factor) in [
            (5, 4, 0.25),
            (7, 6, 0.5),
            (9, 8, f32::from_bits(0x3DAAAAAB)),
            (11, 10, 1.0),
            (13, 12, 0.1),
        ] {
            if list[derived] == 0.0 {
                values[derived] = values[parent] * factor;
            }
        }
        Self(values)
    }
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

    /// Full UpdatePhysics remains open under AF2. Use the numerical helpers
    /// below to inspect its settled candidate/scalar stages independently.
    /// No mutation or partial flight tick on failure. Time is seconds.
    pub fn update_physics(&mut self, _dt_seconds: f32, _world: &dyn AircraftWorld) -> Result<(), AircraftUnsupported> {
        Err(AircraftUnsupported::FlightModel)
    }
}

pub const AIRPLANE_FIELDS: [&str; 18] = [
    "Mass",
    "WingArea",
    "TakeOffSpeed",
    "MaxSpeed",
    "ElevatorMax",
    "ElevatorChangeRate",
    "AileronMax",
    "AileronChangeRate",
    "RudderMax",
    "RudderChangeRate",
    "ThrottleMax",
    "ThrottleChangeRate",
    "FlapsMax",
    "FlapsChangeRate",
    "StallAngle",
    "LiftFactorLevelFlight",
    "LiftFactorMaxClimb",
    "LiftFactorFullStall",
];

/// A present Lua table always imports a complete record, including when empty.
/// Table absence is represented separately by DriveParameterSource::None.
pub fn import_airplane_record(mut field: impl FnMut(&str) -> Option<f32>) -> [f32; 18] {
    AIRPLANE_FIELDS.map(|name| field(name).unwrap_or(0.0))
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| f64::from(x) * f64::from(y)).sum()
}

fn length(v: Vec3) -> f64 {
    dot(v, v).sqrt()
}

/// Animation callers retain milliseconds for callbacks, but use this single
/// converted/capped step for pursuit and physics. None means player drive.
pub fn physics_seconds(milliseconds: f32, ai_multiplier: Option<f32>) -> f32 {
    let seconds = milliseconds * f32::from_bits(0x3A83126F);
    let seconds = match ai_multiplier {
        Some(multiplier) => seconds * multiplier,
        None => seconds,
    };
    let cap = f32::from_bits(0x3E4CCCCD);
    // No lower clamp or subdivision. Non-finite caller behavior is not settled
    // by §3.4.1; literal comparison preserves NaN rather than sanitizing it.
    if seconds > cap {
        cap
    } else {
        seconds
    }
}

/// Whole 3D pre-force limiter; the tiny-speed branch has priority over the cap.
pub fn limit_velocity(velocity: Vec3, max_speed: f32) -> Vec3 {
    let speed = length(velocity);
    let factor = if speed < f64::from(f32::from_bits(0x38D1B717)) && speed > 0.0 {
        1.0 / speed
    } else if speed as f32 > max_speed && speed > 0.0 {
        f64::from(max_speed) / speed
    } else {
        return velocity;
    };
    velocity.map(|component| (f64::from(component) * factor) as f32)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateMotion {
    pub position: Vec3,
    pub velocity: Vec3,
}

/// Integrate an already limited velocity with the once-evaluated force.
/// No terrain, runway, collision, clipping or attitude work is implied.
pub fn integrate_candidate(position: Vec3, velocity: Vec3, force: Vec3, mass: f32, seconds: f32) -> CandidateMotion {
    let mut result = CandidateMotion { position, velocity };
    for i in 0..3 {
        let old = f64::from(velocity[i]);
        let next = old + f64::from(force[i]) / f64::from(mass) * f64::from(seconds);
        result.velocity[i] = next as f32;
        result.position[i] = (f64::from(position[i]) + (old + next) * (f64::from(seconds) * 0.5)) as f32;
    }
    result
}

/// §3.4.1's final ground-phase projection, after the unresolved attitude update.
pub fn project_ground_velocity(velocity: Vec3, updated_orientation: Vec3, phase: u32) -> Vec3 {
    if phase > 1 {
        return velocity;
    }
    let along = dot(velocity, updated_orientation);
    updated_orientation.map(|component| (f64::from(component) * along) as f32)
}

/// The scalar portion of CalculateForces. Lift direction and angular motion
/// cannot be reconstructed from these terms alone (AF2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceScalars {
    pub speed: f32,
    pub direction: Vec3,
    pub angle: f32,
    pub coefficient: f32,
    pub pressure: f32,
    pub lift: f32,
    pub drag_normalization: f32,
    pub gravity: f32,
    pub thrust: f32,
}

pub fn lift_coefficient(angle: f32, stall_angle: f32, level: f32, climb: f32, full_stall: f32) -> f32 {
    let alpha = f64::from(angle);
    let sigma = f64::from(stall_angle);
    let level = f64::from(level);
    let climb = f64::from(climb);
    // Equality selects the lower segment, including sigma == A. No guards
    // for zero/out-of-range loaded stall angles are specified.
    if alpha <= sigma {
        (level + ((climb - level) / sigma) * alpha) as f32
    } else {
        (climb + ((alpha - sigma) * (f64::from(full_stall) - climb)) / (f64::from(f32::from_bits(0x3FC90FDB)) - sigma))
            as f32
    }
}

impl AirplaneDriveParameters {
    pub fn force_scalars(
        &self,
        velocity: Vec3,
        orientation: Vec3,
        throttle: f32,
        flaps: f32,
        resistance: f32,
        phase: u32,
    ) -> ForceScalars {
        let p = self.0.map(f64::from);
        let speed = length(velocity);
        let source = if speed < f64::from(f32::from_bits(0x38D1B717)) {
            orientation
        } else {
            velocity
        };
        let source_length = length(source);
        // No zero-vector fallback after substitution: a zero orientation is
        // deliberately not replaced by a conventional forward direction.
        let direction = source.map(|component| (f64::from(component) / source_length) as f32);
        let uy = f64::from(direction[1]);
        let dy = f64::from(orientation[1]);
        let inner = (((1.0 - dy * dy).sqrt() * uy + (1.0 - uy * uy).sqrt() * dy).clamp(-1.0, 1.0)) as f32;
        let angle = f64::from(inner)
            .acos()
            .clamp(0.0, f64::from(f32::from_bits(0x3FC90FDB))) as f32;
        let coefficient = lift_coefficient(angle, self.0[14], self.0[15], self.0[16], self.0[17]);
        let pressure = ((((p[12] * f64::from(flaps)) + 1.0) * speed) * speed) * f64::from(resistance) * 0.5;
        let k = f64::from(f32::from_bits(0x3DCCCCCD));
        let drag_normalization = (p[10] / ((((p[3] * p[3]) * f64::from(resistance)) * 0.5) * k)) as f32;
        let thrust = if phase <= 1 && throttle < 0.0 && dot(velocity, orientation) < 0.0 {
            0.0
        } else {
            (f64::from(throttle) * p[10]) as f32
        };
        ForceScalars {
            speed: speed as f32,
            direction,
            angle,
            coefficient,
            pressure: pressure as f32,
            lift: (f64::from(coefficient) * pressure) as f32,
            drag_normalization,
            gravity: (-p[0] * f64::from(f32::from_bits(0x411CCCCD))) as f32,
            thrust,
        }
    }
}

impl ForceScalars {
    /// Vertical lift is a stored binary32 component of the unresolved lift
    /// frame, not the total lift magnitude. Equality with weight has no R.
    pub fn rolling_resistance(&self, mass: f32, vertical_lift: f32, phase: u32) -> f32 {
        let weight = f64::from(self.gravity.abs());
        if phase <= 1 && f64::from(vertical_lift) < weight {
            ((1.0 - (1.0 / weight) * f64::from(vertical_lift))
                * f64::from(mass)
                * f64::from(self.speed)
                * f64::from(f32::from_bits(0x3DCCCCCD))) as f32
        } else {
            0.0
        }
    }

    pub fn drag_magnitude(&self, rolling_resistance: f32) -> f32 {
        (f64::from(rolling_resistance)
            + (f64::from(self.angle) * f64::from(self.angle) + f64::from(f32::from_bits(0x3DCCCCCD)))
                * f64::from(self.pressure)
                * f64::from(self.drag_normalization)) as f32
    }
}
