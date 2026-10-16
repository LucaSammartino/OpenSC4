// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot configuration object record (lot-developer.md §17.1) and the
//! detail filter of prop-lotconfig.md §B3.3.

use crate::property::PropertyHolder;
use crate::props;

use super::{kind, prop};

/// One 0x34-byte lot configuration object (`cSC4LotConfigurationObject`,
/// lot-developer.md §17.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LotConfigObject {
    /// +0x00: the object type `v0`.
    pub kind: u8,
    /// +0x01: `v1` (bits 4–5: the LOD).
    pub flags: u8,
    /// +0x02: the low byte of `v2`.
    pub orientation: u8,
    /// +0x03: `(v2 >> 31) & 1`.
    pub mirror: u8,
    /// +0x04, +0x08, +0x0C: `v3`, `v4`, `v5` (16.16).
    pub offset: [f32; 3],
    /// +0x10, +0x14: `v8`, `v9` (16.16).
    pub max_x: f32,
    pub max_z: f32,
    /// +0x18, +0x1C: `v6`, `v7` (16.16).
    pub min_x: f32,
    pub min_z: f32,
    /// +0x20: `v10`.
    pub foundation: u32,
    /// +0x24: the lot tag (`v11`, or a generated serial number).
    pub tag: u32,
    /// +0x28: the remaining values.
    pub ids: Vec<u32>,
}

/// A signed 16.16 value as the reader converts it (`fild`, × the double
/// 2^-16, stored as a float).
pub fn fixed_16_16(v: u32) -> f32 {
    (f64::from(v as i32) * (1.0 / 65536.0)) as f32
}

/// `LOD := (v1 >> 4) & 3` (§B3.3).
pub fn lod_of(v1: u32) -> u32 {
    (v1 >> 4) & 3
}

/// Whether an object is kept at detail `d` (§B3.3): the building (type 0)
/// always, everything else when `LOD < d`.
pub fn is_kept(v0: u32, v1: u32, detail: u32) -> bool {
    v0 == kind::BUILDING || lod_of(v1) < detail
}

/// One record from its values (at least 10). `tag`: `Some(serial)` for a
/// generated tag (the IDs then start at `v11`), `None` to read `v11` (the IDs
/// start at `v12`). Missing values past `v9` read as 0 / empty [choice].
pub fn object_from_values(v: &[u32], tag: Option<u32>) -> LotConfigObject {
    let at = |i: usize| v.get(i).copied().unwrap_or(0);
    let (tag, ids_from) = match tag {
        Some(t) => (t, 11),
        None => (at(11), 12),
    };
    LotConfigObject {
        kind: at(0) as u8,
        flags: at(1) as u8,
        orientation: at(2) as u8,
        mirror: ((at(2) >> 31) & 1) as u8,
        offset: [fixed_16_16(at(3)), fixed_16_16(at(4)), fixed_16_16(at(5))],
        min_x: fixed_16_16(at(6)),
        min_z: fixed_16_16(at(7)),
        max_x: fixed_16_16(at(8)),
        max_z: fixed_16_16(at(9)),
        foundation: at(10),
        tag,
        ids: v.get(ids_from..).map(<[u32]>::to_vec).unwrap_or_default(),
    }
}

/// The object array `ConvertPropertiesToObjectArray` (`0x00479D10`) builds
/// at detail `d` (§B3.3, lot-configuration.md §3.5): properties
/// `0x88EDC900`, `0x88EDC901`, … up to and including `0x88EDCDFF` (the scan
/// stops at `0x88EDCE00`: at most 1280 objects); the scan stops at the first
/// missing property; a property with fewer than 10 values is skipped; the
/// kept objects follow in property order without gaps.
///
/// The configuration's version is property `0x88EDC789`. `Load` refuses an
/// absent or 0 version before building the array (lot-configuration.md
/// §3.1); here an absent one reads as 1 [choice]. Below 2 each kept object
/// gets the tag `*next_serial`, which then advances by one: the process-wide
/// serial `0x00B2166C` of lot-configuration.md §3.5 (the host seeds it from
/// the GZCOM clock when 0), and the ID list starts at `v11`.
pub fn lot_object_array(h: &PropertyHolder, detail: u32, next_serial: &mut u32) -> Vec<LotConfigObject> {
    let version = props::u32_of(h, prop::VERSION).unwrap_or(1);
    let mut out = Vec::new();
    for id in prop::LOT_OBJECT_FIRST..=prop::LOT_OBJECT_END {
        let Some(values) = props::u32s(h, id) else {
            break;
        };
        if values.len() < 10 || !is_kept(values[0], values[1], detail) {
            continue;
        }
        let tag = if version < 2 {
            let t = *next_serial;
            *next_serial = next_serial.wrapping_add(1);
            Some(t)
        } else {
            None
        };
        out.push(object_from_values(&values, tag));
    }
    out
}
