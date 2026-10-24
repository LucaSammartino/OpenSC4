// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4Pedestrian and cSC4PedestrianOneShot owners.
//! Records and inherited traffic movement are shared with automata/vehicles.

mod states;
#[cfg(test)]
mod tests;

use crate::automata::{AutomatonBase, PedestrianRecord, SetStateArgs, Vec3};
use crate::stream::{InStream, OutStream, RefValue};
use crate::vehicles::{
    advance_tweens, return_to_default_speed, tween_value, EffectAction, InitWorld, Movement, RouteStep,
    TrafficAutomaton, TrafficOwner, TrafficRuntime, VehicleRandom, VehicleWorld, SPEED_UNIT,
};

pub const CLSID_PEDESTRIAN: u32 = 0x896E_75AF;
pub const CLSID_ONE_SHOT: u32 = 0xAA09_DC28;
pub const RIOT_ANIMATION: u32 = 0x3AB6_0B61;
pub const IID_AUTOMATON: u32 = 0xA9B4_0F05;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    pub speed: (i32, i32),
    pub initial_parameter: (f32, f32),
    pub play_count: (u8, u8),
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            speed: (15, 20),
            initial_parameter: (-0.5, -0.5),
            play_count: (3, 10),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbor {
    pub id: u64,
    pub position: Vec3,
}

/// Foreign occupant, controller, pathfinder, renderer and network interfaces.
/// No method dispatches a pedestrian owner body in the host.
pub trait PedestrianWorld: VehicleWorld + InitWorld {
    fn nearby_automata(&mut self, occupant_type: u32, bounds: Bounds) -> Vec<Neighbor>;
    fn pending_push(&mut self, id: u64, push: Vec3);
    fn ground_position(&mut self, position: Vec3, flags: u32) -> Vec3;
    fn play_animation(&mut self, animation: u32);
    fn stopped_animation(&mut self);
    fn play_idle(&mut self);
    fn model_count(&mut self) -> usize;
    fn model_present(&mut self, index: usize) -> bool;
    fn instance_value(&mut self, index: usize, value: f32);
    fn exemplar_play_count(&mut self, property: u32) -> Vec<u8>;
    fn controller_attractor(&mut self, controller: &RefValue) -> Option<Vec3>;
    fn controller_target(&mut self, controller: &RefValue) -> Option<Bounds>;
    fn queue_step(&mut self, controller: &RefValue, position: Vec3) -> Option<Vec3>;
    fn queue_head(&mut self, controller: &RefValue) -> Option<Vec3>;
    fn find_path_to_box(&mut self, position: Vec3, target: Bounds) -> Option<Vec<RouteStep>>;
    fn return_to_source(&mut self);
    fn reaction_animations(&mut self, kind: u32) -> Vec<u32>;
    fn animation_finished(&mut self) -> bool;
    fn road_cell(&mut self, position: Vec3) -> Option<[i32; 2]>;
    fn walkable_heading(&mut self, cell: [i32; 2], heading: u8) -> bool;
    fn acquire_leader(&mut self, iid: u32) -> RefValue;
    fn leader_position(&mut self, leader: &RefValue) -> Option<Vec3>;
    fn ignite_target(&mut self, controller: &RefValue, cause: u32);
}

/// Foreign animated-model operations used by the one-shot's slot-72 body.
pub trait OneShotWorld: PedestrianWorld {
    /// Publish the inherited opacity tween's updated value to the renderer.
    fn opacity_updated(&mut self, dt_ms: f32, opacity: f32);
    fn advance_model(&mut self, index: usize, dt_ms: f32) -> u32;
    fn model_frame(&mut self, index: usize) -> i32;
    fn model_frame_count(&mut self, index: usize) -> i32;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Runtime {
    pub traffic: TrafficRuntime,
    pub initial_parameter: f32,
    pub pending_push: Option<Vec3>,
    pub collision_ms: f32,
    pub reaction_kind: u32,
    pub reaction_playing: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pedestrian {
    pub id: u64,
    pub record: PedestrianRecord,
    pub runtime: Runtime,
}

impl Pedestrian {
    pub fn construct(id: u64, tuning: Tuning, random: &mut impl VehicleRandom) -> Self {
        let mut owner = Self { id, ..Self::default() };
        let base = owner.base_mut();
        base.occupant.flags |= 0x10;
        let speed = random.signed_range(tuning.speed.0, tuning.speed.1) as f32 * SPEED_UNIT;
        base.f1[0] = speed;
        base.f3[..4].copy_from_slice(&[speed, speed, 0.0, 0.0]);
        owner
    }

    pub fn base(&self) -> &AutomatonBase {
        &self.record.traffic.base
    }
    pub fn base_mut(&mut self) -> &mut AutomatonBase {
        &mut self.record.traffic.base
    }

    pub fn traffic_owner(&mut self) -> TrafficOwner<'_> {
        TrafficOwner {
            record: &mut self.record.traffic,
            runtime: &mut self.runtime.traffic,
        }
    }

    /// The named parameter remains unnamed in the spec; no guessed semantic alias.
    pub fn init(
        &mut self,
        tuning: Tuning,
        random: &mut impl VehicleRandom,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        if self.runtime.traffic.initialized {
            return Ok(true);
        }
        self.runtime.initial_parameter = random.float_range(
            f64::from(tuning.initial_parameter.0),
            f64::from(tuning.initial_parameter.1),
        ) as f32;
        let success = world.create_models();
        if success {
            world.init_effects();
        }
        world.init_occupant();
        self.runtime.traffic.initialized = true;
        if success && self.base().occupant.status & 2 == 0 {
            if self.base().automata_flags & 0x4000 == 0 {
                InitWorld::refresh_behavior(world);
            }
            if self.base().automata_flags & 0x2000 == 0 {
                self.repath(world)?;
            }
        }
        Ok(success)
    }

    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        let Some(record) = PedestrianRecord::read(input) else {
            return false;
        };
        if record.version != crate::automata::PEDESTRIAN_VERSION || input.error() != 0 {
            return false;
        }
        self.record = record;
        self.runtime = Runtime::default();
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        self.record.write(output)
    }

