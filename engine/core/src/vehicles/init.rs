// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{AutomatonBase, TrafficRecord, Vec3};

pub const SPEED_UNIT: f32 = f32::from_bits(0x3991_A2B5);
pub const UNSET_POINT: Vec3 = [-1.0, -2.0, -3.0];

/// Runtime globals and random primitives are explicit inputs, not guessed defaults.
pub trait VehicleRandom {
    fn signed_range(&mut self, lower: i32, upper: i32) -> i32;
    fn float_range(&mut self, lower: f64, upper: f64) -> f64;
    fn uniform(&mut self, bound: u32) -> u32;
}

#[derive(Clone, Copy, Debug)]
pub struct Construction {
    pub vehicle_speed: (i32, i32),
    pub train_speed: (i32, i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Initialization {
    pub collision: bool,
    pub city_lights: bool,
    pub force_lights: bool,
    pub train_speed: (u32, u32),
    pub freight_count: (u8, u8),
    pub other_count: (u8, u8),
}

/// Only inherited model/occupant operations and foreign tuning/services.
/// No callback implements a traffic or vehicle owner.
pub trait InitWorld {
    fn create_models(&mut self) -> bool;
    fn init_effects(&mut self);
    fn init_occupant(&mut self);
    fn refresh_behavior(&mut self);
    fn initialize_route(&mut self);
    fn load_drive_parameters(&mut self);
    fn enable_lights(&mut self, enabled: bool, mode: u32);
    fn in_family(&mut self, family: u32) -> bool;
}

pub(crate) fn construct_traffic(record: &mut TrafficRecord) {
    *record = TrafficRecord::default();
    record.base.occupant.flags |= 0x10;
}

pub(crate) fn immediate_speed(base: &mut AutomatonBase, speed: f32) {
    base.f1[0] = speed;
    base.f3[..4].copy_from_slice(&[speed, speed, 0.0, 0.0]);
}

pub(crate) fn init_traffic(record: &mut TrafficRecord, runtime: &mut TrafficRuntime, world: &mut impl InitWorld) -> bool {
    if runtime.initialized {
        return true;
    }
    let success = world.create_models();
    if success {
        world.init_effects();
    }
    world.init_occupant();
    runtime.initialized = true;
    if success && record.base.occupant.status & 2 == 0 {
        if record.base.automata_flags & 0x4000 == 0 {
            world.refresh_behavior();
        }
        if record.base.automata_flags & STRAIGHT == 0 {
            world.initialize_route();
        }
    }
    success
}

impl TrafficAutomaton {
    pub fn construct() -> Self {
        let mut result = Self::default();
        construct_traffic(&mut result.record);
        result.runtime.path_word = 0x0000_FFFF;
        result.runtime.travel_type = 7;
        result
    }
}

impl Vehicle {
    pub fn construct(tuning: Construction, random: &mut impl VehicleRandom) -> Self {
        let mut result = Self::default();
        construct_traffic(&mut result.record.traffic);
        result.traffic.path_word = 0x0000_FFFF;
        result.traffic.travel_type = 7;
        result.record.v = UNSET_POINT;
        result.record.constant = UNSET_POINT;
        result.runtime_vectors = [UNSET_POINT, UNSET_POINT, [0.0; 3]];
        result.sound_identity = 0xC7CC_75F1;
        let speed = random.signed_range(tuning.vehicle_speed.0, tuning.vehicle_speed.1) as f32 * SPEED_UNIT;
        immediate_speed(&mut result.record.traffic.base, speed);
        result
    }
}

impl TrainCar {
    pub fn construct(tuning: Construction, random: &mut impl VehicleRandom) -> Self {
        let mut result = Self {
            vehicle: Vehicle::construct(tuning, random),
            classification: false,
            rear_point: UNSET_POINT,
        };
        let speed = random.signed_range(tuning.train_speed.0, tuning.train_speed.1) as f32 * SPEED_UNIT;
        immediate_speed(&mut result.vehicle.record.traffic.base, speed);
        result
    }
}
