// SPDX-License-Identifier: GPL-3.0-or-later
//! Independently derived configuration getters (lot-configuration §§3, 5).
use crate::{property::PropertyHolder, props};

use super::{lot_object_array, LotConfigObject};
use crate::variant::{Elem, Variant};

/// Plain fields shared with the record-only C ABI projection. Capacity and
/// cost are deliberately absent: native comparison contracts await GLUE22.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ConfigurationFields {
    pub width: u32,
    pub depth: u32,
    pub zones: u32,
    pub wealths: u32,
    pub purposes: u32,
    pub growth_stage: u32,
    pub water: u32,
    pub construction: u32,
    pub roads: u32,
    pub max_slope: f32,
    pub min_slope: f32,
    pub foundation_slope: f32,
}

// §3.2 byte-vector helpers copy count bytes, even from wider storage.
// Scalars have no array data (EX39). No numeric widening here.
fn byte_vector(h: &PropertyHolder, id: u32) -> Vec<u8> {
    let Some(Variant::Array { items, .. }) = h.get(id) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    for item in items {
        match *item {
            Elem::Bool(v) => bytes.push(u8::from(v)),
            Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => bytes.push(v),
            Elem::I8(v) => bytes.push(v as u8),
            Elem::U16(v) | Elem::UnicodeChar(v) => bytes.extend(v.to_le_bytes()),
            Elem::I16(v) => bytes.extend(v.to_le_bytes()),
            Elem::U32(v) => bytes.extend(v.to_le_bytes()),
            Elem::I32(v) => bytes.extend(v.to_le_bytes()),
            Elem::U64(v) => bytes.extend(v.to_le_bytes()),
            Elem::I64(v) => bytes.extend(v.to_le_bytes()),
            Elem::F32(v) => bytes.extend(v.to_le_bytes()),
            Elem::F64(v) => bytes.extend(v.to_le_bytes()),
            Elem::VoidPtr(v) => bytes.extend((v as u32).to_le_bytes()),
        }
    }
    bytes.truncate(items.len());
    bytes
}

fn mask(h: &PropertyHolder, id: u32, limit: u32) -> u32 {
    byte_vector(h, id).iter().fold(0, |bits, &v| {
        let v = u32::from(v);
        bits | if v < limit { 1 << v } else { 0 }
    })
}

pub fn configuration_fields(h: &PropertyHolder) -> Option<ConfigurationFields> {
    if props::u32_of(h, 0x88ED_C789).unwrap_or(0) as u8 == 0 {
        return None;
    }
    let size = byte_vector(h, 0x88ED_C790);
    let (width, depth) = if size.len() >= 2 {
        (size[0], size[1])
    } else {
        (0, 0)
    };
    Some(ConfigurationFields {
        width: width.into(),
        depth: depth.into(),
        zones: mask(h, 0x88ED_C793, 16),
        wealths: mask(h, 0x88ED_C795, 4),
        purposes: mask(h, 0x88ED_C796, 9),
        growth_stage: u32::from(props::u32_of(h, 0x2781_2837).unwrap_or(1) as u8),
        water: u32::from(props::u32_of(h, 0x8913_6440).unwrap_or(0) as u8 != 0),
        construction: u32::from(props::u32_of(h, 0xE99B_068D).unwrap_or(0) as u8 != 0),
        roads: u32::from(props::u32_of(h, 0x4A4A_88F0).unwrap_or(0) as u8 | 8),
        max_slope: props::f32_of(h, 0xE99B_068C).unwrap_or(90.0),
        min_slope: props::f32_of(h, 0x699B_08A4).unwrap_or(0.0),
        foundation_slope: props::f32_of(h, 0x88ED_C792).unwrap_or(90.0),
    })
}

