// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Typed reads of exemplar properties held as [`PropertyHolder`]s, plus small
//! numeric helpers shared by the scalar simulators (demand, ordinance,
//! weather).
//!
//! Exemplars reach the core as plain data: the host (the GZCOM shim, or a
//! standalone resource loader) converts a game exemplar into a
//! [`PropertyHolder`] of variants. The readers here convert between numeric
//! element types the way the simulators' property readers do: integers to
//! floats by value, floats to integers by truncation toward zero.
//!
//! The vector readers ([`u32s`], [`i64s`], [`f32s`]) convert **every element**
//! and give a scalar's one value. The original's vector helpers (data-formats
//! §5.2 "The vector helpers", EX33) convert nothing: they copy `GetCount()`
//! elements of their own width from the raw data, so a Float32 `{2.75}` read
//! as `u32` gives `{0x40300000}`, a narrower stored type is over-read and a
//! scalar (count 0) gives an empty vector. *[choice]* (EX33, kept) Converting is
//! not observable on shipped data: the only property read with another type,
//! `0xEA0232A0` (Sint32 read as `vec<u32>`), has the same bits either way, and
//! converting replaces the over-read with defined values.
//!
//! EX39 (data-formats §5.2 "Shipped scalars that only a vector helper reads"):
//! the original's vector helpers read a scalar as an **empty** vector; 172
//! shipped lot configurations store `0x88EDC796` (161 of them `0x88EDC795`, 10
//! also `0x88EDC793`) as a Uint8 scalar 0 and two landfill tiles store the
//! budget cost `0xEA54D286` as a Sint64 scalar. *[choice, a spec question]*
//! These readers still give a scalar its one value, because a core holder
//! cannot tell a scalar from a one-element array: the C ABI setters
//! (`fc4_props_set_u32s` and the like, which the glue's `ToCore` uses after a
//! converting read) store any single value as [`Variant::Scalar`], so reading
//! a scalar as empty here would empty every one-element array that reaches the
//! core that way. What EX39 changes is applied where the type is known: the
//! budget item arrays read through the budget host's `property_is_array`
//! (`budget::items`). A 2026-10-15 sweep of the shipped exemplars and cohorts
//! found no property read by these vector readers stored as a scalar, so on
//! shipped data no value differs; no core reader reads `0x88EDC793`,
//! `0x88EDC795` or `0x88EDC796` yet. A reader added for them must treat a
//! scalar as empty itself (it should ask for the variant type, as the budget
//! items do).

use crate::property::PropertyHolder;
use crate::stream::ResKey;
use crate::variant::{Elem, Variant};

/// The exemplar resource type (`0x6534284A`).
pub const EXEMPLAR_TYPE: u32 = 0x6534_284A;
/// Property `0x10` (ExemplarType).
pub const PROP_EXEMPLAR_TYPE: u32 = 0x0000_0010;

/// Numeric elements of a property (empty for strings, objects and empty
/// values).
fn numeric(v: &Variant) -> Option<Vec<Elem>> {
    match v {
        Variant::Scalar(e) => Some(vec![*e]),
        Variant::Array { elem, items } if *elem != crate::variant::mem::RZ_CHAR && *elem != 0 => Some(items.clone()),
        _ => None,
    }
}

/// A curve from a property value (foundations §4.1). `None` (absent) for a
/// non-numeric value and for a single value: a scalar or a one-element array
/// gives zero points in the original, whose evaluator then crashes; FreeCity4
/// treats it as absent *[choice]*. Otherwise the curve of its points (empty for
/// an empty array; an odd trailing value is ignored).
pub fn curve_of_variant(v: &Variant) -> Option<crate::curve::ResponseCurve> {
    let values = numeric(v)?;
    if values.len() == 1 {
        return None;
    }
    let values: Vec<f32> = values
        .iter()
        .map(|e| match *e {
            Elem::F32(x) => x,
            other => other.as_f64() as f32,
        })
        .collect();
    Some(crate::curve::ResponseCurve::from_flat(&values).unwrap_or_default())
}

/// [`curve_of_variant`] of a property; `None` when it is absent.
pub fn curve(h: &PropertyHolder, id: u32) -> Option<crate::curve::ResponseCurve> {
    curve_of_variant(h.get(id)?)
}

/// All values of a property as `u32` (integers wrap to 32 bits, floats are
/// truncated toward zero; every element converted, EX33 *[choice]*, see the
/// module documentation).
pub fn u32s(h: &PropertyHolder, id: u32) -> Option<Vec<u32>> {
    numeric(h.get(id)?).map(|v| v.iter().map(|e| e.as_i64() as u32).collect())
}

/// All values of a property as `i64` (floats truncated toward zero).
pub fn i64s(h: &PropertyHolder, id: u32) -> Option<Vec<i64>> {
    numeric(h.get(id)?).map(|v| v.iter().map(Elem::as_i64).collect())
}

/// All values of a property as `f32`.
pub fn f32s(h: &PropertyHolder, id: u32) -> Option<Vec<f32>> {
    numeric(h.get(id)?).map(|v| {
        v.iter()
            .map(|e| match *e {
                Elem::F32(x) => x,
                other => other.as_f64() as f32,
            })
            .collect()
    })
}

/// The first value of a property as `u32`.
pub fn u32_of(h: &PropertyHolder, id: u32) -> Option<u32> {
    u32s(h, id).and_then(|v| v.first().copied())
}

/// The first value of a property as `i64`.
pub fn i64_of(h: &PropertyHolder, id: u32) -> Option<i64> {
    i64s(h, id).and_then(|v| v.first().copied())
}

/// The first value of a property as `f32`.
pub fn f32_of(h: &PropertyHolder, id: u32) -> Option<f32> {
    f32s(h, id).and_then(|v| v.first().copied())
}

