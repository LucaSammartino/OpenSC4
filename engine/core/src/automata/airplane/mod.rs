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

mod closure;
pub use closure::*;

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

#[derive(Clone, Debug)]
pub struct Airplane {
    pub record: AircraftPlacementRecord,
    pub drive_parameters: AirplaneDriveParameters,
    pub player_parameters: AirplaneDriveParameters,
    phase: Option<FlightPhase>,
    ammunition: Option<i32>,
    pub loiter_remaining_seconds: f32,
    pub water_drop_active: bool,
    pub dispatch_request: Option<u32>,
    /// Input from the aircraft player-drive flag, independent of the goal stack.
    pub player_driven: bool,
    pub velocity: Vec3,
    pub angular_fields: Vec3,
    pub player_gains: [f32; 12],
    pub control_fields: [f32; 4],
    pub fire_search_anchor: Vec3,
    pub dispatch_state: u32,
    /// Retained references, in offset order 208/20C/210/214; never borrowed pointers.
    pub retained_effects_present: [bool; 4],
    pub fire_filter_present: bool,
}

impl Default for Airplane {
    fn default() -> Self {
        Self {
            record: AircraftPlacementRecord::default(),
            drive_parameters: AirplaneDriveParameters::default(),
            player_parameters: AirplaneDriveParameters::default(),
            phase: Some(FlightPhase::Phase0),
            ammunition: Some(3),
            loiter_remaining_seconds: 0.0,
            water_drop_active: false,
            dispatch_request: None,
            player_driven: false,
            velocity: [0.0; 3],
            angular_fields: [0.0; 3],
            player_gains: [
                0x3ECCCCCD, 0x3E99999A, 0x3E4CCCCD, 0x3F800000, 0x3F000000, 0x3E4CCCCD,
                0x3DCCCCCD, 0x3E99999A, 0x3DCCCCCD, 0x3E4CCCCD, 0x3E4CCCCD, 0x3DCCCCCD,
            ].map(f32::from_bits),
            control_fields: [0.0; 4],
            fire_search_anchor: [-1.0, -2.0, -3.0],
            dispatch_state: 0,
            retained_effects_present: [false; 4],
            fire_filter_present: false,
        }
    }
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
    pub fn ammunition(&self) -> Option<i32> {
        self.ammunition
    }
    pub fn set_ammunition(&mut self, ammunition: i32) {
        self.ammunition = Some(ammunition);
    }

    /// Parameter and placement portion of Init. AF4 blocks native Init closure.
    pub fn init_data(&mut self, ai: Option<&dyn DriveParameterSource>, player: Option<&dyn DriveParameterSource>) {
        self.drive_parameters = AirplaneDriveParameters::load(ai);
        self.player_parameters = AirplaneDriveParameters::load(player);
        self.record.init_placement();
    }

    /// aircraft §3.5: same-phase requests have no side effects.
    pub fn set_airplane_state(&mut self, next: FlightPhase) -> Option<FlightPhaseChange> {
        if self.phase == Some(next) {
            return None;
        }
        let previous = self.phase.replace(next);
        if next == FlightPhase::Phase1 {
            self.ammunition = Some(3);
        }
        use super::SoundRequest::{End, Start};
        let player = self.player_driven;
        let sounds = if !player {
            Vec::new()
        } else {
            match (next, previous) {
                (FlightPhase::Phase0, _) => super::aircraft_actions::stopped_sounds(),
                (FlightPhase::Phase1, Some(FlightPhase::Phase0)) => vec![Start(9)],
                (FlightPhase::Phase1, Some(FlightPhase::Phase3)) => vec![End(13), Start(13)],
                _ => Vec::new(),
            }
        };
        Some(FlightPhaseChange {
            previous,
            current: next,
            sounds,
            engine_notifications: Vec::new(),
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
        // §3.3.1: Read does not reset unsaved countdown/drop/ammunition.
        self.record = record;
        true
    }

    pub fn pursue_goal(&mut self) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        match self.base().states.last().map(|s| s.state_type) {
            Some(state_type::CHASE | 58) => {
                // automata §6.5: name-only CROP_DUST has no control law;
                // the literal specified default is PopState, just like CHASE.
                self.base_mut().pop_state();
                Ok(AircraftGoalOutcome::Popped)
            }
            Some(53) => Ok(self.visit_loiter()),
            Some(50 | 51 | 57) => Err(AircraftUnsupported::GoalControl),
            _ => Ok(AircraftGoalOutcome::NoAircraftHandler),
        }
    }

    /// The airplane's documented LOITER push uses an infinite stack timeout;
    /// the separate two-second timer and ammo decrement end it (§6.6).
    pub fn begin_loiter(&mut self) {
        self.base_mut().push_state(53, SetStateArgs::default());
    }

    pub fn tick_loiter(&mut self, dt_ms: f32) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        self.tick_loiter_with_multiplier(dt_ms, None)
    }

