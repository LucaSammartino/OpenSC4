// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Crime types (§7), lot bits (§6.7) and the weighted type choice.

use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

use super::prop;

/// A mask that matches everything.
pub const ANY_BITS: u32 = 0xFFFF_FFFF;
/// The "none / other" bit of the zone, wealth and condition sets (§6.7).
pub const NONE_BIT: u32 = 0x8000_0000;
/// The purpose bit of "no purpose / no building" (§6.7).
pub const NO_PURPOSE_BIT: u32 = 0x100;

/// One crime type (§7).
#[derive(Clone, Debug, PartialEq)]
pub struct CrimeType {
    /// The exemplar instance.
    pub id: u32,
    pub name: Vec<u8>,
    pub icon: ResKey,
    pub zone_mask: u32,
    pub wealth_mask: u32,
    pub purpose_mask: u32,
    pub condition_mask: u32,
    /// Added to the crime grid (default 64).
    pub severity: u8,
    /// Starts a fire (default false).
    pub arson: bool,
    /// Selection weight (default 10).
    pub weight: f32,
}

/// Mask parsing (§7): each byte `b` sets bit `b − 1` (shift count modulo
/// 32), a byte 0 makes the mask all bits, an empty array gives 0 and a
/// missing property all bits.
pub fn parse_mask(bytes: Option<&[u8]>) -> u32 {
    let Some(bytes) = bytes else { return ANY_BITS };
    let mut m = 0u32;
    for &b in bytes {
        if b == 0 {
            m = ANY_BITS;
        } else {
            m |= 1u32 << (u32::from(b - 1) & 31);
        }
    }
    m
}

/// The bytes of a byte-array property: numeric elements reduced to their
/// low 8 bits; `None` if absent or not numeric ([choice]).
fn bytes_of(h: &PropertyHolder, id: u32) -> Option<Vec<u8>> {
    props::u32s(h, id).map(|v| v.into_iter().map(|x| x as u8).collect())
}

/// A mask property of a crime type exemplar (§7).
pub fn mask_of(h: &PropertyHolder, id: u32) -> u32 {
    parse_mask(bytes_of(h, id).as_deref())
}

impl CrimeType {
    /// Builds a type from its exemplar (§7). `ltext` resolves LTEXT keys.
    pub fn from_exemplar(instance: u32, h: &PropertyHolder, ltext: &dyn Fn(ResKey) -> Option<Vec<u8>>) -> CrimeType {
        let key = props::u32s(h, prop::TYPE_NAME_KEY)
            .filter(|v| v.len() >= 3)
            .map(|v| ResKey {
                type_id: v[0],
                group: v[1],
                instance: v[2],
            });
        let name = key
            .and_then(ltext)
            .or_else(|| props::string_of(h, prop::TYPE_NAME))
            .unwrap_or_default();
        let icon_values = props::u32s(h, prop::TYPE_ICON).unwrap_or_default();
        let part = |i: usize| icon_values.get(i).copied().unwrap_or(0);
        CrimeType {
            id: instance,
            name,
            icon: ResKey {
                type_id: part(0),
                group: part(1),
                instance: part(2),
            },
            zone_mask: mask_of(h, prop::TYPE_ZONE_MASK),
            wealth_mask: mask_of(h, prop::TYPE_WEALTH_MASK),
            purpose_mask: mask_of(h, prop::TYPE_PURPOSE_MASK),
            condition_mask: mask_of(h, prop::TYPE_CONDITION_MASK),
            severity: props::u32_of(h, prop::TYPE_SEVERITY).map_or(64, |v| v as u8),
            arson: props::u32_of(h, prop::TYPE_ARSON).is_some_and(|v| v as u8 != 0),
            weight: props::f32_of(h, prop::TYPE_WEIGHT).unwrap_or(10.0),
        }
    }

