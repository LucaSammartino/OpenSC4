// SPDX-License-Identifier: GPL-3.0-or-later
//! Concrete record wrappers from automata.md §§6.4.1–6.4.4. No Init or movement runs here.

use super::codec::{get_vec3, put_vec3};
use super::{same_ref, EmergencyRecord, Vec3, VehicleRecord, IID_AUTOMATON, IID_NETWORK_OCCUPANT};
use crate::stream::{InStream, OutStream, RefValue};

pub const CLSID_CARJACKED: u32 = 0xCBB3_2B0A;
pub const CLSID_LIMO: u32 = 0x4C0D_D224;
pub const CLSID_AMBULANCE: u32 = 0x6C1A_CDE1;
pub const CLSID_FIRE_TRUCK: u32 = 0xAA22_2B83;
pub const CLSID_TANK: u32 = 0xEBE0_DCAD;
pub const CLSID_MASS_TRANSIT_TRAIN: u32 = 0x4C22_AE53;
const IID_TRAIN_CAR: u32 = 0x4A41_6254;

/// Explicit wrapper ownership, in addition to the stream helper's acquired reference.
/// Core Object references already own their Rc; callbacks handle only host references.
pub trait DerivedRecordRefs {
    fn add_ref(&self, reference: &RefValue);
    fn release(&self, reference: &RefValue);
}

impl DerivedRecordRefs for () {
    fn add_ref(&self, _: &RefValue) {}
    fn release(&self, _: &RefValue) {}
}

/// The shared Carjacked/Limo payload. Class identity belongs to the object-store framing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VehicleWrapperRecord {
    pub vehicle: VehicleRecord,
}

impl VehicleWrapperRecord {
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        if input.get_u16() != Some(1) {
            return false;
        }
        let Some(vehicle) = VehicleRecord::read(input) else {
            return false;
        };
        self.vehicle = vehicle;
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        output.put_u16(1);
        self.vehicle.write(output)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AmbulanceRecord {
    pub emergency: EmergencyRecord,
}

impl AmbulanceRecord {
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        input.get_u16() == Some(1) && self.emergency.read_into(input)
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        output.put_u16(1);
        self.emergency.write(output)
    }
}

#[derive(Clone, Debug)]
pub struct FireTruckRecord {
    pub emergency: EmergencyRecord,
    pub crew: RefValue,
}

impl Default for FireTruckRecord {
    fn default() -> Self {
        Self {
            emergency: EmergencyRecord::default(),
            crew: RefValue::Null,
        }
    }
}

impl FireTruckRecord {
    pub fn read(&mut self, input: &mut dyn InStream, refs: &impl DerivedRecordRefs) -> bool {
        if input.get_u16() != Some(2) || !self.emergency.read_into(input) {
            return false;
        }
        let crew = input.get_ref_as(IID_AUTOMATON).unwrap_or(RefValue::Null);
        if !same_ref(&self.crew, &crew) {
            if !crew.is_null() {
                refs.add_ref(&crew);
            }
            let old = std::mem::replace(&mut self.crew, crew);
            if !old.is_null() {
                refs.release(&old);
            }
        }
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        output.put_u16(2);
        if !self.emergency.write(output) {
            return false;
        }
        output.put_ref(&self.crew);
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TankRecord {
    pub vehicle: VehicleRecord,
    pub saved_indices: [i8; 2],
}

impl Default for TankRecord {
    fn default() -> Self {
        Self {
            vehicle: VehicleRecord::default(),
            saved_indices: [-1; 2],
        }
    }
}

impl TankRecord {
    pub fn read(&mut self, input: &mut dyn InStream) -> bool {
        let Some(version) = input.get_u16() else { return false };
        if version > 2 {
            return false;
        }
        if let Some(vehicle) = VehicleRecord::read(input) {
            self.vehicle = vehicle;
            if version == 2 {
                for index in &mut self.saved_indices {
                    if let Some(value) = input.get_i8() {
                        *index = value;
                    }
                }
            }
        }
        input.error() == 0
    }

    /// Re-encode saved indices without constructing live models (record-only replay).
    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        self.write_indices(output, self.saved_indices)
    }

    /// Live Write computes indices from opaque model identities: last match, low byte.
    pub fn write_with_models(&self, output: &mut dyn OutStream, models: &[u64], handles: [u64; 2]) -> bool {
        let mut indices = [-1; 2];
        for (position, model) in models.iter().enumerate() {
            for (index, handle) in indices.iter_mut().zip(handles) {
                if *model == handle {
                    *index = position as i8;
                }
            }
        }
        self.write_indices(output, indices)
    }

    fn write_indices(&self, output: &mut dyn OutStream, indices: [i8; 2]) -> bool {
        output.put_u16(2);
        if !self.vehicle.write(output) {
            return false;
        }
        for index in indices {
            output.put_i8(index);
        }
        true
    }
}

/// MassTransitTrain → Train v2 → TrainCar v2 → Vehicle, then the Train tail.
/// This codec has no runtime cars, model initialization or movement state.
#[derive(Clone, Debug)]
pub struct MassTransitTrainRecord {
    pub vehicle: VehicleRecord,
    pub cars: Vec<RefValue>,
    pub raw_2c0: u32,
    pub vector_2d4: Vec3,
    pub network: RefValue,
    pub raw_2e4: u32,
}

impl Default for MassTransitTrainRecord {
    fn default() -> Self {
        Self {
            vehicle: VehicleRecord::default(),
            cars: Vec::new(),
            raw_2c0: 0,
            vector_2d4: [0.0; 3],
            network: RefValue::Null,
            raw_2e4: 0,
        }
    }
}

impl MassTransitTrainRecord {
    pub fn read(&mut self, input: &mut dyn InStream, refs: &impl DerivedRecordRefs) -> bool {
        if input.get_u16() != Some(1) || input.get_u16() != Some(2) || input.get_u16() != Some(2) {
            return false;
        }
        let Some(vehicle) = VehicleRecord::read(input) else {
            return false;
        };
        self.vehicle = vehicle;
        let count = input.get_u32().unwrap_or(0);
        for _ in 0..count {
            if let Some(car) = input.get_ref_as(IID_TRAIN_CAR).filter(|car| !car.is_null()) {
                self.cars.push(car);
            }
        }
        if let Some(raw) = input.get_u32() {
            self.raw_2c0 = raw;
        }
        if let Some(vector) = get_vec3(input) {
            self.vector_2d4 = vector;
        }
        let old = std::mem::replace(&mut self.network, RefValue::Null);
        if !old.is_null() {
            refs.release(&old);
        }
        self.network = input.get_ref_as(IID_NETWORK_OCCUPANT).unwrap_or(RefValue::Null);
        if let Some(raw) = input.get_u32() {
            self.raw_2e4 = raw;
        }
        true
    }

    pub fn write(&self, output: &mut dyn OutStream) -> bool {
        output.put_u16(1);
        output.put_u16(2);
        output.put_u16(2);
        if !self.vehicle.write(output) {
            return false;
        }
        output.put_u32(self.cars.len() as u32);
        for car in &self.cars {
            output.put_ref(car);
        }
        output.put_u32(self.raw_2c0);
        put_vec3(output, &self.vector_2d4);
        output.put_ref(&self.network);
        output.put_u32(self.raw_2e4);
        true
    }
}
