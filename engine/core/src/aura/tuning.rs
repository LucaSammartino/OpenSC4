// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The aura tuning (§10): code defaults, the shipped exemplar and the
//! property reader with the "delete" curve pattern (foundations §4.1).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};

/// Tuning property IDs (§10).
pub mod prop {
    pub const PARK_CURVE: u32 = 0xEA02_3270;
    /// Air, water, garbage, radiation.
    pub const POLLUTION_CURVES: [u32; 4] = [0xEA02_3271, 0xEA02_3272, 0xEA02_3273, 0xEA02_3274];
    pub const SCHOOL_CURVE: u32 = 0xEA02_3275;
    pub const HOSPITAL_CURVE: u32 = 0xEA02_3276;
    pub const CRIME_CURVE: u32 = 0xEA02_3277;
    pub const TRIP_CURVE: u32 = 0xEA02_3278;
    pub const CONGESTION_CURVE: u32 = 0xEA02_3279;
    pub const LONG_DECAY: u32 = 0xEA02_327A;
    pub const SHORT_DECAY: u32 = 0xEA02_327B;
    /// R$, R$$, R$$$.
    pub const BASE_AURA: [u32; 3] = [0xEA02_327C, 0xEA02_327D, 0xEA02_327E];
    pub const TAX_CHANGE_CURVE: u32 = 0xEA02_327F;
    pub const VALUE_WEALTH_CURVE: u32 = 0xEA02_3280;
    /// Fire, flora, civic plop, network plop.
    pub const TRANSIENT: [u32; 4] = [0xEA02_3290, 0xEA02_3291, 0xEA02_3292, 0xEA02_3293];
    pub const HOTSPOT: u32 = 0xEA02_32A0;
    pub const RIOT_CURVE: u32 = 0xEA02_32A1;
}

/// Code defaults (§10).
pub const DEFAULT_LONG_DECAY: f32 = 0.6;
pub const DEFAULT_SHORT_DECAY: f32 = 0.4;
/// {hotspotThreshold, hotspotRadius, hotspotMinWeight} unless the property
/// has exactly three values.
pub const DEFAULT_HOTSPOT: [i32; 3] = [196, 8, 16];

/// The tuning fields of §3 (constructor values by `Default`).
#[derive(Clone, Debug, PartialEq)]
pub struct AuraTuning {
    pub long_decay: f32,
    pub short_decay: f32,
    /// R$, R$$, R$$$.
    pub base_aura: [i32; 3],
    /// {threshold, radius, min weight}.
    pub hotspot: [i32; 3],
    /// Per transient type (fire, flora, civic, network).
    pub transient_magnitude: [f32; 4],
    pub transient_radius: [f32; 4],
    pub park: Option<ResponseCurve>,
    /// Air, water, garbage, radiation.
    pub pollution: [Option<ResponseCurve>; 4],
    pub school: Option<ResponseCurve>,
    pub hospital: Option<ResponseCurve>,
    pub crime: Option<ResponseCurve>,
    pub trip: Option<ResponseCurve>,
    pub congestion: Option<ResponseCurve>,
    pub tax_change: Option<ResponseCurve>,
    pub value_wealth: Option<ResponseCurve>,
    pub riot: Option<ResponseCurve>,
}

impl Default for AuraTuning {
    /// The constructor values (§3): decays 0.6 / 0.4, everything else 0,
    /// no curves.
    fn default() -> Self {
        AuraTuning {
            long_decay: DEFAULT_LONG_DECAY,
            short_decay: DEFAULT_SHORT_DECAY,
            base_aura: [0; 3],
            hotspot: [0; 3],
            transient_magnitude: [0.0; 4],
            transient_radius: [0.0; 4],
            park: None,
            pollution: [None, None, None, None],
            school: None,
            hospital: None,
            crime: None,
            trip: None,
            congestion: None,
            tax_change: None,
            value_wealth: None,
            riot: None,
        }
    }
}

/// A curve property under the "delete" pattern (§10): `None` when absent,
/// and also ([choice], §6, §10) for a property with fewer than two values
/// (a scalar, a one-element or an empty array: the original builds a curve
/// with no points and evaluates it, crashing or reading a stale value) and
/// for a non-numeric one ([`props::curve`], foundations §4.1). Otherwise a
/// new curve from its points. Reloading in place (the twelve stage curves)
/// and delete-then-create (the riot curve) give the same result here.
fn curve(h: &PropertyHolder, id: u32) -> Option<ResponseCurve> {
    props::curve(h, id).filter(|c| !c.is_empty())
}

/// One transient type's {magnitude, radius} (§10 "Transient tuning in
/// detail"). The original copies the variant's raw 32-bit words and reads
/// `v[0]`, `v[1]` unchecked (a crash for a scalar or an empty array, garbage
/// past a one-value array, integer bits taken as floats). [choice] Only a
/// Float32 array of at least 2 values is used (extra values ignored: the raw
/// copy reads the same numbers); anything else keeps {0, 0}, as if the
/// property were missing.
fn transient_pair(v: Option<&Variant>) -> (f32, f32) {
    let Some(Variant::Array { elem, items }) = v else {
        return (0.0, 0.0);
    };
    match items.as_slice() {
        [Elem::F32(magnitude), Elem::F32(radius), ..] if *elem == mem::FLOAT32 => (*magnitude, *radius),
        _ => (0.0, 0.0),
    }
}