    /// Whether every mask shares a bit with the lot's set (§6.7).
    pub fn matches(&self, bits: &LotBits) -> bool {
        self.zone_mask & bits.zone != 0
            && self.wealth_mask & bits.wealth != 0
            && self.purpose_mask & bits.purpose != 0
            && self.condition_mask & bits.condition != 0
    }
}

/// Zone type → bits (§6.7).
pub fn zone_bits(zone: u32) -> u32 {
    match zone {
        1..=3 => 0x1,
        4..=6 => 0x2,
        7..=9 => 0x4,
        _ => NONE_BIT,
    }
}

/// Building wealth → bits; `None` = no building (§6.7).
pub fn wealth_bits(wealth: Option<i32>) -> u32 {
    match wealth {
        Some(w) if w >= 1 => 1u32 << ((w - 1) as u32 & 31),
        _ => NONE_BIT,
    }
}

/// Building purpose → bits; `None` = no building (§6.7).
pub fn purpose_bits(purpose: Option<i32>) -> u32 {
    match purpose {
        Some(p) if p >= 1 => 1u32 << ((p - 1) as u32 & 31),
        _ => NO_PURPOSE_BIT,
    }
}

/// Lot condition → bits (§6.7).
pub fn condition_bits(condition: u8) -> u32 {
    match condition {
        0 | 1 => 0x1,
        2 | 3 | 5 | 6 => 0x2,
        4 => 0x4,
        _ => NONE_BIT,
    }
}

/// The four bit sets of a lot (`GetLotBits`, §6.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LotBits {
    pub zone: u32,
    pub wealth: u32,
    pub purpose: u32,
    pub condition: u32,
}

impl LotBits {
    /// From the zone type, the building profile (`None` = no building) and
    /// the lot condition.
    pub fn new(zone: u32, building: Option<(i32, i32)>, condition: u8) -> LotBits {
        LotBits {
            zone: zone_bits(zone),
            wealth: wealth_bits(building.map(|b| b.1)),
            purpose: purpose_bits(building.map(|b| b.0)),
            condition: condition_bits(condition),
        }
    }
}

/// The walk of §6.7: `r := r − weight` over the candidates in order, the
/// first after which `r < 0`; otherwise the last. Returns an index into
/// `candidates`, or `None` when there are none. Each subtraction is one
/// 24-bit x87 operation (foundations §18): the double `r` enters
/// unrounded, the difference is rounded to `float32`.
pub fn choose_candidate(weights: &[f32], r: f64) -> Option<usize> {
    let last = weights.len().checked_sub(1)?;
    let mut r = r;
    for (i, &w) in weights.iter().enumerate() {
        r = f64::from((r - f64::from(w)) as f32);
        if r < 0.0 {
            return Some(i);
        }
    }
    Some(last)
}

