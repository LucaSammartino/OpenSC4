// SPDX-License-Identifier: GPL-3.0-or-later
//! Aircraft §§4.2.6–7 and 4.4: bounded rotor/goal stages, not complete motion.
use super::*;
use crate::automata::{EngineNotification, SoundRequest};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotorPower {
    pub engine_flag: u32,
    pub rotor_speed: f32,
    pub collective: f32,
    pub power: f32,
    pub vertical_velocity: f32,
    pub mass: f32,
    pub pitch: f32,
    pub horizontal_speed: f32,
    pub collective_fraction: f32,
    pub vertical_force: f32,
}

impl RotorPower {
    /// Finite-input contract of §4.2.6. Flags other than 1 can spin, but not power lift.
    pub fn update(&mut self, seconds: f32) {
        let h = f64::from(seconds);
        let e = f64::from(self.rotor_speed);
        if self.engine_flag == 0 {
            self.rotor_speed = (e - 100.0 * h).clamp(0.0, 400.0) as f32;
        } else if e < 400.0 {
            self.rotor_speed = (e + 100.0 * h).clamp(0.0, 400.0) as f32;
        }
        self.collective_fraction =
            ((f64::from(self.collective) + 100.0) * f64::from(f32::from_bits(0x3BA3D70A))) as f32;
        self.vertical_force = 0.0;
        let powered = self.engine_flag == 1;
        if powered {
            let factor = (f64::from(self.rotor_speed) * f64::from(f32::from_bits(0x3B23D70A))).clamp(0.0, 1.0);
            self.vertical_force = (f64::from(self.collective_fraction) * f64::from(self.power) * factor) as f32;
        }
        let vy = f64::from(self.vertical_velocity);
        let k = f32::from_bits(if powered { 0x3C9374BC } else { 0x3CC0AD04 });
        let q = vy * f64::from(k) * vy * f64::from(self.mass);
        let correction = if powered {
            if vy >= 0.0 {
                -q * 4.0
            } else {
                q
            }
        } else if self.pitch < 0.0 && self.horizontal_speed > 10.0 {
            q * f64::from(self.collective_fraction) * 4.0
        } else {
            self.collective_fraction = self.collective_fraction.clamp(0.0, 1.0);
            (1.0 - f64::from(self.collective_fraction)) * q
        };
        self.vertical_force = (f64::from(self.vertical_force) + correction) as f32;
    }
}

