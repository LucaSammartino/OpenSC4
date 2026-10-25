// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed raw resource properties (Â§2). No host-normalized MapView configuration.
use super::Unsupported;

pub const SOURCE: u32 = 0x4a0b_47e5;
pub const RAMP: u32 = 0x4a0b_47e4;
pub const SCALE: u32 = 0x4a0b_47ec;
pub const INTERPOLATE: u32 = 0x4a0b_47e7;
pub const BLOCK_BY_LOT: u32 = 0x4a0b_47e8;
pub const LAYER: u32 = 0x4a0b_47ed;
pub const HIGHLIGHT: u32 = 0x4a0b_47e9;
pub const OCCUPANT_TYPES: u32 = 0x4a0b_47e0;
pub const TERRAIN_MODE: u32 = 0x4a0b_47e3;
pub const TERRAIN_MODE_SECOND: u32 = 0x4a0b_47f1;
pub const CHILD_FLAG: u32 = 0x4a0b_47f5;
pub const TRAVEL_TYPES: u32 = 0x6b7c_5cdd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExemplarKey {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u32,
}

pub fn exemplar_key(view: u32) -> Result<ExemplarKey, Unsupported> {
    Ok(ExemplarKey {
        resource_type: 0x6534_284a,
        group: 0x690f_693f,
        instance: 0x4a0b_6800u32.wrapping_add(view),
    })
}

/// Values as typed by the external resource, including lists as lists.
/// Unknown bytes can be retained but cannot masquerade as a typed property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawValue {
    U32(u32),
    U8(u8),
    Bool(bool),
    U32List(Vec<u32>),
    String(String),
    Uninterpreted(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub id: u32,
    pub value: RawValue,
}

impl Property {
    pub fn new(id: u32, value: RawValue) -> Self {
        Self { id, value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawExemplar(pub Vec<Property>);

/// Only fields needed before the ramp boundary. This is core-produced state;
/// no public API accepts it in place of raw exemplar properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionFields {
    pub source: u32,
    pub scale: u32,
    pub layer: u32,
    pub interpolate: bool,
    pub block_by_lot: Option<u32>,
    pub highlight_kind: u32,
    pub ramp_pairs: Vec<u32>,
}

impl RawExemplar {
    pub(super) fn find(&self, id: u32) -> Result<Option<&RawValue>, Unsupported> {
        // The adapter supplies the property-holder lookup result, not duplicate assignments.
        Ok(self.0.iter().find(|p| p.id == id).map(|p| &p.value))
    }

    pub fn raw_field(&self, id: u32) -> Result<&RawValue, Unsupported> {
        self.find(id)?.ok_or(Unsupported::MissingProperty(id))
    }

    pub fn u32_field(&self, id: u32) -> Result<u32, Unsupported> {
        match self.raw_field(id)? {
            RawValue::U32(v) => Ok(*v),
            RawValue::U8(v) => Ok(u32::from(*v)),
            RawValue::Bool(v) => Ok(u32::from(*v)),
            RawValue::Uninterpreted(v) if v.len() >= 4 => Ok(u32::from_le_bytes([v[0], v[1], v[2], v[3]])),
            // Pointer payloads are not dereferenced by the portable core.
            _ => Err(Unsupported::MalformedProperty(id)),
        }
    }

    pub fn bool_field(&self, id: u32) -> Result<bool, Unsupported> {
        match self.raw_field(id)? {
            RawValue::Bool(v) => Ok(*v),
            RawValue::U8(v) => Ok(*v != 0),
            RawValue::U32(v) => Ok(*v as u8 != 0),
            _ => Err(Unsupported::MalformedProperty(id)),
        }
    }

    pub fn u32_list(&self, id: u32) -> Result<&[u32], Unsupported> {
        match self.raw_field(id)? {
            RawValue::U32List(v) => Ok(v),
            _ => Err(Unsupported::MalformedProperty(id)),
        }
    }

    pub(super) fn optional_u32(&self, id: u32) -> Result<Option<u32>, Unsupported> {
        if self.find(id)?.is_some() {
            self.u32_field(id).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn selection_fields(&self) -> Result<SelectionFields, Unsupported> {
        let source = self.optional_u32(SOURCE)?.unwrap_or(0);
        let ramp = if self.find(RAMP)?.is_some() {
            self.u32_list(RAMP)?
        } else {
            &[]
        };
        let scale = self.optional_u32(SCALE)?.unwrap_or(256);
        Ok(SelectionFields {
            source,
            scale,
            layer: self.optional_u32(LAYER)?.unwrap_or(1),
            interpolate: self.bool_or(INTERPOLATE, false)? || self.find(BLOCK_BY_LOT)?.is_some(),
            // Presence, including argument zero, is significant (Â§2).
            block_by_lot: self.optional_u32(BLOCK_BY_LOT)?,
            highlight_kind: self.optional_u32(HIGHLIGHT)?.unwrap_or(0),
            ramp_pairs: ramp.to_vec(),
        })
    }

    /// Raw nonzero occupant type selection only, not construction of a filter (QV1).
    pub fn occupant_types(&self) -> Result<Vec<u32>, Unsupported> {
        Ok(self
            .u32_list(OCCUPANT_TYPES)?
            .iter()
            .copied()
            .filter(|v| *v != 0)
            .collect())
    }

    /// The literal value mapping, without inferring a terrain/zone API call.
    pub fn terrain_mode(&self, property: u32) -> Result<u32, Unsupported> {
        if property != TERRAIN_MODE && property != TERRAIN_MODE_SECOND {
            return Err(Unsupported::MalformedProperty(property));
        }
        let entering = if property == TERRAIN_MODE_SECOND {
            self.terrain_mode(TERRAIN_MODE)?
        } else {
            1
        };
        match self.optional_u32(property)? {
            Some(0) => Ok(1),
            Some(1) => Ok(3),
            Some(2) => Ok(2),
            _ => Ok(entering),
        }
    }

    /// Only the unambiguous domain: at most eight entries, all already bytes.
    /// Long lists/high bits are refused, not silently truncated (QV6).
    /// This never selects or renders the traffic volume grid (QV2).
    pub fn travel_types(&self) -> Result<Vec<u8>, Unsupported> {
        let list = self.u32_list(TRAVEL_TYPES)?;
        Ok(list
            .iter()
            .take((list.len() as u8).min(8) as usize)
            .map(|v| *v as u8)
            .collect())
    }
}