    /// Countdown stage of animation, before pursuit; this is not a full AF2 tick.
    pub fn tick_loiter_with_multiplier(
        &mut self,
        dt_ms: f32,
        ai_multiplier: Option<f32>,
    ) -> Result<AircraftGoalOutcome, AircraftUnsupported> {
        let player = self.player_driven;
        let step = physics_seconds(dt_ms, if player { None } else { ai_multiplier });
        if self.loiter_remaining_seconds > 0.0 {
            self.loiter_remaining_seconds -= step;
        }
        if self.base().states.last().map(|s| s.state_type) != Some(53) {
            return Ok(AircraftGoalOutcome::NoAircraftHandler);
        }
        Ok(self.visit_loiter())
    }

    fn visit_loiter(&mut self) -> AircraftGoalOutcome {
        self.dispatch_request = Some(5);
        if !self.water_drop_active {
            self.water_drop_active = true;
            self.loiter_remaining_seconds = 2.0;
            self.ammunition = self.ammunition.map(|n| n.wrapping_sub(1));
        }
        if self.loiter_remaining_seconds <= 0.0 {
            self.water_drop_active = false;
            self.base_mut().pop_state();
            AircraftGoalOutcome::Popped
        } else {
            AircraftGoalOutcome::Waiting
        }
    }

    pub fn weapon_requests(
        &mut self,
        player: &mut super::PlayerWeaponRecord,
        effects: &mut dyn super::AircraftWeaponEffects,
    ) -> super::WeaponOutcome {
        super::aircraft_actions::weapon_requests(player, effects, &["jetbomb"])
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
    let conversion = f64::from(milliseconds) * f64::from(f32::from_bits(0x3A83126F));
    let stored = conversion as f32;
    let cap = f32::from_bits(0x3E4CCCCD);
    match ai_multiplier {
        Some(multiplier) => {
            let seconds = stored * multiplier;
            if seconds > cap { cap } else { seconds }
        }
        None => {
            // §3.4.1a: player cap compares the unrounded conversion.
            if conversion > f64::from(cap) { cap } else { stored }
        }
    }
}

/// Whole 3D pre-force limiter; the tiny-speed branch has priority over the cap.
pub fn limit_velocity(velocity: Vec3, max_speed: f32) -> Vec3 {
    let speed = length(velocity);
    let factor = if (speed as f32) < f32::from_bits(0x38D1B717) && speed > 0.0 {
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
    /// Assemble the settled force sum from the already scaled, stored lift
    /// vector. Constructing that vector from attitude remains AF2; this stage
    /// neither chooses a lift frame nor runs the environment corrections.
    pub fn force_with_lift(&self, lift: Vec3, orientation: Vec3, mass: f32, phase: u32) -> Vec3 {
        let rolling = self.rolling_resistance(mass, lift[1], phase);
        let drag = f64::from(self.drag_magnitude(rolling));
        std::array::from_fn(|i| {
            // Literal sum from §3.4.2: lift, thrust, gravity, opposing drag.
            (f64::from(lift[i])
                + f64::from(self.thrust) * f64::from(orientation[i])
                + if i == 1 { f64::from(self.gravity) } else { 0.0 }
                - drag * f64::from(self.direction[i])) as f32
        })
    }

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
