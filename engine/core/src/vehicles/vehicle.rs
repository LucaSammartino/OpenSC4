// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::{SetStateArgs, StateExtra, Vec3, VehicleRecord};
use crate::stream::{InStream, OutStream};

use super::*;

pub const CLSID_VEHICLE: u32 = 0x896E_714A;
pub const COLLISION_MESSAGE: u32 = 0x49E6_4802;

/// Terrain, controllers, renderer and effects are foreign. These methods
/// never run a vehicle, train or traffic owner in the host.
pub trait VehicleWorld: TrafficWorld {
    fn in_city(&mut self, position: Vec3) -> bool;
    fn heights(&mut self, position: Vec3) -> (f32, f32);
    fn road_height(&mut self, position: Vec3) -> f32;
    fn controller_point(&mut self) -> Vec3;
    fn controller_continues(&mut self, position: Vec3) -> bool;
    fn traffic_value(&mut self, position: Vec3) -> u32;
    fn splash(&mut self, position: Vec3) -> Option<u64>;
    fn destroy_effect(&mut self, effect: u64);
    fn post_message(&mut self, id: u32);
    fn create_accident(&mut self, position: Vec3);
    fn crash_transform(&mut self);
    fn set_visible(&mut self, visible: bool);
    fn notify_occupant(&mut self);
    fn refresh_behavior(&mut self);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    pub record: VehicleRecord,
    pub traffic: TrafficRuntime,
    /// Derived speed (+0x94), not the speed tween, not serialized.
    pub measured_speed: f32,
    pub splash_effect: Option<u64>,
    /// Known externally supplied fade tuning, in seconds. VH8: source absent.
    pub splash_seconds: Option<f32>,
    /// The two PullOver vectors are transient and never added to its record.
    pub pull_heading: Vec3,
    pub pull_sideways: Vec3,
    pub crash_elapsed_ms: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            record: VehicleRecord::new(),
            traffic: TrafficRuntime::default(),
            measured_speed: 0.0,
            splash_effect: None,
            splash_seconds: None,
            pull_heading: [1.0, 0.0, 0.0],
            pull_sideways: [0.0, 0.0, 1.0],
            crash_elapsed_ms: 0.0,
        }
    }
}