impl AuraTuning {
    /// Deletes every curve (§4.2).
    pub fn delete_curves(&mut self) {
        self.park = None;
        self.pollution = [None, None, None, None];
        self.school = None;
        self.hospital = None;
        self.crime = None;
        self.trip = None;
        self.congestion = None;
        self.tax_change = None;
        self.value_wealth = None;
        self.riot = None;
    }

    /// Reads the exemplar (§10) over the current values: the decays and the
    /// transient arrays are reset to their code defaults first, every curve
    /// is deleted and recreated only when its property is present, the base
    /// auras are kept when absent, and the hotspot triple is the property
    /// when it has exactly three values, else {196, 8, 16}.
    pub fn read(&mut self, h: &PropertyHolder) {
        self.long_decay = props::f32_of(h, prop::LONG_DECAY).unwrap_or(DEFAULT_LONG_DECAY);
        self.short_decay = props::f32_of(h, prop::SHORT_DECAY).unwrap_or(DEFAULT_SHORT_DECAY);
        for (slot, &id) in self.base_aura.iter_mut().zip(prop::BASE_AURA.iter()) {
            if let Some(v) = props::i64_of(h, id) {
                *slot = v as i32;
            }
        }
        self.park = curve(h, prop::PARK_CURVE);
        for (slot, &id) in self.pollution.iter_mut().zip(prop::POLLUTION_CURVES.iter()) {
            *slot = curve(h, id);
        }
        self.school = curve(h, prop::SCHOOL_CURVE);
        self.hospital = curve(h, prop::HOSPITAL_CURVE);
        self.crime = curve(h, prop::CRIME_CURVE);
        self.trip = curve(h, prop::TRIP_CURVE);
        self.congestion = curve(h, prop::CONGESTION_CURVE);
        self.tax_change = curve(h, prop::TAX_CHANGE_CURVE);
        self.value_wealth = curve(h, prop::VALUE_WEALTH_CURVE);
        for (i, &id) in prop::TRANSIENT.iter().enumerate() {
            let (magnitude, radius) = transient_pair(h.get(id));
            self.transient_magnitude[i] = magnitude;
            self.transient_radius[i] = radius;
        }
        self.hotspot = match props::i64s(h, prop::HOTSPOT) {
            Some(v) if v.len() == 3 => [v[0] as i32, v[1] as i32, v[2] as i32],
            _ => DEFAULT_HOTSPOT,
        };
        self.riot = curve(h, prop::RIOT_CURVE);
    }
}