/// Full Load array, including §3.6 position correction and §3.7 foundation.
pub fn configuration_objects(h: &PropertyHolder, detail: u32, serial: &mut u32) -> Vec<LotConfigObject> {
    let Some(fields) = configuration_fields(h) else {
        return Vec::new();
    };
    let mut objects = lot_object_array(h, detail, serial);
    for o in &mut objects {
        for (axis, size) in [(0, fields.width), (2, fields.depth)] {
            let lo = 0.040_000_003_f32;
            let hi = 16.0 * size as f32 - lo;
            let x = o.offset[axis];
            let delta = if x < lo {
                lo - x
            } else if x > hi {
                hi - x
            } else {
                continue;
            };
            o.offset[axis] += delta;
            if axis == 0 {
                o.min_x += delta;
                o.max_x += delta;
            } else {
                o.min_z += delta;
                o.max_z += delta;
            }
        }
    }
    if let Some(foundation) = props::u32_of(h, 0x88FC_D877) {
        objects.push(LotConfigObject {
            kind: 3,
            flags: 0,
            orientation: 0,
            mirror: 0,
            offset: [0.5, 0.0, 0.5],
            max_x: 0.6,
            max_z: 0.6,
            min_x: 0.4,
            min_z: 0.4,
            foundation,
            tag: 0, // §3.7 [choice]; native uninitialised tag is separately reported (PL23).
            ids: Vec::new(),
        });
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::{mem, Elem, Variant};

    fn array(h: &mut PropertyHolder, id: u32, values: &[u32]) {
        h.add(
            id,
            Variant::array(mem::UINT32, values.iter().copied().map(Elem::U32).collect()),
        );
    }

    fn bytes(h: &mut PropertyHolder, id: u32, values: &[u8]) {
        h.add(
            id,
            Variant::array(mem::UINT8, values.iter().copied().map(Elem::U8).collect()),
        );
    }

    #[test]
    fn defaults_shapes_and_compatibility_bounds() {
        let mut h = PropertyHolder::new();
        assert!(configuration_fields(&h).is_none());
        h.add(0x88ED_C789, Variant::Scalar(Elem::U32(2)));
        h.add(0x88ED_C796, Variant::Scalar(Elem::U8(0)));
        bytes(&mut h, 0x88ED_C790, &[3]);
        bytes(&mut h, 0x88ED_C793, &[0, 15, 16, 255]);
        bytes(&mut h, 0x88ED_C795, &[1, 3, 4]);
        let f = configuration_fields(&h).unwrap();
        assert_eq!(
            (f.width, f.depth, f.zones, f.wealths, f.purposes),
            (0, 0, 0x8001, 10, 0)
        );
        assert_eq!(
            (f.roads, f.growth_stage, f.max_slope, f.foundation_slope),
            (8, 1, 90.0, 90.0)
        );
        array(&mut h, 0x88ED_C790, &[0x0403, 99]);
        let f = configuration_fields(&h).unwrap();
        assert_eq!((f.width, f.depth), (3, 4));
    }

    #[test]
    fn lod_correction_and_unfiltered_foundation() {
        let mut h = PropertyHolder::new();
        h.add(0x88ED_C789, Variant::Scalar(Elem::U32(2)));
        h.add(0x88FC_D877, Variant::Scalar(Elem::U32(123)));
        bytes(&mut h, 0x88ED_C790, &[1, 2]);
        // Building survives LOD 3 at low detail; prop does not.
        array(
            &mut h,
            0x88ED_C900,
            &[0, 0x30, 0, 0, 0, 40 << 16, 0, 0, 1 << 16, 1 << 16, 0, 7, 42],
        );
        array(&mut h, 0x88ED_C901, &[1, 0x30, 0, 0, 0, 0, 0, 0, 0, 0]);
        let objects = configuration_objects(&h, 1, &mut 0);
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].offset[0], 0.040_000_003);
        assert_eq!(objects[0].offset[2], 32.0 - 0.040_000_003);
        assert_eq!((objects[0].tag, objects[0].ids.as_slice()), (7, &[42][..]));
        assert_eq!((objects[1].kind, objects[1].foundation, objects[1].tag), (3, 123, 0));
    }
}