impl Vehicle {
    pub fn traffic_owner(&mut self) -> TrafficOwner<'_> {
        TrafficOwner {
            record: &mut self.record.traffic,
            runtime: &mut self.traffic,
        }
    }

    /// This is a record-backed portable owner, not a claim that unspecified
    /// native constructor flags equal zero (VH4).
    pub fn init(&mut self) -> Result<()> {
        Err(Unsupported::InitializationVh4)
    }

    /// Read before Init: no city calls or world-size allocations. Atomic on
    /// every malformed/truncated stream. Live effects must be shut down first.
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        if self.traffic.initialized || self.splash_effect.is_some() {
            return false;
        }
        let Some(record) = VehicleRecord::read(input) else {
            return false;
        };
        if input.error() != 0 {
            return false;
        }
        *self = Self {
            record,
            ..Self::default()
        };
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        self.record.write(output)
    }

    pub fn shutdown(&mut self, world: &mut impl VehicleWorld) {
        if let Some(effect) = self.splash_effect.take() {
            world.destroy_effect(effect);
        }
        self.traffic_owner().shutdown(world);
    }

    /// Effects/SFX are interface operations. The original owner is never
    /// invoked to handle a state. The shared stack enforces final semantics.
    pub fn set_state(&mut self, kind: u32, args: SetStateArgs, world: &mut impl VehicleWorld) -> Result<bool> {
        if self
            .record
            .traffic
            .base
            .states
            .last()
            .map(|s| s.base.is_final)
            .unwrap_or(false)
        {
            return Ok(false);
        }
        if kind == 14 {
            return Err(Unsupported::PlummetEntryVh8);
        }
        if let Some(old) = self.record.traffic.base.states.last() {
            world.effects(EffectAction::ExitState(old.state_type));
        }
        if let Some(effect) = self.splash_effect.take() {
            world.destroy_effect(effect);
        }
        self.record.traffic.base.set_state(kind, args);
        self.enter_state(kind, world);
        Ok(true)
    }

    pub fn push_state(&mut self, kind: u32, args: SetStateArgs, world: &mut impl VehicleWorld) -> Result<bool> {
        if kind == 0 && args.controller == 0 {
            return self.set_state(kind, args, world);
        }
        if kind == 14 {
            return Err(Unsupported::PlummetEntryVh8);
        }
        self.record.traffic.base.push_state(kind, args);
        self.enter_state(kind, world);
        Ok(true)
    }

    fn enter_state(&mut self, kind: u32, world: &mut impl VehicleWorld) {
        if kind == 30 {
            tween_speed(&mut self.record.traffic.base, 0.0, 0.5);
        }
        world.effects(EffectAction::EnterState(kind));
    }

    pub fn pop_state(&mut self, world: &mut impl VehicleWorld) -> bool {
        let old = self.record.traffic.base.states.last().map(|s| s.state_type);
        let popped = self.record.traffic.base.pop_state();
        if let Some(old) = old {
            world.effects(EffectAction::ExitState(old));
            if old == 14 {
                if let Some(effect) = self.splash_effect.take() {
                    world.destroy_effect(effect);
                }
            }
        }
        if popped {
            if let Some(top) = self.record.traffic.base.states.last() {
                world.effects(EffectAction::EnterState(top.state_type));
            }
        }
        popped
    }

    pub fn path_tick(&mut self, dt_ms: f32, movement: Movement, world: &mut impl VehicleWorld) -> Result<()> {
        if let Some(speed) = self.traffic_owner().path_tick(dt_ms, movement, true, world)? {
            self.measured_speed = speed;
        }
        Ok(())
    }

    /// vehicles §2 dispatcher. `moves` and `held` are slot 86 and slot 165
    /// results supplied by a caller, rather than guessed native defaults.
    pub fn animation_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        moves: bool,
        held: bool,
        world: &mut impl VehicleWorld,
    ) -> Result<()> {
        advance_tweens(&mut self.record.traffic.base, dt_ms);
        world.effects(EffectAction::Update);
        if moves && !self.state_tick(dt_ms, movement, world)? {
            if self.record.traffic.base.automata_flags & REACHED == 0 {
                if !matches!(self.record.traffic.network, crate::stream::RefValue::Null)
                    && self.traffic.has_tile_position
                    && self.traffic.has_path
                    && self.record.traffic.base.automata_flags & STRAIGHT == 0
                {
                    self.path_tick(dt_ms, movement, world)?;
                } else {
                    self.traffic_owner().straight_tick(dt_ms, movement.speed_scale, world);
                }
            } else if self.record.traffic.base.automata_flags & REMOVE == 0 && !held {
                self.traffic_owner().remove(world);
            }
        }
        // §6.5 specifies behavior refresh only in IDLE, not every frame.
        world.event(0, 0);
        Ok(())
    }

    pub fn state_tick(&mut self, dt_ms: f32, movement: Movement, world: &mut impl VehicleWorld) -> Result<bool> {
        let kind = self
            .record
            .traffic
            .base
            .states
            .last()
            .map(|s| s.state_type)
            .unwrap_or(0);
        match kind {
            1 => {
                self.traffic_owner().remove(world);
                Ok(true)
            }
            7 => {
                if self.record.traffic.base.automata_flags & REMOVE != 0 {
                    return Ok(false);
                }
                if self.pop_state(world) {
                    Ok(false)
                } else {
                    world.refresh_behavior();
                    Ok(true)
                }
            }
            10 => Ok(self.record.traffic.base.automata_flags & REMOVE == 0),
            13 => Ok(self
                .traffic_owner()
                .blocked_wait_tick(dt_ms, movement.speed_scale, world)),
            14 => self.plummet_tick(dt_ms, world),
            16 => Err(Unsupported::PlayerDrivingVh1),
            20 => {
                self.collide_tick(dt_ms, world);
                Ok(self.record.traffic.base.automata_flags & REMOVE == 0)
            }
            21 => {
                self.pull_over_tick(dt_ms, world);
                Ok(true)
            }
            30 => {
                if tween_value(&self.record.traffic.base.f3[..4]) > 0.0 {
                    self.path_tick(dt_ms, movement, world)?;
                }
                self.countdown(dt_ms, world);
                Ok(true)
            }
            // These handlers belong to the automaton base but have insufficient
            // portable controller/model details here (VH9), not default seeks.
            11 | 15 | 18 | 19 => Err(Unsupported::BaseStateVh9),
            _ => {
                self.countdown(dt_ms, world);
                Ok(false)
            }
        }
    }

    fn countdown(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        let expired = if let Some(top) = self.record.traffic.base.states.last_mut() {
            if top.base.countdown_ms > 0.0 {
                top.base.countdown_ms -= dt_ms;
                top.base.countdown_ms <= 0.0
            } else {
                false
            }
        } else {
            false
        };
        if expired {
            self.pop_state(world);
        }
    }

    pub fn collide_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        if self.record.traffic.base.automata_flags & REMOVE != 0 {
            return;
        }
        let flags = self.record.traffic.base.occupant.flags;
        if flags & 2 != 0 {
            world.set_visible(false);
        }
        world.crash_transform();
        if flags & 1 != 0 {
            world.notify_occupant();
        }
        self.crash_elapsed_ms += dt_ms;
        if self.crash_elapsed_ms > 100.0 {
            world.event(9, 0);
            self.crash_elapsed_ms = 0.0;
        }
        self.countdown(dt_ms, world);
    }

    /// The direction distribution is open (VH6). It is never replaced with a
    /// made-up RNG algorithm. A resolved direction can enter the known crash.
    pub fn handle_collision(&mut self) -> Result<()> {
        Err(Unsupported::CollisionDirectionVh6)
    }

    pub fn enter_collision(&mut self, direction: Vec3, world: &mut impl VehicleWorld) -> Result<bool> {
        world.post_message(COLLISION_MESSAGE);
        self.record.traffic.base.automata_flags |= 0x10000000;
        self.record.traffic.base.vectors[2] = super::unit(direction);
        self.set_state(
            20,
            SetStateArgs {
                timeout_ms: 5000.0,
                final_flag: 1,
                ..SetStateArgs::default()
            },
            world,
        )
    }

    pub fn update_after_damage(&mut self, world: &mut impl VehicleWorld) -> Result<()> {
        world.create_accident(self.record.traffic.base.vectors[0]);
        self.record.traffic.base.automata_flags |= 0x08000000;
        // [spec gap VH6] Event ID and damaged resource mapping are not supplied.
        Err(Unsupported::DamageModelsVh6)
    }
    pub fn update_motion_blur(&self) -> Result<()> {
        Err(Unsupported::MotionBlurVh6)
    }
    pub fn player_drive(&self) -> Result<()> {
        Err(Unsupported::PlayerDrivingVh1)
    }

    /// managers §5 slot 135: high self-controlling route ends fall. Returning
    /// true means a fall is required; initialization remains explicitly VH8.
    pub fn reached_route_destination(&mut self, world: &mut impl VehicleWorld) -> Result<bool> {
        let position = self.record.traffic.base.vectors[0];
        if self.record.traffic.base.automata_flags & 2 != 0 && world.in_city(position) {
            let (terrain, water) = world.heights(position);
            if (position[1] - terrain.max(water)).abs() > 5.0 {
                return Err(Unsupported::PlummetEntryVh8);
            }
        }
        Ok(false)
    }

    pub fn plummet_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) -> Result<bool> {
        let position = self.record.traffic.base.vectors[0];
        if self.record.traffic.base.automata_flags & REMOVE != 0 {
            world.move_models(position, self.record.traffic.base.vectors[2]);
            return Ok(true);
        }
        let Some(state) = self.record.traffic.base.states.last_mut() else {
            return Ok(false);
        };
        let StateExtra::T0E {
            descent,
            roll_rate,
            roll_centre,
            tumble_ms,
            post_splash_ms,
        } = &mut state.extra
        else {
            return Ok(false);
        };
        if *tumble_ms > 0.0 {
            *tumble_ms -= dt_ms;
            let mut facing = self.record.traffic.base.vectors[2];
            facing[1] += dt_ms * 0.01 * *roll_rate;
            if facing[1] < *roll_centre - 0.05 || facing[1] > *roll_centre + 0.05 {
                *roll_rate = -*roll_rate;
                facing[1] = facing[1].clamp(*roll_centre - 0.05, *roll_centre + 0.05);
            }
            self.record.traffic.base.vectors[2] = super::unit(facing);
            world.move_models(position, self.record.traffic.base.vectors[2]);
            return Ok(true);
        }
        let next = [position[0], position[1] + dt_ms * *descent, position[2]];
        let mut remove = false;
        if *post_splash_ms > 0.0 {
            *post_splash_ms -= dt_ms;
            if *post_splash_ms <= 0.0 {
                if let Some(effect) = self.splash_effect.take() {
                    world.destroy_effect(effect);
                }
                remove = true;
            }
        }
        *descent += dt_ms * -0.00083333347;
        if !world.in_city(next) {
            remove = true;
        } else {
            let (terrain, water) = world.heights(next);
            if terrain.max(water) >= next[1] && *post_splash_ms <= 0.0 && !remove {
                if terrain < next[1] {
                    let Some(seconds) = self.splash_seconds else {
                        return Err(Unsupported::SplashDurationVh8);
                    };
                    *descent = 0.0002777778;
                    self.splash_effect = world.splash(next);
                    *post_splash_ms = seconds * 1000.0;
                    let opacity = tween_value(&self.record.traffic.base.f3[4..]);
                    self.record.traffic.base.f3[4..].copy_from_slice(&[opacity, 0.0, seconds, 0.0]);
                } else {
                    *descent = 0.0002777778;
                    *post_splash_ms = 0.0;
                    remove = true;
                }
            }
        }
        if remove {
            self.traffic_owner().remove(world);
        }
        self.record.traffic.base.vectors[0] = next;
        world.move_models(next, self.record.traffic.base.vectors[2]);
        Ok(true)
    }

    /// PullOver's conflicting latch/counter descriptions are read literally
    /// as the detailed elapsed counter (VH8). Only its scalar is serialized.
    pub fn pull_over_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        if self.record.traffic.base.automata_flags & REMOVE != 0 {
            return;
        }
        let elapsed = match self.record.traffic.base.states.last().map(|s| &s.extra) {
            Some(StateExtra::T15(value)) => *value,
            _ => return,
        };
        let position = self.record.traffic.base.vectors[0];
        if elapsed == 0.0 {
            tween_speed(&mut self.record.traffic.base, f32::from_bits(0x3B88888A), 1.0);
            self.pull_heading = self.record.traffic.base.vectors[2];
            self.pull_sideways = [-self.pull_heading[2], self.pull_heading[1], self.pull_heading[0]];
            let delta = super::sub(position, world.controller_point());
            if delta.iter().zip(self.pull_sideways).map(|(a, b)| a * b).sum::<f32>() < 0.0 {
                self.pull_sideways = super::scale(self.pull_sideways, -1.0);
            }
        } else if elapsed < 1000.0 {
            let forward = tween_value(&self.record.traffic.base.f3[..4]) * dt_ms;
            if forward > 0.0 {
                let sideways = self.record.traffic.base.f1[0] * 1.5 * dt_ms / 1000.0;
                let mut next = super::add(
                    position,
                    super::add(
                        super::scale(self.pull_heading, forward),
                        super::scale(self.pull_sideways, sideways),
                    ),
                );
                next[1] = world.road_height(next);
                let facing = super::unit(super::sub(next, position));
                if facing[1].is_finite() {
                    self.record.traffic.base.vectors[2] = facing;
                }
                self.record.traffic.base.vectors[0] = next;
                self.measured_speed = 0.0;
                world.move_models(next, self.record.traffic.base.vectors[2]);
            }
        } else {
            self.record.traffic.base.vectors[2] = super::unit(self.pull_heading);
            world.move_models(position, self.record.traffic.base.vectors[2]);
            if world.traffic_value(position) > 3 {
                self.pop_state(world);
            }
            if !world.controller_continues(position) {
                self.pop_state(world);
            }
        }
        // The old state may have been popped (even twice). Never advance the
        // newly revealed state's timer in place of the original scalar.
        if let Some(top) = self.record.traffic.base.states.last_mut() {
            if let StateExtra::T15(value) = &mut top.extra {
                *value += dt_ms;
            }
        }
    }
}