/// The shipped "Aura Simulator" exemplar (§10, `SimCity_1.dat`), for hosts
/// and tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let curve = |pts: &[(f32, f32)]| {
        Variant::array(
            mem::FLOAT32,
            pts.iter().flat_map(|&(x, y)| [Elem::F32(x), Elem::F32(y)]).collect(),
        )
    };
    let s32s = |v: &[i32]| Variant::array(mem::SINT32, v.iter().map(|&x| Elem::I32(x)).collect());
    let pollution = [
        (0.0, 10.0),
        (200.0, 7.0),
        (400.0, 0.0),
        (600.0, -10.0),
        (800.0, -20.0),
        (1024.0, -35.0),
    ];
    h.add(prop::PARK_CURVE, curve(&[(0.0, 0.0), (50.0, 2.0), (100.0, 3.0)]));
    h.add(prop::POLLUTION_CURVES[0], curve(&pollution));
    h.add(prop::POLLUTION_CURVES[1], curve(&pollution));
    h.add(
        prop::POLLUTION_CURVES[2],
        curve(&[
            (0.0, 10.0),
            (200.0, 7.0),
            (400.0, 3.0),
            (1800.0, -4.0),
            (2400.0, -16.0),
            (4096.0, -35.0),
        ]),
    );
    h.add(prop::POLLUTION_CURVES[3], curve(&[(0.0, 0.0), (1.0, -50.0)]));
    h.add(prop::SCHOOL_CURVE, curve(&[(0.0, -7.0), (1.0, 7.0)]));
    h.add(prop::HOSPITAL_CURVE, curve(&[(0.0, -7.0), (1.0, 7.0)]));
    h.add(
        prop::CRIME_CURVE,
        curve(&[
            (0.0, 10.0),
            (40.0, 5.0),
            (60.0, 2.0),
            (80.0, -3.0),
            (100.0, -5.0),
            (255.0, -50.0),
        ]),
    );
    h.add(prop::TRIP_CURVE, curve(&[(0.0, 7.0), (255.0, -7.0)]));
    h.add(prop::CONGESTION_CURVE, curve(&[(0.0, 7.0), (255.0, -7.0)]));
    h.add(prop::LONG_DECAY, Variant::Scalar(Elem::F32(0.9)));
    h.add(prop::SHORT_DECAY, Variant::Scalar(Elem::F32(0.75)));
    for id in prop::BASE_AURA {
        h.add(id, Variant::Scalar(Elem::I32(0)));
    }
    h.add(
        prop::TAX_CHANGE_CURVE,
        curve(&[
            (-20.0, 25.0),
            (-10.0, 20.0),
            (-5.0, 10.0),
            (-2.0, 7.0),
            (-1.0, 5.0),
            (1.0, -5.0),
            (2.0, -7.0),
            (5.0, -10.0),
            (10.0, -20.0),
            (20.0, -25.0),
        ]),
    );
    h.add(
        prop::VALUE_WEALTH_CURVE,
        curve(&[(-2.0, 5.0), (-1.0, 3.0), (0.0, 0.0), (1.0, -3.0), (2.0, -5.0)]),
    );
    // Float32 arrays of 2, as shipped (§10).
    let f32s = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
    h.add(prop::TRANSIENT[0], f32s(&[-20.0, 128.0]));
    h.add(prop::TRANSIENT[1], f32s(&[3.0, 32.0]));
    h.add(prop::TRANSIENT[2], f32s(&[25.0, 128.0]));
    h.add(prop::TRANSIENT[3], f32s(&[0.0, 0.0]));
    h.add(prop::HOTSPOT, s32s(&[-50, 3, 4]));
    h.add(
        prop::RIOT_CURVE,
        curve(&[(0.0, 0.0), (5.0, 10.0), (10.0, 60.0), (20.0, 100.0)]),
    );
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_values() {
        let mut t = AuraTuning::default();
        t.read(&shipped_tuning());
        assert_eq!((t.long_decay, t.short_decay), (0.9, 0.75));
        assert_eq!(t.hotspot, [-50, 3, 4]);
        assert_eq!(t.transient_magnitude, [-20.0, 3.0, 25.0, 0.0]);
        assert_eq!(t.transient_radius, [128.0, 32.0, 128.0, 0.0]);
        assert_eq!(t.tax_change.as_ref().unwrap().eval(1.0), -5.0);
        assert_eq!(t.riot.as_ref().unwrap().eval(4.0), 8.0);
        assert!(t.pollution.iter().all(Option::is_some));
    }

    #[test]
    fn delete_pattern_and_defaults() {
        let mut t = AuraTuning::default();
        t.read(&shipped_tuning());
        t.base_aura = [3, 4, 5];
        let mut h = PropertyHolder::new();
        h.add(
            prop::HOTSPOT,
            Variant::array(mem::SINT32, vec![Elem::I32(1), Elem::I32(2)]),
        );
        h.add(prop::PARK_CURVE, Variant::array(mem::FLOAT32, vec![]));
        h.add(prop::CRIME_CURVE, Variant::Scalar(Elem::F32(3.0)));
        h.add(prop::TRANSIENT[0], Variant::array(mem::FLOAT32, vec![Elem::F32(9.0)]));
        t.read(&h);
        assert_eq!((t.long_decay, t.short_decay), (0.6, 0.4));
        assert_eq!(t.base_aura, [3, 4, 5], "kept when absent");
        assert_eq!(t.hotspot, DEFAULT_HOTSPOT);
        assert!(t.park.is_none(), "an empty curve is absent [choice]");
        assert!(t.crime.is_none() && t.riot.is_none() && t.tax_change.is_none());
        assert_eq!(
            (t.transient_magnitude[0], t.transient_radius[0]),
            (0.0, 0.0),
            "one Float32 value: {{0, 0}} [choice] (AU2)"
        );
        assert_eq!(t.transient_magnitude[1], 0.0, "reset before each read");
    }

    /// §10 "Transient tuning in detail" (AU2): only a Float32 array of at
    /// least 2 values is used; every other shape keeps {0, 0}.
    #[test]
    fn transient_tuning_shapes() {
        let f = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
        let cases: [(Variant, (f32, f32)); 7] = [
            (f(&[-20.0, 128.0]), (-20.0, 128.0)),
            (f(&[3.0, 32.0, 99.0]), (3.0, 32.0)),
            (f(&[9.0]), (0.0, 0.0)),
            (f(&[]), (0.0, 0.0)),
            (Variant::Scalar(Elem::F32(5.0)), (0.0, 0.0)),
            (
                Variant::array(mem::SINT32, vec![Elem::I32(-20), Elem::I32(128)]),
                (0.0, 0.0),
            ),
            (
                Variant::array(mem::UINT32, vec![Elem::U32(25), Elem::U32(128)]),
                (0.0, 0.0),
            ),
        ];
        for (v, want) in cases {
            let mut h = PropertyHolder::new();
            h.add(prop::TRANSIENT[2], v.clone());
            let mut t = AuraTuning::default();
            t.read(&shipped_tuning());
            t.read(&h);
            assert_eq!((t.transient_magnitude[2], t.transient_radius[2]), want, "{v:?}");
        }
    }
}
