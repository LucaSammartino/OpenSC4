// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Types and pure helpers shared by the growth subsystems:
//! the building development simulator (`docs/specs/building-development.md`,
//! "BDS"), the growth developers (`growth-developer.md`) and the tract
//! developer (`tract-developer.md`).
//!
//! Section references are to `building-development.md` unless another spec
//! is named. Everything here is plain data; the modules that use it reach
//! one another only through their host traits.

/// Developer types (BDS §1.5): the growth developer's map key, property
/// `0x21` of its exemplar.
pub mod dev_type {
    pub const R1: u32 = 0x1010;
    pub const R2: u32 = 0x1020;
    pub const R3: u32 = 0x1030;
    pub const CS1: u32 = 0x3110;
    pub const CS2: u32 = 0x3120;
    pub const CS3: u32 = 0x3130;
    pub const CO2: u32 = 0x3320;
    pub const CO3: u32 = 0x3330;
    pub const I_AGRICULTURE: u32 = 0x4100;
    pub const I_DIRTY: u32 = 0x4200;
    pub const I_MANUFACTURING: u32 = 0x4300;
    pub const I_HIGH_TECH: u32 = 0x4400;
    /// The twelve shipped developer types in map (ascending) order.
    pub const ALL: [u32; 12] = [
        R1,
        R2,
        R3,
        CS1,
        CS2,
        CS3,
        CO2,
        CO3,
        I_AGRICULTURE,
        I_DIRTY,
        I_MANUFACTURING,
        I_HIGH_TECH,
    ];
}

/// `cISC4BuildingOccupant::PurposeType` values.
pub mod purpose {
    pub const NONE: u32 = 0;
    pub const RESIDENCE: u32 = 1;
    pub const SERVICES: u32 = 2;
    pub const OFFICE: u32 = 3;
    pub const TOURISM: u32 = 4;
    pub const AGRICULTURE: u32 = 5;
    pub const PROCESSING: u32 = 6;
    pub const MANUFACTURING: u32 = 7;
    pub const HIGH_TECH: u32 = 8;
}

/// `GetDeveloperType(p, w)` (§4.7.3, slot 30).
pub fn developer_type(purpose: u32, wealth: u32) -> u32 {
    use dev_type::*;
    match (purpose, wealth) {
        (1, 1) => R1,
        (1, 2) => R2,
        (1, 3) => R3,
        (2, 1) => CS1,
        (2, 2) => CS2,
        (2, 3) => CS3,
        (3, 2) => CO2,
        (3, 3) => CO3,
        (5, _) => I_AGRICULTURE,
        (6, _) => I_DIRTY,
        (7, _) => I_MANUFACTURING,
        (8, _) => I_HIGH_TECH,
        _ => 0,
    }
}

/// `GetWealth(devType)` (§4.7.3, slot 31).
pub fn wealth_of(dev_type: u32) -> u32 {
    use dev_type::*;
    match dev_type {
        R1 | CS1 | I_AGRICULTURE => 1,
        R2 | CS2 | CO2 | I_DIRTY | I_MANUFACTURING => 2,
        R3 | CS3 | CO3 | I_HIGH_TECH => 3,
        _ => 0,
    }
}

/// `GetBuildingMapKey(p, w)` (§4.7.3, slot 42): `uint8(4p + w + 1)`; key 0
/// means "no building".
pub fn building_map_key(purpose: u32, wealth: u32) -> u8 {
    purpose.wrapping_mul(4).wrapping_add(wealth).wrapping_add(1) as u8
}

/// `GrowthStageFitsInZone(stage, zone)` (§4.7.3, static `0x00695340`):
/// low-density zones 1, 4 take stages up to 3, medium zones 2, 5 up to 6,
/// zones 3 and 6–9 any stage, other zones none.
pub fn growth_stage_fits_in_zone(stage: u32, zone: i32) -> bool {
    match zone {
        1 | 4 => stage <= 3,
        2 | 5 => stage <= 6,
        3 | 6..=9 => true,
        _ => false,
    }
}

/// `BuildingFitsInZone(bz, cz)` (§4.7.3, static `0x00695390`): `bz` is the
/// building's zone type, `cz` the cell's. A denser zone accepts the
/// lower-density buildings of its family.
pub fn building_fits_in_zone(building_zone: i32, cell_zone: i32) -> bool {
    if building_zone == cell_zone {
        return true;
    }
    match cell_zone {
        2 => building_zone == 1,
        3 => building_zone == 1 || building_zone == 2,
        5 => building_zone == 4,
        6 => building_zone == 4 || building_zone == 5,
        9 => building_zone == 8,
        0x0B | 0x0C | 0x0F => building_zone == 0,
        _ => false,
    }
}

/// One entry of a growth developer's growth or decline list
/// (growth-developer §5, `{int32 x, int32 z, int32 maxStage}`, 12 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GrowthItem {
    /// Tract coordinates in the developer's grids.
    pub x: i32,
    pub z: i32,
    pub max_stage: i32,
}

/// `GrowthProfile` as the BDS builds it and the tract developer reads it
/// (BDS §4.4, tract-developer §7.1): the developer, the tract and its stage
/// (only the low byte of `stage` is used by the tract developer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrowthProfile {
    /// The growth developer, as a host token.
    pub developer: u64,
    pub x: i32,
    pub z: i32,
    pub stage: i32,
}