    pub fn shutdown(&mut self, world: &mut impl PedestrianWorld) {
        self.traffic_owner().shutdown(world);
    }

    fn move_to(&mut self, position: Vec3, orientation: Vec3, world: &mut impl PedestrianWorld) {
        self.base_mut().vectors[0] = position;
        self.base_mut().vectors[2] = orientation;
        world.move_models(position, orientation);
    }

    pub fn test_collision_expensive(&mut self, position: Vec3, dt_ms: f32, world: &mut impl PedestrianWorld) -> bool {
        self.runtime.collision_ms += dt_ms;
        if self.runtime.collision_ms < 500.0 {
            return false;
        }
        // Literal accumulated-time cadence: one query, no catch-up burst.
        self.runtime.collision_ms = 0.0;
        let bounds = Bounds {
            min: add(position, [-1.0, 0.0, -1.0]),
            max: add(position, [1.0, 2.0, 1.0]),
        };
        let mut collided = false;
        for other in world.nearby_automata(crate::automata::AUTOMATON_OCCUPANT_TYPE, bounds) {
            if other.id == self.id {
                continue;
            }
            let delta = sub(other.position, position);
            let distance = length(delta);
            if distance < 1.0 {
                let push = if distance < 0.01 {
                    [
                        world.automata_range(-0.5, 0.5) as f32,
                        0.0,
                        world.automata_range(-0.5, 0.5) as f32,
                    ]
                } else {
                    delta
                };
                world.pending_push(other.id, push);
                collided = true;
            }
        }
        collided
    }

    pub fn move_forward_with_collision(
        &mut self,
        direction: Vec3,
        dt_ms: f32,
        modifier: f32,
        world: &mut impl PedestrianWorld,
    ) -> bool {
        let facing = unit(direction);
        let step = modifier * tween_value(&self.base().f3[..4]) * dt_ms;
        let mut next = add(self.base().vectors[0], scale(facing, step));
        if self.base().automata_flags & 0x20000 == 0 {
            next = world.ground_position(next, 0xC49);
        }
        let mut collided = self.test_collision_expensive(next, dt_ms, world);
        let push = self.runtime.pending_push.take();
        if collided {
            if let Some(push) = push.filter(|v| length(*v) != 0.0) {
                next = world.ground_position(add(next, push), 0xC49);
                collided = self.test_collision_expensive(next, 0.0, world);
            }
        }
        if collided {
            return false;
        }
        self.move_to(next, facing, world);
        true
    }

    pub fn crowd_reaction(
        &mut self,
        ignored_flag: bool,
        event_mode: u32,
        event_position: Vec3,
        modifier: f32,
        world: &mut impl PedestrianWorld,
    ) -> bool {
        if ignored_flag {
            return false;
        }
        self.runtime.reaction_kind = if event_mode != 0 { 1 } else { 2 };
        return_to_default_speed(self.base_mut(), modifier);
        let position = self.base().vectors[0];
        self.move_to(position, unit(sub(event_position, position)), world);
        true
    }