// SPEC-GAP: aircraft §9.1 requires rotor attitude/motion/collision stage observations.
/// The unresolved motion kernels are supplied by a future adapter, never guessed here.
pub trait RotorFrameStages {
    fn engine_power(&mut self, seconds: f32);
    fn attitude_slide(&mut self, seconds: f32);
    fn horizontal_motion(&mut self, seconds: f32);
    fn vertical_motion(&mut self, seconds: f32);
    fn transform_xz(&self) -> [f32; 2];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotorFrameState {
    pub phase: u32,
    pub copied_xz: [f32; 2],
    pub applied_xz: [f32; 2],
    pub phase1_seconds: f32,
}
impl RotorFrameState {
    pub fn next_frame(&mut self, seconds: f32, stages: &mut dyn RotorFrameStages) {
        if self.phase == 6 {
            return;
        }
        stages.engine_power(seconds);
        stages.attitude_slide(seconds);
        stages.horizontal_motion(seconds);
        stages.vertical_motion(seconds);
        self.copied_xz = stages.transform_xz();
        if self.applied_xz != self.copied_xz {
            self.applied_xz = self.copied_xz;
        }
        if self.phase == 1 {
            self.phase1_seconds += seconds;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartActions {
    pub sounds: Vec<SoundRequest>,
    pub notifications: Vec<EngineNotification>,
}
impl Helicopter {
    pub fn start_stage(
        &mut self,
        rotor_speed: f32,
        vertical_velocity: f32,
    ) -> Result<StartActions, AircraftUnsupported> {
        let phase = self.phase.ok_or(AircraftUnsupported::EffectsAndLifecycle)?;
        let mut result = StartActions::default();
        let next = match phase {
            FlightPhase::Phase0 => {
                if self.player_driven {
                    result.sounds.push(SoundRequest::Start(9));
                }
                FlightPhase::Phase4
            }
            FlightPhase::Phase4 if rotor_speed >= 400.0 && vertical_velocity > 0.0 => FlightPhase::Phase1,
            _ => return Ok(result),
        };
        self.engine_enabled = Some(true);
        self.phase = Some(next);
        result.notifications = vec![
            EngineNotification::Base(true),
            EngineNotification::Embedded {
                offset: 0x240,
                operation: 0x18,
                argument: 0,
            },
            EngineNotification::Embedded {
                offset: 0x22C,
                operation: 0x1C,
                argument: 1,
            },
        ];
        Ok(result)
    }

    /// Complete only the home path. Other visits delegate to the unresolved normal tick.
    pub fn physics_home_stage(
        &mut self,
        seconds: f32,
        pitch_factor: f32,
        normal_tick: impl FnOnce(&mut Self, f32) -> Result<Option<FlightPhaseChange>, AircraftUnsupported>,
    ) -> Result<Option<FlightPhaseChange>, AircraftUnsupported> {
        if self.landing_latch != Some(true) {
            return normal_tick(self, seconds);
        }
        self.ai_control_reset(seconds, pitch_factor)?;
        let goal = self.base().states.last().map_or(0, |s| s.state_type);
        let mut phase_change = None;
        if matches!(goal, 0 | 0x32) {
            let position = self.base().vectors[0];
            let home = self.record.home_position;
            let distance = distance(position, home);
            if distance < 1.0 {
                self.base_mut().vectors[0] = home;
                phase_change = self.set_helicopter_state(FlightPhase::Phase0);
            } else {
                let blend = (seconds * f32::from_bits(0x3EAAAAAB)).min(1.0);
                self.base_mut().vectors[0] = std::array::from_fn(|i| {
                    ((1.0 - f64::from(blend)) * f64::from(position[i]) + f64::from(blend) * f64::from(home[i])) as f32
                });
            }
        } else {
            self.landing_latch = Some(false);
        }
        self.scaled_rotor_position =
            Some(self.base().vectors[0].map(|v| (f64::from(v) * f64::from(f32::from_bits(0x40555555))) as f32));
        Ok(phase_change)
    }

    pub fn shutdown_with_claims(
        &mut self,
        claims: &mut Vec<Vec3>,
        base: impl FnOnce(&mut Self, &mut Vec<Vec3>) -> bool,
    ) -> ShutdownActions {
        let destination = self.base().vectors[2];
        let sentinel = [-1.0, -2.0, -3.0];
        if !same_claim(destination, sentinel) {
            claims.retain(|claim| !same_claim(*claim, destination));
            self.base_mut().vectors[2] = sentinel;
        }
        let result = base(self, claims);
        self.shutdown_after_base(result)
    }
}

fn same_claim(a: Vec3, b: Vec3) -> bool {
    a.into_iter()
        .zip(b)
        .all(|(a, b)| (f64::from(a) - f64::from(b)).abs() < f64::from(f32::from_bits(0x38D1B717)))
}
fn distance(a: Vec3, b: Vec3) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (f64::from(a) - f64::from(b)).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[derive(Clone, Debug, PartialEq)]
pub enum HelicopterGoalAction {
    Dispatch(u32),
    Pop,
    Reset,
    Start,
    Spotlight(bool),
    StopHorizontal,
    MoveXZ { displacement: Vec3, scalar: f32 },
    DestinationControl(Vec3),
    Hover(f32),
    BuildingLift,
    HomeComplete,
    AimGround(Vec3),
    RespondToCrimeScene,
    Chase,
}
impl Helicopter {
    /// Bounded §4.4 decisions; completion/steering service results are independent inputs.
    pub fn goal_actions(
        &self,
        destination_reached: bool,
        chase_valid: bool,
    ) -> Result<Vec<HelicopterGoalAction>, AircraftUnsupported> {
        use HelicopterGoalAction::*;
        let phase = self.phase.ok_or(AircraftUnsupported::EffectsAndLifecycle)?;
        let goal = self.base().states.last().map_or(0, |s| s.state_type);
        let mut actions = vec![];
        match goal {
            0x33 => {
                actions.extend([Dispatch(3), Start, Spotlight(false)]);
                if destination_reached {
                    actions.push(Pop);
                } else {
                    actions.push(DestinationControl(self.base().vectors[2]));
                }
                actions.push(BuildingLift);
                return Ok(actions);
            }
            0x34 => {
                return Ok(if chase_valid {
                    vec![Start, Spotlight(true), Chase]
                } else {
                    vec![Pop]
                })
            }
            0x35 => {
                actions.extend([Dispatch(5), Start, Spotlight(true), AimGround(self.base().vectors[2])]);
                if self.base().states.iter().any(|s| s.state_type == 0x36) {
                    actions.push(RespondToCrimeScene);
                }
                actions.extend([StopHorizontal, BuildingLift]);
                return Ok(actions);
            }
            // SPEC-GAP: aircraft §9.2 requires the claim/crime/rescue service branches.
            0x36..=0x38 => return Err(AircraftUnsupported::GoalControl),
            0 => {}
            _ => {
                if goal != 0x32 {
                    actions.push(Pop);
                }
                if phase == FlightPhase::Phase0 {
                    actions.extend([Dispatch(0), Pop]);
                } else {
                    actions.push(Dispatch(6));
                }
            }
        }
        if phase == FlightPhase::Phase0 {
            actions.extend([Reset, Spotlight(false)]);
        } else {
            actions.extend([Start, Spotlight(false)]);
            let position = self.base().vectors[0];
            let home = self.record.home_position;
            let delta = std::array::from_fn(|i| home[i] - position[i]);
            let horizontal = (f64::from(delta[0]).powi(2) + f64::from(delta[2]).powi(2)).sqrt();
            if horizontal <= 8.0 {
                actions.push(StopHorizontal);
            } else {
                actions.push(MoveXZ {
                    displacement: delta,
                    scalar: 10.0,
                });
            }
            if horizontal <= 48.0 {
                actions.push(Hover((f64::from(home[1]) + horizontal - 2.0) as f32));
            } else {
                actions.push(BuildingLift);
            }
            if distance(position, home) < 24.0 {
                actions.push(HomeComplete);
            }
        }
        Ok(actions)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchAction {
    /// Owner request ("riot_police_repulsor",0), then service operation +0x14.
    AddRiotRepulsor,
    /// Owner operation +0x14(), then service operation +0x38.
    RemoveRiotRepulsor,
    Notify(u32, u32),
}
impl Helicopter {
    pub fn set_dispatch_state(&mut self, state: u32, service_present: bool, mut emit: impl FnMut(DispatchAction)) {
        if self.dispatch_state == Some(state) {
            return;
        }
        if service_present && self.base().states.iter().any(|s| s.state_type == 0x36) {
            emit(if state == 5 {
                DispatchAction::AddRiotRepulsor
            } else {
                DispatchAction::RemoveRiotRepulsor
            });
        }
        self.dispatch_state = Some(state);
        emit(DispatchAction::Notify(7, state));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelicopterStateCallback {
    pub base_result: bool,
    pub limits: Option<[f32; 2]>,
    // SPEC-GAP: aircraft §9.3 leaves the native refresh helper's internals unobserved.
    pub refresh_helper: bool,
}
impl Helicopter {
    pub fn enter_state(
        &mut self,
        state: Option<u32>,
        player: Option<&dyn PlayerParameterSource>,
        base: impl FnOnce() -> bool,
    ) -> Result<HelicopterStateCallback, AircraftUnsupported> {
        let base_result = base();
        let is_player = state == Some(0x10);
        if is_player {
            self.on_player_take_over(player)?;
        }
        Ok(HelicopterStateCallback {
            base_result,
            limits: is_player.then_some([-100.0, 100.0]),
            refresh_helper: is_player,
        })
    }

    pub fn exit_state(
        &mut self,
        state: Option<u32>,
        ai: Option<&dyn DriveParameterSource>,
        service_present: bool,
        base: impl FnOnce() -> bool,
        emit: impl FnMut(DispatchAction),
    ) -> bool {
        let base_result = base();
        if state == Some(0x10) {
            self.drive_parameters.load(ai);
            let mass = self.drive_parameters.get(DriveField::Mass);
            self.runtime_masses = [mass; 2];
        } else if state == Some(0x35) {
            self.set_dispatch_state(2, service_present, emit);
        }
        base_result
    }
}
