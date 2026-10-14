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

/// The raw storage of an array value, as the crime effect `0xCA5B9306` is
/// read (§6.10, CQ13): the bytes the variant's data pointer (`RefUint8`,
/// slot 95) points at, `count × element size`, each element little-endian,
/// whatever the element type (bool 1 byte, a pointer 4, as in the 32-bit
/// game). No numeric conversion: `Uint32 {100, 8}` gives `64 00 00 00 08 00
/// 00 00`, so m = 100, r = 0. A string (an `RZChar` array, tag `0x800C`
/// in a file) is its stored bytes, so `"AB"` gives m = 65, r = 66 (CQ22).
/// [choice] `None` for a scalar (the original dereferences its value as a
/// pointer), an object or object array (the original reads the low bytes of
/// an address; CQ22) or an empty value.
pub fn raw_array_bytes(v: &Variant) -> Option<Vec<u8>> {
    let Variant::Array { items, .. } = v else {
        return None;
    };
    let mut out = Vec::new();
    for e in items {
        match *e {
            Elem::Bool(b) => out.push(u8::from(b)),
            Elem::U8(x) | Elem::Char(x) | Elem::RzChar(x) => out.push(x),
            Elem::I8(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::U16(x) | Elem::UnicodeChar(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::I16(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::U32(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::I32(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::U64(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::I64(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::F32(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::F64(x) => out.extend_from_slice(&x.to_le_bytes()),
            Elem::VoidPtr(x) => out.extend_from_slice(&(x as u32).to_le_bytes()),
        }
    }
    Some(out)
}

/// The LTEXT type of every crime type name: value [0] of the name key
/// `0x8A416B2E` is ignored (§7 step 1, CQ8).
pub const NAME_LTEXT_TYPE: u32 = 0x2026_960B;

/// What the host's LTEXT load of a crime type name did: §7 steps 2 and 3,
/// which need the game's language manager, resource manager and string
/// detokenizer (CQ8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LtextLoad {
    /// Neither (`0x2026960B`, G + language ID, I) nor (`0x2026960B`, G, I)
    /// exists: the load fails and the name is untouched.
    Missing,
    /// The text was found, copied into the name and detokenized in place
    /// (the step is skipped with no detokenizer).
    Loaded(Vec<u8>),
    /// The text was found and copied, but the detokenizer returned false:
    /// the load fails and the copied text is discarded. A failure in step 2
    /// or 3 empties the name (CQ16: `SC4LoadString`'s failure exit resizes
    /// the string to 0), so the name is empty unless `0xAA12FCB0` exists.
    DetokenizeFailed,
}

impl From<Option<Vec<u8>>> for LtextLoad {
    /// `Some(text)`: loaded; `None`: missing.
    fn from(text: Option<Vec<u8>>) -> LtextLoad {
        text.map_or(LtextLoad::Missing, LtextLoad::Loaded)
    }
}

/// A key property read as the original reads the name and icon keys (§7
/// "How step 1 reads the key" and "Other value types", CQ17, CQ23): the
/// first 12 bytes of the value's data (its raw storage, [`raw_array_bytes`])
/// as three little-endian `u32`, whatever the element type. No value is
/// converted: a u16 array gives `e[2] + (e[3] << 16)` as value [1], an f32
/// array the bits, a string its bytes. [choice] (CQ23, recommended; CQ17) A
/// value with fewer than 12 data bytes, a scalar, an object or an empty
/// value counts as missing: the original reads past the data block there
/// (garbage, or a fault for a scalar).
pub fn raw_key3(h: &PropertyHolder, id: u32) -> Option<[u32; 3]> {
    let b = raw_array_bytes(h.get(id)?)?;
    if b.len() < 12 {
        return None;
    }
    let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    Some([word(0), word(4), word(8)])
}

/// The LTEXT key of a crime type's name (§7 step 1): values [1] and [2] of
/// `0x8A416B2E` (bytes 4-7 and 8-11 of its data, [`raw_key3`]) are the group
/// and instance, [0] is ignored and the type is always [`NAME_LTEXT_TYPE`].
/// `None` (no lookup, the load fails) when the property is missing or holds
/// fewer than 12 data bytes ([choice] CQ17, CQ23): the original's lookup of
/// a key read past the data almost always misses, so this gives its usual
/// outcome without reading past the data.
pub fn name_ltext_key(h: &PropertyHolder) -> Option<ResKey> {
    raw_key3(h, prop::TYPE_NAME_KEY).map(|v| ResKey {
        type_id: NAME_LTEXT_TYPE,
        group: v[1],
        instance: v[2],
    })
}

/// §7 step 4 (data-formats §6.1): every U+2022 (`E2 80 A2`) becomes U+200B
/// (`E2 80 8B`), in place. Both are three bytes, so nothing else moves.
pub fn bullets_to_zero_width_spaces(text: &mut [u8]) {
    let mut i = 0;
    while i + 3 <= text.len() {
        if text[i..i + 3] == [0xE2, 0x80, 0xA2] {
            text[i + 2] = 0x8B;
            i += 3;
        } else {
            i += 1;
        }
    }
}

impl CrimeType {
    /// Builds a type from its exemplar (§7). `ltext` runs steps 2 and 3 of
    /// the name load for the key of [`name_ltext_key`] (CQ8).
    ///
    /// Name: (0) the entry starts empty; (1) the LTEXT key; (2)-(3) the
    /// host's lookup and detokenizer; (4) U+2022 → U+200B. A failed load
    /// leaves the name **empty** in every case (CQ16: step 0 empties it, and
    /// a failure in step 2 or 3 discards the copied text), and the string
    /// `0xAA12FCB0` replaces it only when that property exists.
    pub fn from_exemplar(instance: u32, h: &PropertyHolder, ltext: &dyn Fn(ResKey) -> LtextLoad) -> CrimeType {
        let name = match name_ltext_key(h).map(ltext) {
            Some(LtextLoad::Loaded(mut text)) => {
                bullets_to_zero_width_spaces(&mut text);
                text
            }
            None | Some(LtextLoad::Missing | LtextLoad::DetokenizeFailed) => {
                props::string_of(h, prop::TYPE_NAME).unwrap_or_default()
            }
        };
        // The icon key goes through the same reader as the name key (§7
        // "Other value types", CQ23): missing or under 12 data bytes gives
        // the default (0, 0, 0).
        let [icon_type, icon_group, icon_instance] = raw_key3(h, prop::TYPE_ICON).unwrap_or([0; 3]);
        CrimeType {
            id: instance,
            name,
            icon: ResKey {
                type_id: icon_type,
                group: icon_group,
                instance: icon_instance,
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

/// The type and group of the shipped crime icons (§7).
pub const SHIPPED_ICON_TYPE: u32 = 0x7AB5_0E44;
pub const SHIPPED_ICON_GROUP: u32 = 0x1ABE_787D;
/// The type and group of the shipped crime name keys (LTEXT, §7).
pub const SHIPPED_NAME_TYPE: u32 = 0x2026_960B;
pub const SHIPPED_NAME_GROUP: u32 = 0x0A55_4AE0;
/// The exemplar name property (`0x20`), which the simulator does not read.
const EXEMPLAR_NAME: u32 = 0x20;

/// The shipped crime type exemplars of §7 (`SimCity_1.dat`), in ascending
/// instance order (§16 question 4), as property maps: the name key
/// `0x8A416B2E` and the icon key `0xAA12FCB1` of the spec's table, no
/// inline name `0xAA12FCB0` (none is shipped), and the exemplar name as
/// property `0x20`. Masks not listed are `[0]`. The names therefore load
/// from the host's LTEXT strings (empty when it has none, as in the shipped
/// game for every type but Generic).
pub fn shipped_type_exemplars() -> Vec<(u32, PropertyHolder)> {
    const ALL_BUT_5: &[u8] = &[9, 1, 2, 3, 4, 6, 7, 8];
    #[allow(clippy::type_complexity)]
    let rows: [(u32, &str, u8, f32, bool, [&[u8]; 4], u32, u32); 14] = [
        (
            0x01,
            "Generic",
            40,
            0.0,
            false,
            [&[0], &[0], &[0], &[0]],
            0x8A1D_C833,
            0xEA5E_A698,
        ),
        (
            0x03,
            "Fighting",
            60,
            3.0,
            false,
            [&[0], &[0], &[0], &[0]],
            0xCA63_A970,
            0xDA5E_A698,
        ),
        (
            0x04,
            "Narcotic Deals",
            80,
            2.0,
            false,
            [&[0], &[0], &[4, 2, 3], &[0]],
            0x0A63_9636,
            0xEA5E_A697,
        ),
        (
            0x05,
            "Arson",
            100,
            0.5,
            true,
            [&[0], &[2, 1], ALL_BUT_5, &[3]],
            0x2A63_95EF,
            0xCA5E_A698,
        ),
        (
            0x06,
            "Money Snatcher",
            60,
            3.0,
            false,
            [&[0], &[0], &[4, 2], &[0]],
            0x4A63_9681,
            0xAA5E_A698,
        ),
        (
            0x07,
            "Stickup",
            80,
            2.0,
            false,
            [&[0], &[0], &[3, 2], &[0]],
            0x2A63_9653,
            0x2A5E_A698,
        ),
        (
            0x08,
            "Car Theft",
            60,
            3.0,
            false,
            [&[0], &[0], ALL_BUT_5, &[0]],
            0xAA63_A98D,
            0x3A5E_A698,
        ),
        (
            0x09,
            "Solicitation",
            60,
            3.0,
            false,
            [&[0], &[0], &[4, 2, 3], &[0]],
            0x6A63_A9A4,
            0x4A5E_A698,
        ),
        (
            0x0A,
            "Mugging",
            80,
            2.0,
            false,
            [&[0], &[0], ALL_BUT_5, &[0]],
            0xAA63_966B,
            0x0A5E_A698,
        ),
        (
            0x0B,
            "Bank Robbery",
            80,
            2.0,
            false,
            [&[0], &[0], &[3, 2], &[0]],
            0xAA63_966B,
            0x0A5E_A698,
        ),
        (
            0x10,
            "Flashing",
            40,
            4.0,
            false,
            [&[0], &[0], ALL_BUT_5, &[0]],
            0x8A63_A9BA,
            0x5A5E_A698,
        ),
        (
            0x11,
            "Graffiti",
            40,
            4.0,
            false,
            [&[0], &[0], ALL_BUT_5, &[0]],
            0xCA63_A9D1,
            0x6A5E_A698,
        ),
        (
            0x12,
            "Pie Throwing",
            40,
            4.0,
            false,
            [&[0], &[0], ALL_BUT_5, &[0]],
            0x4A63_A9E4,
            0x7A5E_A698,
        ),
        (
            0x13,
            "TPing",
            40,
            4.0,
            false,
            [&[1], &[3, 2], &[1], &[0]],
            0x2A63_A9F9,
            0x8A5E_A698,
        ),
    ];
    let bytes = |b: &[u8]| Variant::array(mem::UINT8, b.iter().map(|&x| Elem::U8(x)).collect());
    let key = |t: u32, g: u32, i: u32| Variant::array(mem::UINT32, vec![Elem::U32(t), Elem::U32(g), Elem::U32(i)]);
    rows.iter()
        .map(
            |&(id, name, severity, weight, arson, [zone, wealth, purpose, condition], icon, name_key)| {
                let mut h = PropertyHolder::new();
                h.add(EXEMPLAR_NAME, Variant::string(name.as_bytes()));
                h.add(
                    prop::TYPE_NAME_KEY,
                    key(SHIPPED_NAME_TYPE, SHIPPED_NAME_GROUP, name_key),
                );
                h.add(prop::TYPE_ICON, key(SHIPPED_ICON_TYPE, SHIPPED_ICON_GROUP, icon));
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

    /// CQ13 (§6.10 table): raw storage, no conversion.
    #[test]
    fn crime_effect_raw_bytes_cq13() {
        let arr = |elem, items| raw_array_bytes(&Variant::array(elem, items)).unwrap();
        // Uint8 x2 (all six shipped buildings): m = 100, r = 8.
        assert_eq!(arr(mem::UINT8, vec![Elem::U8(100), Elem::U8(8)]), [100, 8]);
        assert_eq!(arr(mem::SINT8, vec![Elem::I8(-1), Elem::I8(8)]), [0xFF, 8]);
        // Wider arrays: bytes 0 and 1 of element 0, little-endian.
        let u32s = arr(mem::UINT32, vec![Elem::U32(100), Elem::U32(8)]);
        assert_eq!(u32s, [100, 0, 0, 0, 8, 0, 0, 0]);
        assert_eq!((u32s[0], u32s[1]), (100, 0));
        let f32s = arr(mem::FLOAT32, vec![Elem::F32(100.0), Elem::F32(8.0)]);
        assert_eq!(f32s[..4], 100.0f32.to_le_bytes());
        assert_eq!((f32s[0], f32s[1]), (0, 0));
        // One Uint16 element is already 2 bytes: m = low, r = high.
        assert_eq!(arr(mem::UINT16, vec![Elem::U16(0x0864)]), [0x64, 0x08]);
        assert_eq!(arr(mem::BOOL, vec![Elem::Bool(true), Elem::Bool(false)]), [1, 0]);
        assert_eq!(raw_array_bytes(&Variant::string(b"ab")), Some(b"ab".to_vec()));
        // CQ22: a string gives the codes of its first two bytes.
        let ab = raw_array_bytes(&Variant::string(b"AB")).unwrap();
        assert_eq!((ab[0], ab[1]), (65, 66));
        // Too short for {m, r}: the simulator skips it ([choice]), a
        // one-character string included (CQ22).
        assert_eq!(arr(mem::UINT8, vec![Elem::U8(100)]), [100]);
        assert!(arr(mem::UINT8, vec![]).is_empty());
        assert_eq!(raw_array_bytes(&Variant::string(b"A")), Some(vec![65]));
        // Scalars, objects and empty values: none ([choice]; CQ22 for an
        // object value, whose storage is an address in the original).
        assert_eq!(raw_array_bytes(&Variant::Scalar(Elem::U8(100))), None);
        assert_eq!(raw_array_bytes(&Variant::Scalar(Elem::U32(0x0864))), None);
        assert_eq!(raw_array_bytes(&Variant::Object(crate::stream::RefValue::Null)), None);
        let host = crate::stream::RefValue::Host {
            token: 0x1234,
            class_id: 7,
        };
        assert_eq!(raw_array_bytes(&Variant::Object(host)), None);
        assert_eq!(raw_array_bytes(&Variant::Empty), None);
    }

    fn shipped() -> Vec<CrimeType> {
        shipped_type_exemplars()
            .iter()
            .map(|(id, h)| CrimeType::from_exemplar(*id, h, &|_| LtextLoad::Missing))
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
        // CQ23 [choice]: one u32 (4 data bytes) is under 12 bytes: missing,
        // the default (0, 0, 0).
        h.add(prop::TYPE_ICON, Variant::array(mem::UINT32, vec![Elem::U32(7)]));
        let t = CrimeType::from_exemplar(9, &h, &|k| (k.instance == 3).then(|| b"ltext".to_vec()).into());
        assert_eq!(t.name, b"ltext");
        assert_eq!((t.severity, t.arson, t.weight), (64, false, 10.0));
        assert_eq!(
            t.icon,
            ResKey {
                type_id: 0,
                group: 0,
                instance: 0
            }
        );
        assert_eq!(
            t.zone_mask & t.wealth_mask & t.purpose_mask & t.condition_mask,
            ANY_BITS
        );
        let t = CrimeType::from_exemplar(9, &h, &|_| LtextLoad::Missing);
        assert_eq!(t.name, b"fallback");
        let t = CrimeType::from_exemplar(9, &PropertyHolder::new(), &|_| LtextLoad::Missing);
        assert!(t.name.is_empty());
    }

    /// CQ8: the §7 name load, steps 1-4 and the fallback.
    #[test]
    fn name_load_cq8() {
        use std::cell::RefCell;
        let key3 = |a: u32, b: u32, c: u32| Variant::array(mem::UINT32, vec![Elem::U32(a), Elem::U32(b), Elem::U32(c)]);
        // Step 1: value [0] is ignored, the type is always 0x2026960B.
        let mut h = PropertyHolder::new();
        h.add(prop::TYPE_NAME_KEY, key3(0x1234_5678, 0x0A55_4AE0, 0xEA5E_A698));
        let asked = RefCell::new(Vec::new());
        let t = CrimeType::from_exemplar(1, &h, &|k| {
            asked.borrow_mut().push(k);
            LtextLoad::Loaded(b"x".to_vec())
        });
        let shipped_key = ResKey {
            type_id: NAME_LTEXT_TYPE,
            group: 0x0A55_4AE0,
            instance: 0xEA5E_A698,
        };
        assert_eq!(*asked.borrow(), vec![shipped_key]);
        assert_eq!(t.name, b"x");
        // Group and instance 0 are looked up like any other key.
        let mut zero = PropertyHolder::new();
        zero.add(prop::TYPE_NAME_KEY, key3(0, 0, 0));
        let t = CrimeType::from_exemplar(1, &zero, &|k| {
            (k.group == 0 && k.instance == 0).then(|| b"zero".to_vec()).into()
        });
        assert_eq!(t.name, b"zero");
        // Step 4: U+2022 -> U+200B, on a successful load only.
        let bullets = "a\u{2022}b\u{2022}\u{2022}".as_bytes().to_vec();
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::Loaded(bullets.clone()));
        assert_eq!(t.name, "a\u{200B}b\u{200B}\u{200B}".as_bytes());
        // CQ16: the detokenizer returned false: the load fails and the
        // copied text is discarded, so with no 0xAA12FCB0 the name is empty.
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::DetokenizeFailed);
        assert!(t.name.is_empty());
        // With 0xAA12FCB0 the fallback replaces it.
        h.add(prop::TYPE_NAME, Variant::string(b"inline"));
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::DetokenizeFailed);
        assert_eq!(t.name, b"inline");
        // The inline string is read only when the load fails.
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::Loaded(b"ltext".to_vec()));
        assert_eq!(t.name, b"ltext");
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::Missing);
        assert_eq!(t.name, b"inline");
        // [choice] Fewer than 3 values: no key, no lookup, the fallback.
        let mut short = PropertyHolder::new();
        short.add(
            prop::TYPE_NAME_KEY,
            Variant::array(mem::UINT32, vec![Elem::U32(1), Elem::U32(2)]),
        );
        short.add(prop::TYPE_NAME, Variant::string(b"inline"));
        assert_eq!(name_ltext_key(&short), None);
        let t = CrimeType::from_exemplar(1, &short, &|_| panic!("no lookup without a key"));
        assert_eq!(t.name, b"inline");
        // CQ17 [choice]: one value or a scalar counts as missing too, and
        // with no 0xAA12FCB0 the name stays empty.
        for key in [
            Variant::array(mem::UINT32, vec![Elem::U32(1)]),
            Variant::Scalar(Elem::U32(0x0A55_4AE0)),
        ] {
            let mut short = PropertyHolder::new();
            short.add(prop::TYPE_NAME_KEY, key);
            assert_eq!(name_ltext_key(&short), None);
            let t = CrimeType::from_exemplar(1, &short, &|_| panic!("no lookup without a key"));
            assert!(t.name.is_empty());
        }
        // CQ23: other value types are not converted: the first 12 data bytes
        // as three little-endian u32 (§7 "Other value types" worked examples).
        let named = |key: Variant| {
            let mut h = PropertyHolder::new();
            h.add(prop::TYPE_NAME_KEY, key);
            name_ltext_key(&h).map(|k| (k.type_id, k.group, k.instance))
        };
        let u16s = [0x960B, 0x2026, 0x4AE0, 0x0A55, 0xA698, 0xEA5E];
        assert_eq!(
            named(Variant::array(
                mem::UINT16,
                u16s.iter().map(|&x| Elem::U16(x)).collect()
            )),
            Some((NAME_LTEXT_TYPE, 0x0A55_4AE0, 0xEA5E_A698)),
            "the shipped Generic key as u16"
        );
        assert_eq!(
            named(Variant::array(
                mem::FLOAT32,
                vec![Elem::F32(0.0), Elem::F32(1.0), Elem::F32(2.0)]
            )),
            Some((NAME_LTEXT_TYPE, 0x3F80_0000, 0x4000_0000))
        );
        assert_eq!(
            named(Variant::string(b"ABCDEFGHIJKL")),
            Some((NAME_LTEXT_TYPE, 0x4847_4645, 0x4C4B_4A49))
        );
        assert_eq!(
            named(Variant::array(
                mem::SINT64,
                vec![Elem::I64(0x1111_2222_3333_4444), Elem::I64(0x5555_6666_7777_8888)]
            )),
            Some((NAME_LTEXT_TYPE, 0x1111_2222, 0x7777_8888)),
            "64-bit: the high half of e[0], the low half of e[1]"
        );
        // [choice] Under 12 data bytes: missing, no lookup (a u8 [a, b, c] is
        // 3 bytes, a string of 11 bytes, a u16 array of 5).
        assert_eq!(
            named(Variant::array(mem::UINT8, vec![Elem::U8(1), Elem::U8(2), Elem::U8(3)])),
            None
        );
        assert_eq!(named(Variant::string(b"ABCDEFGHIJK")), None);
        assert_eq!(
            named(Variant::array(
                mem::UINT16,
                u16s[..5].iter().map(|&x| Elem::U16(x)).collect()
            )),
            None
        );
        // The icon key through the same reader: bytes 0-3 are its type.
        let mut h = PropertyHolder::new();
        h.add(
            prop::TYPE_ICON,
            Variant::array(mem::UINT16, [1, 0, 2, 0, 3, 0].iter().map(|&x| Elem::U16(x)).collect()),
        );
        let t = CrimeType::from_exemplar(1, &h, &|_| LtextLoad::Missing);
        assert_eq!((t.icon.type_id, t.icon.group, t.icon.instance), (1, 2, 3));
        // Other bytes stay, a cut-off sequence too.
        let mut v = vec![0xE2, 0x80, 0xA2, 0xE2, 0x80];
        bullets_to_zero_width_spaces(&mut v);
        assert_eq!(v, [0xE2, 0x80, 0x8B, 0xE2, 0x80]);
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
        // Shipped keys (§7): no LTEXT → empty names; Bank Robbery shares
        // Mugging's icon.
        let flashing = types.iter().find(|t| t.id == 0x10).unwrap();
        assert!(flashing.name.is_empty());
        assert_eq!(
            flashing.icon,
            ResKey {
                type_id: SHIPPED_ICON_TYPE,
                group: SHIPPED_ICON_GROUP,
                instance: 0x8A63_A9BA
            }
        );
        let icon = |id: u32| types.iter().find(|t| t.id == id).unwrap().icon;
        assert_eq!(icon(0x0B), icon(0x0A));
        let named = shipped_type_exemplars()
            .iter()
            .map(|(id, h)| {
                CrimeType::from_exemplar(*id, h, &|k| {
                    (k == ResKey {
                        type_id: SHIPPED_NAME_TYPE,
                        group: SHIPPED_NAME_GROUP,
                        instance: 0xEA5E_A698,
                    })
                    .then(|| b"text".to_vec())
                    .into()
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(named[0].name, b"text", "Generic");
        assert!(named[1].name.is_empty(), "Fighting");
        // No building: wealth 0x80000000, purpose 0x100.
        let none = LotBits::new(1, None, 0);
        assert_eq!((none.wealth, none.purpose), (NONE_BIT, NO_PURPOSE_BIT));
    }
}
