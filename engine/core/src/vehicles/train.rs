// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const CLSID_TRAIN_CAR: u32 = 0x4A41_6254;
pub const CLSID_TRAIN: u32 = 0x8A41_625A;
pub const TRAIN_TRIP_MESSAGE: u32 = 0x0C05_E4CD;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrainCar {
    pub vehicle: Vehicle,
}

impl TrainCar {
    pub fn init(&mut self) -> Result<()> {
        // vehicles §5 gives this flag, but not the inherited Init flags (VH4).
        self.vehicle.record.traffic.base.automata_flags |= 0x10000000;
        Err(Unsupported::InitializationVh4)
    }
    pub fn read(&mut self, _input: &mut dyn crate::stream::InStream) -> Result<()> {
        Err(Unsupported::TrainRecordVh3)
    }
    pub fn write(&self, _output: &mut dyn crate::stream::OutStream) -> Result<()> {
        Err(Unsupported::TrainRecordVh3)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Train {
    pub vehicle: Vehicle,
    /// Owned cars, never borrowed game objects. Dropping/shutdown releases each.
    pub cars: Vec<TrainCar>,
}

impl Train {
    pub fn init(&mut self) -> Result<()> {
        Err(Unsupported::TrainCompositionVh5)
    }
    pub fn read(&mut self, _input: &mut dyn crate::stream::InStream) -> Result<()> {
        Err(Unsupported::TrainRecordVh3)
    }
    pub fn write(&self, _output: &mut dyn crate::stream::OutStream) -> Result<()> {
        Err(Unsupported::TrainRecordVh3)
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
        self.vehicle.shutdown(world);
    }

    /// managers §5: train collision timeout is at least 10,000 ms.
    pub fn collide_tick(&mut self, dt_ms: f32, world: &mut impl VehicleWorld) {
        if let Some(state) = self.vehicle.record.traffic.base.states.last_mut() {
            if state.state_type == 20 && state.base.countdown_ms < 10000.0 {
                state.base.countdown_ms = 10000.0;
            }
        }
        self.vehicle.collide_tick(dt_ms, world);
    }

    /// automata §6.5: copy measured speed before spinning coupled vehicles.
    /// The supplied directions have already been resolved; distribution VH6.
    pub fn enter_collision(
        &mut self,
        directions: &[crate::automata::Vec3],
        world: &mut impl VehicleWorld,
    ) -> Result<()> {
        if directions.len() != self.cars.len() + 1 {
            return Err(Unsupported::CollisionDirectionVh6);
        }
        self.vehicle.enter_collision(directions[0], world)?;
        for (car, direction) in self.cars.iter_mut().zip(&directions[1..]) {
            car.vehicle.measured_speed = self.vehicle.measured_speed;
            car.vehicle.enter_collision(*direction, world)?;
        }
        Ok(())
    }
}