/// The shipped crime type exemplars of §7 (`SimCity_1.dat`), in ascending
/// instance order (§16 question 4), as property maps. The name is given as
/// the fallback string `0xAA12FCB0`; the icon instance is not in the spec
/// and is 0 here. Masks not listed are `[0]`.
pub fn shipped_type_exemplars() -> Vec<(u32, PropertyHolder)> {
    const ALL_BUT_5: &[u8] = &[9, 1, 2, 3, 4, 6, 7, 8];
    #[allow(clippy::type_complexity)]
    let rows: [(u32, &str, u8, f32, bool, &[u8], &[u8], &[u8], &[u8]); 14] = [
        (0x01, "Generic", 40, 0.0, false, &[0], &[0], &[0], &[0]),
        (0x03, "Fighting", 60, 3.0, false, &[0], &[0], &[0], &[0]),
        (0x04, "Narcotic Deals", 80, 2.0, false, &[0], &[0], &[4, 2, 3], &[0]),
        (0x05, "Arson", 100, 0.5, true, &[0], &[2, 1], ALL_BUT_5, &[3]),
        (0x06, "Money Snatcher", 60, 3.0, false, &[0], &[0], &[4, 2], &[0]),
        (0x07, "Stickup", 80, 2.0, false, &[0], &[0], &[3, 2], &[0]),
        (0x08, "Car Theft", 60, 3.0, false, &[0], &[0], ALL_BUT_5, &[0]),
        (0x09, "Solicitation", 60, 3.0, false, &[0], &[0], &[4, 2, 3], &[0]),
        (0x0A, "Mugging", 80, 2.0, false, &[0], &[0], ALL_BUT_5, &[0]),
        (0x0B, "Bank Robbery", 80, 2.0, false, &[0], &[0], &[3, 2], &[0]),
        (0x10, "Flashing", 40, 4.0, false, &[0], &[0], ALL_BUT_5, &[0]),
        (0x11, "Graffiti", 40, 4.0, false, &[0], &[0], ALL_BUT_5, &[0]),
        (0x12, "Pie Throwing", 40, 4.0, false, &[0], &[0], ALL_BUT_5, &[0]),
        (0x13, "TPing", 40, 4.0, false, &[1], &[3, 2], &[1], &[0]),
    ];
    let bytes = |b: &[u8]| Variant::array(mem::UINT8, b.iter().map(|&x| Elem::U8(x)).collect());
    rows.iter()
        .map(
            |&(id, name, severity, weight, arson, zone, wealth, purpose, condition)| {
                let mut h = PropertyHolder::new();
                h.add(prop::TYPE_NAME, Variant::string(name.as_bytes()));
                h.add(
                    prop::TYPE_ICON,
                    Variant::array(
                        mem::UINT32,
                        vec![Elem::U32(0x7AB5_0E44), Elem::U32(0x1ABE_787D), Elem::U32(0)],
                    ),
                );
                h.add(prop::TYPE_ZONE_MASK, bytes(zone));
                h.add(prop::TYPE_WEALTH_MASK, bytes(wealth));
                h.add(prop::TYPE_PURPOSE_MASK, bytes(purpose));
                h.add(prop::TYPE_CONDITION_MASK, bytes(condition));
                h.add(prop::TYPE_SEVERITY, Variant::Scalar(Elem::U8(severity)));
                h.add(prop::TYPE_ARSON, Variant::Scalar(Elem::U8(u8::from(arson))));
                h.add(prop::TYPE_WEIGHT, Variant::Scalar(Elem::F32(weight)));
                (id, h)
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Vec<CrimeType> {
        shipped_type_exemplars()
            .iter()
            .map(|(id, h)| CrimeType::from_exemplar(*id, h, &|_| None))
            .collect()
    }

    #[test]
    fn masks() {
        assert_eq!(parse_mask(Some(&[0])), ANY_BITS);
        assert_eq!(parse_mask(Some(&[])), 0);
        assert_eq!(parse_mask(Some(&[1, 3])), 0x5);
        assert_eq!(parse_mask(Some(&[9])), 0x100);
        assert_eq!(parse_mask(None), ANY_BITS);
        assert_eq!(parse_mask(Some(&[33])), 0x1, "shift count modulo 32");
        assert_eq!(parse_mask(Some(&[255, 3])), 0x4000_0004);
        let mut h = PropertyHolder::new();
        h.add(1, Variant::string(b"x"));
        h.add(2, Variant::array(mem::UINT8, vec![]));
        h.add(3, Variant::Scalar(Elem::U8(2)));
        assert_eq!(mask_of(&h, 1), ANY_BITS);
        assert_eq!(mask_of(&h, 2), 0);
        assert_eq!(mask_of(&h, 3), 0x2);
        assert_eq!(mask_of(&h, 4), ANY_BITS);
    }

    #[test]
    fn exemplar_defaults_and_names() {
        let mut h = PropertyHolder::new();
        h.add(
            prop::TYPE_NAME_KEY,
            Variant::array(mem::UINT32, vec![Elem::U32(1), Elem::U32(2), Elem::U32(3)]),
        );
        h.add(prop::TYPE_NAME, Variant::string(b"fallback"));
        h.add(prop::TYPE_ICON, Variant::array(mem::UINT32, vec![Elem::U32(7)]));
        let t = CrimeType::from_exemplar(9, &h, &|k| (k.instance == 3).then(|| b"ltext".to_vec()));
        assert_eq!(t.name, b"ltext");
        assert_eq!((t.severity, t.arson, t.weight), (64, false, 10.0));
        assert_eq!(
            t.icon,
            ResKey {
                type_id: 7,
                group: 0,
                instance: 0
            }
        );
        assert_eq!(
            t.zone_mask & t.wealth_mask & t.purpose_mask & t.condition_mask,
            ANY_BITS
        );
        let t = CrimeType::from_exemplar(9, &h, &|_| None);
        assert_eq!(t.name, b"fallback");
        let t = CrimeType::from_exemplar(9, &PropertyHolder::new(), &|_| None);
        assert!(t.name.is_empty());
    }

    #[test]
    fn lot_bits() {
        assert_eq!(
            [0, 1, 3, 4, 6, 7, 9, 10].map(zone_bits),
            [NONE_BIT, 1, 1, 2, 2, 4, 4, NONE_BIT]
        );
        assert_eq!(wealth_bits(None), NONE_BIT);
        assert_eq!(wealth_bits(Some(0)), NONE_BIT);
        assert_eq!(wealth_bits(Some(-3)), NONE_BIT);
        assert_eq!(wealth_bits(Some(3)), 4);
        assert_eq!(purpose_bits(None), NO_PURPOSE_BIT);
        assert_eq!(purpose_bits(Some(0)), NO_PURPOSE_BIT);
        assert_eq!(purpose_bits(Some(9)), 0x100);
        assert_eq!(purpose_bits(Some(33)), 1);
        assert_eq!(
            [0, 1, 2, 3, 4, 5, 6, 7, 255].map(condition_bits),
            [1, 1, 2, 2, 4, 2, 2, NONE_BIT, NONE_BIT]
        );
    }

    /// Test 9 (shipped types, ascending instance order).
    #[test]
    fn selection() {
        let types = shipped();
        let bits = LotBits::new(1, Some((1, 1)), 1);
        let cands: Vec<&CrimeType> = types.iter().filter(|t| t.matches(&bits)).collect();
        let ids: Vec<u32> = cands.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![0x01, 0x03, 0x08, 0x0A, 0x10, 0x11, 0x12]);
        let w: Vec<f32> = cands.iter().map(|t| t.weight).collect();
        assert_eq!(w.iter().sum::<f32>(), 20.0);
        assert_eq!(cands[choose_candidate(&w, 10.0).unwrap()].id, 0x10);
        assert_eq!(cands[choose_candidate(&w, 2.0).unwrap()].id, 0x03);
        assert_eq!(cands[choose_candidate(&w, 0.0).unwrap()].id, 0x03);
        assert_eq!(choose_candidate(&[0.0, 0.0, 0.0], 0.0), Some(2));
        assert_eq!(choose_candidate(&[f32::NAN, 1.0], 0.5), Some(1));
        assert_eq!(choose_candidate(&[], 0.0), None);
        // TPing needs $$ or $$$; arson needs condition 4.
        let tp = types.iter().find(|t| t.id == 0x13).unwrap();
        assert!(!tp.matches(&bits));
        assert!(tp.matches(&LotBits::new(2, Some((1, 2)), 0)));
        let arson = types.iter().find(|t| t.id == 0x05).unwrap();
        assert!(arson.arson && !arson.matches(&bits));
        assert!(arson.matches(&LotBits::new(1, Some((1, 1)), 4)));
        // No building: wealth 0x80000000, purpose 0x100.
        let none = LotBits::new(1, None, 0);
        assert_eq!((none.wealth, none.purpose), (NONE_BIT, NO_PURPOSE_BIT));
    }
}