/// A boolean property: any non-zero first value is true.
pub fn bool_of(h: &PropertyHolder, id: u32) -> Option<bool> {
    i64_of(h, id).map(|v| v != 0)
}

/// The bytes of a string property.
pub fn string_of(h: &PropertyHolder, id: u32) -> Option<Vec<u8>> {
    h.get(id)?.bytes()
}

/// `u32` pairs (`a, b, a, b, …`); an odd trailing value is ignored.
pub fn u32_pairs(h: &PropertyHolder, id: u32) -> Vec<(u32, u32)> {
    u32s(h, id)
        .map(|v| v.chunks_exact(2).map(|c| (c[0], c[1])).collect())
        .unwrap_or_default()
}

/// Whether the holder is an exemplar of the given ExemplarType (property `0x10`).
pub fn has_exemplar_type(h: &PropertyHolder, exemplar_type: u32) -> bool {
    u32_of(h, PROP_EXEMPLAR_TYPE) == Some(exemplar_type)
}

/// An exemplar key in the game's (type, group, instance) order.
pub const fn exemplar_key(group: u32, instance: u32) -> ResKey {
    ResKey {
        type_id: EXEMPLAR_TYPE,
        group,
        instance,
    }
}

/// Round to the nearest integer, ties to even, in `f32` (the spec's
/// `roundHalfEven`). NaN stays NaN; infinities stay infinite.
pub fn round_half_even_f32(x: f32) -> f32 {
    let r = x.round(); // ties away from zero
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        r
    }
}

/// Round to the nearest integer, ties to even, in `f64`.
pub fn round_half_even_f64(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::mem;

    #[test]
    fn conversions() {
        let mut h = PropertyHolder::new();
        h.add(1, Variant::Scalar(Elem::I32(-5)));
        h.add(2, Variant::array(mem::FLOAT32, vec![Elem::F32(1.5), Elem::F32(-2.7)]));
        h.add(3, Variant::string(b"abc"));
        h.add(
            4,
            Variant::array(mem::UINT32, vec![Elem::U32(7), Elem::U32(8), Elem::U32(9)]),
        );
        assert_eq!(i64_of(&h, 1), Some(-5));
        assert_eq!(u32_of(&h, 1), Some(0xFFFF_FFFB));
        assert_eq!(f32_of(&h, 1), Some(-5.0));
        assert_eq!(i64s(&h, 2), Some(vec![1, -2]));
        assert_eq!(f32s(&h, 2), Some(vec![1.5, -2.7]));
        assert_eq!(u32s(&h, 3), None);
        assert_eq!(string_of(&h, 3), Some(b"abc".to_vec()));
        assert_eq!(u32_pairs(&h, 4), vec![(7, 8)]);
        assert_eq!(u32_of(&h, 99), None);
    }

    #[test]
    fn single_value_curves_are_absent() {
        // foundations §4.1: a scalar or a one-element array gives zero points
        // in the original (its evaluator then crashes); FreeCity4 treats it
        // as absent [choice]. An empty array is a present, empty curve.
        let mut h = PropertyHolder::new();
        h.add(1, Variant::Scalar(Elem::F32(5.0)));
        h.add(2, Variant::array(mem::FLOAT32, vec![Elem::F32(5.0)]));
        h.add(3, Variant::array(mem::FLOAT32, vec![]));
        h.add(
            4,
            Variant::array(mem::SINT32, vec![Elem::I32(0), Elem::I32(2), Elem::I32(10)]),
        );
        h.add(5, Variant::string(b"0 1"));
        assert_eq!(curve(&h, 1), None);
        assert_eq!(curve(&h, 2), None);
        assert!(curve(&h, 3).is_some_and(|c| c.is_empty()));
        assert_eq!(curve(&h, 4).map(|c| c.eval(7.0)), Some(2.0));
        assert_eq!(curve(&h, 5), None);
        assert_eq!(curve(&h, 99), None);
    }

    #[test]
    fn rounding() {
        for (x, want) in [
            (0.5f32, 0.0f32),
            (1.5, 2.0),
            (2.5, 2.0),
            (-0.5, -0.0),
            (-1.5, -2.0),
            (1.4, 1.0),
            (1.6, 2.0),
        ] {
            assert_eq!(round_half_even_f32(x), want, "{x}");
            assert_eq!(round_half_even_f64(f64::from(x)), f64::from(want), "{x}");
        }
        assert!(round_half_even_f32(f32::NAN).is_nan());
    }

    #[test]
    fn vector_readers_convert_every_element() {
        // data-formats §5.2 (EX33): the original copies raw bits (a Float32
        // {2.75} as `vec<u32>` gives {0x40300000}) and gives a scalar an empty
        // vector; FreeCity4 converts each element [choice].
        let mut h = PropertyHolder::new();
        h.add(1, Variant::array(mem::FLOAT32, vec![Elem::F32(2.75)]));
        h.add(2, Variant::array(mem::SINT32, vec![Elem::I32(-1), Elem::I32(5)]));
        h.add(3, Variant::Scalar(Elem::U32(7)));
        assert_eq!(u32s(&h, 1), Some(vec![2]));
        // Sint32 read as u32 (as `0xEA0232A0` is): the same bits as a copy.
        assert_eq!(u32s(&h, 2), Some(vec![0xFFFF_FFFF, 5]));
        assert_eq!(f32s(&h, 2), Some(vec![-1.0, 5.0]));
        // EX39 [choice]: a scalar keeps its one value here (the original's
        // helpers give it an empty vector); see the module documentation for
        // why, and budget::items for where EX39 is applied.
        assert_eq!(u32s(&h, 3), Some(vec![7]));
    }
}
