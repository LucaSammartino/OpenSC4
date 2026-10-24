// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{SetStateArgs, Vec3};
use crate::stream::ResKey;

/// A fresh local random primitive, never the shared automata RNG. Seed is
/// requested once per collision as specified; range results may be controlled.
pub trait CollisionRandom {
    fn seed(&mut self, seed: u32);
    fn range(&mut self, lower: f64, upper: f64) -> f64;
}

pub fn random_direction(random: &mut impl CollisionRandom) -> Vec3 {
    loop {
        let x = random.range(-1.0, 1.0) as f32;
        let y = random.range(-1.0, 1.0) as f32;
        let z_resident = random.range(-1.0, 1.0);
        let z = z_resident as f32;
        let sum = (z_resident * f64::from(z) + f64::from(y) * f64::from(y)) + f64::from(x) * f64::from(x);
        if sum > 1.0 || sum < 0.0001 { continue; }
        let norm = ((f64::from(z) * f64::from(z) + f64::from(x) * f64::from(x)) + f64::from(y) * f64::from(y)).sqrt();
        let inv = 1.0 / norm;
        let stored = inv as f32;
        return [(f64::from(x) * inv) as f32, y * stored, z * stored];
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DriveDamage {
    pub damage: u32,
    pub acceleration: f32,
    pub raw_speed: f32,
    pub accident: Option<u64>,
    pub effect: Option<u64>,
}

/// Renderer and manager interface operations; all threshold, ownership and
/// ring decisions are made by the owner below.
pub trait DamageWorld: VehicleWorld {
    fn accident(&mut self, position: Vec3, kind: u32, duration_ms: u32) -> Option<u64>;
    fn request_effect(&mut self, name: &'static str) -> Option<u64>;
    fn stop_effect(&mut self, effect: u64, argument: u32);
    fn release_effect(&mut self, effect: u64);
    fn transform_effect(&mut self, effect: u64, position: Vec3, facing: Vec3, scale: Vec3);
    fn start_effect(&mut self, effect: u64, argument: u32);
    fn blur_model(&mut self, key: ResKey) -> Option<u64>;
    fn place_blur(&mut self, model: u64, position: Vec3, facing: Vec3, newly_created: bool);
    fn blur_alpha(&mut self, model: u64, alpha: f32);
    fn release_blur(&mut self, model: u64);
}

impl Vehicle {
    pub fn handle_collision(&mut self, other_present: bool, random: &mut impl CollisionRandom,
        world: &mut impl VehicleWorld) -> Result<bool> {
        if !other_present { return Ok(true); }
        world.post_message(COLLISION_MESSAGE);
        if self.record.traffic.base.states.last().map(|s| s.state_type) == Some(20) { return Ok(true); }
        random.seed(u32::MAX);
        let r = random_direction(random);
        self.record.traffic.base.vectors[2] = unit(add(self.record.traffic.base.vectors[2], scale(r, 0.5)));
        self.record.traffic.base.automata_flags |= 0x10000000;
        self.set_state(20, SetStateArgs { timeout_ms: 5000.0, final_flag: 1, ..SetStateArgs::default() }, world)
    }

    pub fn update_after_damage(&mut self, state: &mut DriveDamage, health: u32, world: &mut impl DamageWorld) {
        let name = if state.damage >= health {
            state.accident = world.accident(self.record.traffic.base.vectors[0], 2, 30000);
            self.record.traffic.base.automata_flags |= 0x08000000;
            world.event(11, 1);
            "udriveit_crash"
        } else if state.damage >= health / 2 { "udriveit_injured" } else { return; };
        let Some(effect) = world.request_effect(name) else { return; };
        if let Some(old) = state.effect.replace(effect) {
            world.stop_effect(old, 0);
            world.release_effect(old);
        }
        world.transform_effect(effect, self.record.traffic.base.vectors[0], self.record.traffic.base.vectors[2], [1.0; 3]);
        world.start_effect(effect, 0);
    }

    pub fn resolve_collision(&mut self, vector: Vec3, impulse: f32, divisor: f32, health: u32,
        state: &mut DriveDamage, world: &mut impl DamageWorld) {
        let q = impulse / divisor;
        let magnitude = length(vector);
        if magnitude > 0.01 || q > 2.0 {
            let mut increment = (magnitude - 0.01) * 2000.0;
            if q >= 5.0 { increment *= q; }
            // Literal VH6: signed increments are allowed; truncate and retain
            // low 32 bits rather than Rust's saturating float-to-u32 cast.
            let total = f64::from(state.damage) + f64::from(increment);
            state.damage = total.trunc().rem_euclid(4294967296.0) as u32;
        }
        self.update_after_damage(state, health, world);
        let current = tween_value(&self.record.traffic.base.f3[..4]);
        let facing = self.record.traffic.base.vectors[2];
        let dot = vector.iter().zip(facing).map(|(a,b)| a*b).sum::<f32>();
        let speed = (current + dot * q).max(0.0).min(current);
        self.record.traffic.base.f3[..4].copy_from_slice(&[speed, speed, 0.0, 0.0]);
        state.raw_speed = speed;
        if speed == 0.0 { state.acceleration = 0.0; }
    }

    pub fn update_motion_blur(&mut self, world: &mut impl DamageWorld) {
        let index = self.blur_index;
        let new = self.blur_models[index].is_none();
        if new { self.blur_models[index] = world.blur_model(self.record.traffic.base.model); }
        if let Some(model) = self.blur_models[index] {
            world.place_blur(model, self.record.traffic.base.vectors[0], self.record.traffic.base.vectors[2], new);
        }
        let mut alpha = tween_value(&self.record.traffic.base.f3[4..]).min(1.0);
        for back in 0..4 {
            if let Some(model) = self.blur_models[(index + 4 - back) % 4] { world.blur_alpha(model, alpha); }
            alpha = (alpha - 0.33333334).max(0.0);
        }
        self.blur_index = (index + 1) % 4;
    }

    pub fn release_motion_blur(&mut self, world: &mut impl DamageWorld) {
        for model in &mut self.blur_models {
            if let Some(model) = model.take() { world.release_blur(model); }
        }
    }
}

impl Train {
    pub fn handle_collision(&mut self, random: &mut impl CollisionRandom, world: &mut impl VehicleWorld) -> Result<bool> {
        let args = SetStateArgs { timeout_ms: 20000.0, final_flag: 1, ..SetStateArgs::default() };
        let mut success = true;
        if self.vehicle.record.traffic.base.states.last().map(|s| s.state_type) != Some(20) {
            success = self.vehicle.set_state(20, args, world)?;
        }
        random.seed(u32::MAX);
        for car in &mut self.cars {
            car.vehicle.measured_speed = self.vehicle.measured_speed;
            let r = random_direction(random);
            let mut perturbation = scale(r, 0.25 / length(r));
            perturbation[1] = 0.0;
            let facing = &mut car.vehicle.record.traffic.base.vectors[2];
            *facing = unit(add(*facing, perturbation));
            if car.vehicle.record.traffic.base.states.last().map(|s| s.state_type) == Some(20) { return Ok(true); }
            success &= car.vehicle.set_state(20, args, world)?;
        }
        self.vehicle.record.traffic.base.f3[..4].copy_from_slice(&[0.0; 4]);
        Ok(success)
    }
}
