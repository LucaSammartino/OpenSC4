//! The power pole's static control variables (network-occupants.md §4.3,
//! `InitPowerLineControlVariables`). The pole object itself (§4.1, §4.2,
//! §4.4) is not implemented here.

use crate::stream::ResKey;

/// The exemplar the control variables come from (§4.3).
pub const CONTROL_EXEMPLAR: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC911_E35B,
};

pub const PROP_E9A18D3F: u32 = 0xE9A1_8D3F;
pub const PROP_49A1E8C4: u32 = 0x49A1_E8C4;
pub const PROP_89A18FC4: u32 = 0x89A1_8FC4;
pub const PROP_E9A1E8D4: u32 = 0xE9A1_E8D4;
pub const PROP_49A195C0: u32 = 0x49A1_95C0;
pub const PROP_89A1E8FB: u32 = 0x89A1_E8FB;

/// The six control variables (§4.3). Their meanings are not named by the
/// spec; the fields carry the property IDs and the static addresses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerLineControl {
    /// `0xE9A18D3F` → `0xB46754`, default 16.0.
    pub v_e9a18d3f: f32,
    /// `0x49A1E8C4` → `0xB46750`, default 10.0.
    pub v_49a1e8c4: f32,
    /// `0x89A18FC4` → `0xB4674C`, default 10.0.
    pub v_89a18fc4: f32,
    /// `0xE9A1E8D4` → `0xB46748`, default 0.15 (`0x3E19999A`).
    pub v_e9a1e8d4: f32,
    /// `0x49A195C0` → `0xB46744`, default 8.0.
    pub v_49a195c0: f32,
    /// `0x89A1E8FB` → `0xB46740`, default 5 (an integer default; the
    /// spec does not give the static's type, so it is kept as u32).
    pub v_89a1e8fb: u32,
}

impl Default for PowerLineControl {
    fn default() -> Self {
        PowerLineControl {
            v_e9a18d3f: 16.0,
            v_49a1e8c4: 10.0,
            v_89a18fc4: 10.0,
            v_e9a1e8d4: f32::from_bits(0x3E19_999A),
            v_49a195c0: 8.0,
            v_89a1e8fb: 5,
        }
    }
}

impl PowerLineControl {
    /// Reads the variables from the control exemplar: each property that
    /// `float`/`uint` answers replaces its default; a missing one keeps it.
    pub fn load(float: impl Fn(u32) -> Option<f32>, uint: impl Fn(u32) -> Option<u32>) -> Self {
        let d = PowerLineControl::default();
        PowerLineControl {
            v_e9a18d3f: float(PROP_E9A18D3F).unwrap_or(d.v_e9a18d3f),
            v_49a1e8c4: float(PROP_49A1E8C4).unwrap_or(d.v_49a1e8c4),
            v_89a18fc4: float(PROP_89A18FC4).unwrap_or(d.v_89a18fc4),
            v_e9a1e8d4: float(PROP_E9A1E8D4).unwrap_or(d.v_e9a1e8d4),
            v_49a195c0: float(PROP_49A195C0).unwrap_or(d.v_49a195c0),
            v_89a1e8fb: uint(PROP_89A1E8FB).unwrap_or(d.v_89a1e8fb),
        }
    }
}
