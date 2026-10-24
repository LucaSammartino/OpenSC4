// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::stream::{InStream, OutStream, RefValue};

pub const CLSID_TRAIN_CAR: u32 = 0x4A41_6254;
pub const CLSID_TRAIN: u32 = 0x8A41_625A;
pub const TRAIN_TRIP_MESSAGE: u32 = 0x0C05_E4CD;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrainCar {
    pub vehicle: Vehicle,
    pub classification: bool,
    pub rear_point: crate::automata::Vec3,
}

impl TrainCar {
    pub fn init(&mut self, tuning: Initialization, world: &mut impl InitWorld) -> bool {
        if self.vehicle.traffic.initialized {
            return true;
        }
        let success = self.vehicle.init(tuning, world);
        self.vehicle.record.traffic.base.automata_flags |= 0x10014000;
        self.classification = world.in_family(0x4217) || world.in_family(0x4208);
        success
    }
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        input.get_u16() == Some(2) && self.vehicle.read(input)
    }
    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        output.put_u16(2) && self.vehicle.write(output)
    }
}

#[derive(Clone, Debug)]
pub struct Train {
    pub vehicle: Vehicle,
    /// Owned cars, never borrowed game objects. Dropping/shutdown releases each.
    pub cars: Vec<TrainCar>,
    /// Serializable identities, in stored order. References use the stream's
    /// shared reference protocol; they are never interpreted as pointer bytes.
    pub car_references: Vec<RefValue>,
    pub classification: bool,
    pub rear_point: crate::automata::Vec3,
    pub target_count: u32,
    pub car_family: u32,
    pub cached_cell: [i32; 3],
    pub raw_2c0: u32,
    pub vector_2d4: crate::automata::Vec3,
    pub network: RefValue,
    pub raw_2e4: u32,
    pub blocked_ms: f32,
    pub flag_2f8: bool,
}

impl Default for Train {
    fn default() -> Self {
        Self {
            vehicle: Vehicle::default(),
            cars: Vec::new(),
            car_references: Vec::new(),
            classification: false,
            rear_point: UNSET_POINT,
            target_count: 0,
            car_family: 0,
            cached_cell: [-1; 3],
            raw_2c0: 0,
            vector_2d4: [0.0; 3],
            network: RefValue::Null,
            raw_2e4: 0,
            blocked_ms: 0.0,
            flag_2f8: false,
        }
    }
}

impl Train {
    pub fn construct(tuning: Construction, random: &mut impl VehicleRandom) -> Self {
        let car = TrainCar::construct(tuning, random);
        Self {
            vehicle: car.vehicle,
            classification: car.classification,
            rear_point: car.rear_point,
            cached_cell: [-1; 3],
            ..Self::default()
        }
    }
    pub fn init(
        &mut self,
        tuning: Initialization,
        random: &mut impl VehicleRandom,
        world: &mut impl InitWorld,
    ) -> bool {
        if self.vehicle.traffic.initialized {
            return true;
        }
        let success = self.vehicle.init(tuning, world);
        self.vehicle.record.traffic.base.automata_flags |= 0x14000;
        self.vehicle.record.traffic.base.automata_flags &= !0x10000000;
        self.classification = world.in_family(0x4217) || world.in_family(0x4208);
        let speed = random.float_range(
            (tuning.train_speed.0 as f32 * SPEED_UNIT) as f64,
            (tuning.train_speed.1 as f32 * SPEED_UNIT) as f64,
        ) as f32;
        super::init::immediate_speed(&mut self.vehicle.record.traffic.base, speed);
        for (engine, family) in [
            (0x4216, 0x4203),
            (0x4215, 0x4201),
            (0x4217, 0x4208),
            (0x422C, 0x422D),
            (0x422E, 0x422F),
        ] {
            if world.in_family(engine) {
                self.car_family = family;
                break;
            }
        }
        if self.car_family != 0 {
            let (lower, upper) = if self.car_family == 0x4203 {
                tuning.freight_count
            } else {
                tuning.other_count
            };
            self.target_count = u32::from(lower) + random.uniform(u32::from(upper.wrapping_sub(lower)));
        }
        self.blocked_ms = 0.0;
        self.flag_2f8 = false;
        success
    }
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        if input.get_u16() != Some(2) || input.get_u16() != Some(2) || !self.vehicle.read(input) {
            return false;
        }
        let Some(count) = input.get_u32() else {
            return false;
        };
        for _ in 0..count {
            let Some(car) = input.get_ref_as(CLSID_TRAIN_CAR) else {
                return false;
            };
            if !car.is_null() {
                self.car_references.push(car);
            }
        }
        let Some(raw) = input.get_u32() else {
            return false;
        };
        self.raw_2c0 = raw;
        for value in &mut self.vector_2d4 {
            let Some(v) = input.get_f32() else {
                return false;
            };
            *value = v;
        }
        self.network = RefValue::Null;
        let Some(network) = input.get_ref_as(0xA821_EF94) else {
            return false;
        };
        self.network = network;
        let Some(raw) = input.get_u32() else {
            return false;
        };
        self.raw_2e4 = raw;
        input.error() == 0
    }
    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        if !output.put_u16(2) || !output.put_u16(2) || !self.vehicle.write(output) {
            return false;
        }
        output.put_u32(self.car_references.len() as u32);
        for car in &self.car_references {
            output.put_ref(car);
        }
        output.put_u32(self.raw_2c0);
        for v in self.vector_2d4 {
            output.put_f32(v);
        }
        output.put_ref(&self.network);
        output.put_u32(self.raw_2e4) && output.error() == 0
    }
    pub fn start_trip(&mut self) -> Result<()> {
        Err(Unsupported::TrainTripVh5)
    }
    pub fn animation_tick(&mut self) -> Result<()> {
        Err(Unsupported::TrainFollowingVh5)
    }
    pub fn player_drive(&mut self) -> Result<()> {
        Err(Unsupported::PlayerDrivingVh1)
    }

    pub fn shutdown(&mut self, world: &mut impl VehicleWorld) {
        for car in &mut self.cars {
            car.vehicle.shutdown(world);
        }
        self.cars.clear();
        self.car_references.clear();
        self.network = RefValue::Null;
        self.vehicle.shutdown(world);
    }

    /// managers §5: train collision timeout is at least 10,000 ms.
    pub fn collide_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        if let Some(state) = self.vehicle.record.traffic.base.states.last_mut() {
            if state.state_type == 20 && state.base.countdown_ms < 10000.0 {
                state.base.countdown_ms = 10000.0;
            }
        }
        let _ = self.vehicle.state_tick(
            dt_ms,
            Movement {
                speed_scale: 1.0,
                max_speed: 0.0,
                leader_speed: None,
            },
            world,
        );
    }
}