    /// Wrap the shared traffic dispatcher without duplicating its base-state code.
    fn inherited_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        let mut traffic = TrafficAutomaton {
            record: std::mem::take(&mut self.record.traffic),
            runtime: std::mem::take(&mut self.runtime.traffic),
        };
        let result = traffic.state_tick(dt_ms, movement, world);
        self.record.traffic = traffic.record;
        self.runtime.traffic = traffic.runtime;
        result
    }

    pub fn animation_tick(
        &mut self,
        dt_ms: f32,
        movement: Movement,
        moves: bool,
        held: bool,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<()> {
        let stopped_before = self.base().automata_flags & 0x20 != 0;
        advance_tweens(self.base_mut(), dt_ms);
        world.effects(EffectAction::Update);
        if moves && !self.state_tick(dt_ms, movement, world)? {
            if self.base().automata_flags & 4 == 0 {
                if !matches!(self.record.traffic.network, RefValue::Null)
                    && self.runtime.traffic.has_tile_position
                    && self.runtime.traffic.has_path
                    && self.base().automata_flags & 0x2000 == 0
                {
                    self.traffic_owner().path_tick(dt_ms, movement, false, world)?;
                } else {
                    self.traffic_owner().straight_tick(dt_ms, movement.speed_scale, world);
                }
            } else if !held && self.base().automata_flags & 1 == 0 {
                self.traffic_owner().remove(world);
            }
        }
        world.event(0, 0);
        if moves {
            if stopped_before {
                world.play_animation(0);
            } else if self.base().automata_flags & 0x20 != 0 {
                world.stopped_animation();
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PedestrianOneShot {
    pub pedestrian: Pedestrian,
    pub play_count: u8,
}

impl Default for PedestrianOneShot {
    fn default() -> Self {
        Self {
            pedestrian: Pedestrian::default(),
            play_count: 5,
        }
    }
}

impl PedestrianOneShot {
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        if !self.pedestrian.read(input) {
            return false;
        }
        self.play_count = 0;
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        self.pedestrian.write(output)
    }

    pub fn construct(id: u64, tuning: Tuning, random: &mut impl VehicleRandom) -> Self {
        Self {
            pedestrian: Pedestrian::construct(id, tuning, random),
            play_count: 5,
        }
    }

    pub fn init(
        &mut self,
        tuning: Tuning,
        random: &mut impl VehicleRandom,
        world: &mut impl PedestrianWorld,
    ) -> crate::vehicles::Result<bool> {
        let success = self.pedestrian.init(tuning, random, world)?;
        if success {
            for index in 0..world.model_count() {
                if world.model_present(index) {
                    // PN1: SetSpeed only; preserve frame, accumulator and playback mode.
                    world.instance_value(index, 0.01);
                }
            }
            let values = world.exemplar_play_count(0xEA95_8845);
            let bounds = if values.len() >= 2 {
                (values[0], values[1])
            } else {
                tuning.play_count
            };
            let span = u32::from(bounds.1).wrapping_sub(u32::from(bounds.0)).wrapping_add(1);
            self.play_count = u32::from(bounds.0).wrapping_add(random.uniform(span)) as u8;
        }
        Ok(success)
    }

    /// Slots 177/178 are byte accessors without any playback or lifetime effects.
    pub fn play_count(&self) -> u8 {
        self.play_count
    }

    pub fn set_play_count(&mut self, count: u8) {
        self.play_count = count;
    }

    /// Charge one sampled final frame, rather than one completed animation loop.
    pub fn consume_play_count(&mut self, world: &mut impl PedestrianWorld) -> bool {
        self.play_count = self.play_count.saturating_sub(1);
        if self.play_count == 0 {
            // The shared removal path marks the automaton: ordinary Remove(false).
            self.pedestrian.traffic_owner().remove(world);
            false
        } else {
            true
        }
    }

    /// PN1 slot 72: opacity first, then every live model, with one final-frame sample.
    pub fn animation_tick(&mut self, _dt_us: u32, dt_ms: f32, world: &mut impl OneShotWorld) -> bool {
        // Only opacity is advanced here, not the speed tween or animation/state clock.
        let opacity = &mut self.pedestrian.base_mut().f3[4..8];
        opacity[3] += dt_ms / 1000.0;
        if opacity[3] >= opacity[2] {
            opacity[0] = opacity[1];
        }
        world.opacity_updated(dt_ms, tween_value(opacity));

        let mut first = true;
        let mut keep = true;
        for index in 0..world.model_count() {
            if !world.model_present(index) {
                continue;
            }
            let _ = world.advance_model(index, dt_ms);
            if first {
                first = false;
                let frame = world.model_frame(index);
                let final_frame = world.model_frame_count(index).wrapping_sub(1);
                if frame >= final_frame {
                    keep = self.consume_play_count(world);
                }
            }
        }
        keep
    }
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn length(a: Vec3) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn unit(a: Vec3) -> Vec3 {
    let n = length(a);
    if n == 0.0 {
        [0.0; 3]
    } else {
        scale(a, 1.0 / n)
    }
}
