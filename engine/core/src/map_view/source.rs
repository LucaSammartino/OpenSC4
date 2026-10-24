// SPDX-License-Identifier: GPL-3.0-or-later
//! Raw simulator requests, not completed maps (map-view §§4.1, 7).
use super::Unsupported;

pub const ANIMATION_TICK: u32 = 0x86ad_10ee;
pub const ACTIVATE: u32 = 0x26d3_1ec1;
pub const DEACTIVATE: u32 = 0x26d3_1ec2;
pub const VALUE_CHANGED: u32 = 0xa6b7_9602;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    U8,
    I8,
    U16,
    I16,
    F32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Aura,
    LandValue,
    Police,
    Fire,
    Crime,
    Flammability,
    Power,
    Pollution,
    Traffic,
    Plumbing,
    Residential,
    /// City slot 70's object, then the requested slot; identity is inferred in §4.1.
    CitySlot70,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridRequest {
    pub provider: Provider,
    pub slot: u8,
    pub argument: Option<u32>,
    pub value_type: ValueType,
    pub own_cells: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeveloperGrid {
    CapacityPercentage,
    Unsigned16Slot14,
    Signed16Slot13,
    /// Select slot 11 entries and the documented ×12 operation only.
    /// Their layout/geometry is unpublished: this does not build a grid.
    DebugEntriesTimes12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceSelection {
    Clear,
    /// No source drawing; whether an old bitmap is cleared is not specified (QV9).
    DrawNothing,
    Grid(GridRequest),
    Developer {
        developer_type: u32,
        layer: u32,
        grid: DeveloperGrid,
    },
    TrafficVolumeQV2,
}

/// Select only the raw data provider. No palette, resampling, layer-5 or
/// layer-6 grid construction, marker generation or rendering is delegated.
pub fn select_source(source: u32, layer: u32, debug: bool) -> Result<SourceSelection, Unsupported> {
    use Provider::*;
    use ValueType::*;
    let grid = |provider, slot, argument, value_type, own_cells| {
        Ok(SourceSelection::Grid(GridRequest {
            provider,
            slot,
            argument,
            value_type,
            own_cells,
        }))
    };
    match source {
        0 => Ok(SourceSelection::Clear),
        1 => grid(Aura, 12, None, I8, false),
        2 => grid(LandValue, 8, None, U8, false),
        3 => grid(Police, 5, None, U8, false),
        4 => grid(Crime, 5, None, U8, false),
        5 => grid(Fire, 5, None, U8, false),
        6 => grid(Flammability, 5, None, U8, false),
        7 => grid(Power, 35, None, U8, false),
        8 => grid(Pollution, 5, Some(0), I16, false),
        9 => grid(Traffic, 10, None, U8, false),
        10 => grid(Plumbing, 7, None, U8, false),
        12 => grid(Aura, 14, None, I16, false),
        11 | 13..=23 | 31..=63 => Ok(SourceSelection::DrawNothing),
        24..=26 => grid(Pollution, 5, Some(source - 23), I16, false),
        27 => grid(Residential, 21, None, F32, false),
        28 => grid(Residential, 22, None, F32, false),
        29 => grid(CitySlot70, 18, None, U8, true),
        30 => grid(Residential, 41, None, U8, false),
        64..=75 => {
            const TYPES: [u32; 12] = [
                0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130, 0x3320, 0x3330, 0x4100, 0x4200, 0x4300, 0x4400,
            ];
            let layer = if debug { 6 } else { layer };
            let grid = match layer {
                2 => DeveloperGrid::Unsigned16Slot14,
                3 | 4 => DeveloperGrid::Signed16Slot13,
                5 => DeveloperGrid::CapacityPercentage,
                6 => DeveloperGrid::DebugEntriesTimes12,
                // [literal, QV4] Even the documented default layer 1 has no
                // published grid selection. Do not infer one from its name.
                _ => return Err(Unsupported::DeveloperLayer(layer)),
            };
            Ok(SourceSelection::Developer {
                developer_type: TYPES[(source - 64) as usize],
                layer,
                grid,
            })
        }
        76 => Ok(SourceSelection::TrafficVolumeQV2),
        _ => Err(Unsupported::UnknownSource(source)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateSubscription {
    pub message: u32,
    pub priority: u8,
}

pub fn subscription(source: u32) -> UpdateSubscription {
    let message = match source {
        1 => 0x2a09_b21a,
        2 => 0x87cc_75b3,
        4 => 0x88ea_4dff,
        7 => 0x6a07_c2ee,
        8 => 0xfa6d_d0bb,
        24 => 0x5a6d_d1aa,
        25 => 0xda6d_d2cb,
        26 => 0xda6d_d35d,
        9 | 76 => 0x6924_7dc7,
        10 => 0x6946_dc34,
        29 => 0x09b6_d58e,
        _ => ANIMATION_TICK,
    };
    UpdateSubscription {
        message,
        priority: if source == 76 { 0xc0 } else { 0 },
    }
}