/// `ConstructionProfile` (growth-developer §6.2, BDS §4.8). Field names
/// follow the specs; `GetConstructionInfo` writes `zone` through
/// `land_value_max` only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstructionProfile {
    /// The building zone code (`unknown1`): the lowest zone type of the
    /// developer's family.
    pub zone: i32,
    /// `unknown2` (never set by the original: 0).
    pub unknown2: i32,
    pub wealth: u32,
    pub purpose: u32,
    pub land_value_min: u8,
    pub land_value_max: u8,
    pub building_type: u32,
    /// Cell coordinates.
    pub x: i32,
    pub z: i32,
}

/// Growth outcome codes (tract-developer §4.1). "Best" is always the
/// numerically largest; `Success` is the largest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Outcome {
    UnknownFailure = 0,
    NothingToReoccupy = 1,
    MonopolyPiece = 2,
    NoLot = 3,
    LotNotReady = 4,
    NoLotConfiguration = 5,
    WrongZone = 6,
    WrongPurpose = 7,
    WrongWealth = 8,
    WrongStyle = 9,
    TooSteep = 10,
    BuildingTooSmall = 11,
    GrowthStageTooSmall = 12,
    BuildingTooBig = 13,
    GrowthStageTooBig = 14,
    ResidentHasDifferentPurpose = 15,
    ResidentHasDifferentWealth = 16,
    NotBiggerThanExistingGrowthStage = 17,
    Historical = 18,
    JustBuiltRecently = 19,
    InsufficentConstructionResources = 20,
    PoorRoads = 21,
    WrongLandValue = 22,
    NoFireCoverage = 23,
    NoPark = 24,
    NoWater = 25,
    NoPower = 26,
    Success = 27,
}

impl Outcome {
    /// Every code in numeric order.
    pub const ALL: [Outcome; 28] = [
        Outcome::UnknownFailure,
        Outcome::NothingToReoccupy,
        Outcome::MonopolyPiece,
        Outcome::NoLot,
        Outcome::LotNotReady,
        Outcome::NoLotConfiguration,
        Outcome::WrongZone,
        Outcome::WrongPurpose,
        Outcome::WrongWealth,
        Outcome::WrongStyle,
        Outcome::TooSteep,
        Outcome::BuildingTooSmall,
        Outcome::GrowthStageTooSmall,
        Outcome::BuildingTooBig,
        Outcome::GrowthStageTooBig,
        Outcome::ResidentHasDifferentPurpose,
        Outcome::ResidentHasDifferentWealth,
        Outcome::NotBiggerThanExistingGrowthStage,
        Outcome::Historical,
        Outcome::JustBuiltRecently,
        Outcome::InsufficentConstructionResources,
        Outcome::PoorRoads,
        Outcome::WrongLandValue,
        Outcome::NoFireCoverage,
        Outcome::NoPark,
        Outcome::NoWater,
        Outcome::NoPower,
        Outcome::Success,
    ];

    pub fn code(self) -> u32 {
        self as u32
    }

    /// The outcome with this code, if any.
    pub fn from_code(code: u32) -> Option<Outcome> {
        Outcome::ALL.get(code as usize).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_types_and_keys() {
        // BDS §11 tests 1 and 2.
        assert_eq!(developer_type(1, 2), 0x1020);
        assert_eq!(developer_type(3, 1), 0);
        assert_eq!(developer_type(3, 3), 0x3330);
        assert_eq!(developer_type(5, 0), 0x4100);
        assert_eq!(developer_type(4, 2), 0);
        assert_eq!(developer_type(1, 0), 0);
        assert_eq!((wealth_of(0x4300), wealth_of(0x3320), wealth_of(0x1234)), (2, 2, 0));
        for t in dev_type::ALL {
            let p = match t >> 8 {
                0x10 => 1,
                0x31 => 2,
                0x33 => 3,
                0x41 => 5,
                0x42 => 6,
                0x43 => 7,
                _ => 8,
            };
            assert_eq!(developer_type(p, wealth_of(t)), t);
        }
        assert_eq!(
            (building_map_key(1, 1), building_map_key(3, 3), building_map_key(8, 3)),
            (6, 16, 36)
        );
        assert_eq!(building_map_key(64, 0), 1);
    }

    #[test]
    fn zone_fits() {
        // BDS §11 test 3.
        assert!(building_fits_in_zone(1, 3));
        assert!(!building_fits_in_zone(3, 1));
        assert!(building_fits_in_zone(0, 0x0B));
        assert!(building_fits_in_zone(8, 9));
        assert!(!building_fits_in_zone(4, 2));
        assert!(!growth_stage_fits_in_zone(4, 1));
        assert!(growth_stage_fits_in_zone(6, 5));
        assert!(growth_stage_fits_in_zone(15, 3));
        assert!(!growth_stage_fits_in_zone(1, 0));
    }

    #[test]
    fn outcome_codes() {
        for (i, o) in Outcome::ALL.iter().enumerate() {
            assert_eq!(o.code(), i as u32);
            assert_eq!(Outcome::from_code(i as u32), Some(*o));
        }
        assert_eq!(Outcome::from_code(28), None);
        assert!(Outcome::Success > Outcome::NoPower);
    }
}
