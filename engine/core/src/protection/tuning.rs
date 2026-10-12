// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tuning of §6.7: code defaults, the shipped exemplars and the
//! "default" curve pattern (foundations §4.1).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};

use super::algo::pct;
use super::{prop, ProtectionKind};

/// The code default of the funding curves (§6.7).
const FUNDING_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (100.0, 100.0), (120.0, 110.0)];
/// The code default of the strike and jailbreak curves (§6.7, quirk 11).
const RISING_DEFAULT: &[(f32, f32)] = &[(0.0, 20.0), (100.0, 100.0)];

/// The tuning in effect (§6.7). Police-only fields keep their code defaults
/// in a fire simulator (they are not read there); `no_road_multiplier` is
/// fire only.
#[derive(Clone, Debug, PartialEq)]
pub struct ProtectionTuning {
    /// Funding → efficiency (fire `0x29233E8D`, police `0x68DDAE93`).
    pub funding_curve: ResponseCurve,
    /// Strike response (fire `0x29233E98`, police `0x68DDAE9E`).
    pub strike_curve: ResponseCurve,
    pub max_funding: f32,
    pub strike_efficiency: f32,
    pub min_funding_to_end: f32,
    pub max_strike_duration: i32,
    pub no_road_multiplier: f32,
    /// Police: jail funding → efficiency (`0x68DDAEA0`).
    pub jail_curve: ResponseCurve,
    /// Police: jailbreak response (`0xEA614F63`).
    pub jailbreak_curve: ResponseCurve,
    pub rehab_rate: f32,
    pub arrest_threshold: i32,
    /// Escapee fraction range (lo, hi).
    pub escapee_range: (f32, f32),
    pub end_jailbreak_capacity: f32,
    pub overflow_factor: f32,
}

/// A curve property (foundations §4.1, [`props::curve`]): `None` when
/// absent, not numeric or a single value ([choice]); otherwise its points
/// (an empty curve when it has no complete point).
fn curve_prop(h: &PropertyHolder, id: u32) -> Option<ResponseCurve> {
    props::curve(h, id)
}

impl ProtectionTuning {
    /// The code defaults of `kind` (§6.7).
    pub fn code_defaults(kind: ProtectionKind) -> ProtectionTuning {
        let funding = || ResponseCurve::from_points(FUNDING_DEFAULT);
        let rising = || ResponseCurve::from_points(RISING_DEFAULT);
        ProtectionTuning {
            funding_curve: funding(),
            strike_curve: rising(),
            max_funding: 1.2,
            strike_efficiency: 0.5,
            min_funding_to_end: 0.9,
            max_strike_duration: match kind {
                ProtectionKind::Fire => 60,
                ProtectionKind::Police => 10,
            },
            no_road_multiplier: 0.8,
            jail_curve: funding(),
            jailbreak_curve: rising(),
            rehab_rate: 0.01,
            arrest_threshold: 16,
            escapee_range: (1.0, 5.0),
            end_jailbreak_capacity: 0.5,
            overflow_factor: 1.1,
        }
    }

    /// Reads the tuning (§6.7): every value starts at its code default
    /// (curves rebuilt from their point lists) and a present property
    /// replaces it; a missing exemplar keeps every default.
    pub fn read(kind: ProtectionKind, exemplar: Option<&PropertyHolder>) -> ProtectionTuning {
        let mut t = ProtectionTuning::code_defaults(kind);
        let Some(h) = exemplar else { return t };
        let load = |curve: &mut ResponseCurve, id: u32| {
            if let Some(c) = curve_prop(h, id) {
                *curve = c;
            }
        };
        let get_pct = |v: &mut f32, id: u32| {
            if let Some(x) = props::f32_of(h, id) {
                *v = pct(x);
            }
        };
        let get_f32 = |v: &mut f32, id: u32| {
            if let Some(x) = props::f32_of(h, id) {
                *v = x;
            }
        };
        let get_s32 = |v: &mut i32, id: u32| {
            if let Some(x) = props::i64_of(h, id) {
                *v = x as i32;
            }
        };
        match kind {
            ProtectionKind::Fire => {
                load(&mut t.funding_curve, prop::FIRE_FUNDING_CURVE);
                load(&mut t.strike_curve, prop::FIRE_STRIKE_CURVE);
                get_pct(&mut t.max_funding, prop::FIRE_MAX_FUNDING);
                get_f32(&mut t.strike_efficiency, prop::FIRE_STRIKE_EFFICIENCY);
                get_pct(&mut t.min_funding_to_end, prop::FIRE_MIN_FUNDING_TO_END);
                get_s32(&mut t.max_strike_duration, prop::FIRE_MAX_STRIKE_DURATION);
                get_f32(&mut t.no_road_multiplier, prop::FIRE_NO_ROAD_MULTIPLIER);
            }
            ProtectionKind::Police => {
                load(&mut t.funding_curve, prop::POLICE_FUNDING_CURVE);
                load(&mut t.jail_curve, prop::JAIL_FUNDING_CURVE);
                load(&mut t.strike_curve, prop::POLICE_STRIKE_CURVE);
                load(&mut t.jailbreak_curve, prop::JAILBREAK_CURVE);
                get_pct(&mut t.max_funding, prop::POLICE_MAX_FUNDING);
                get_pct(&mut t.rehab_rate, prop::REHAB_RATE);
                get_pct(&mut t.strike_efficiency, prop::POLICE_STRIKE_EFFICIENCY);
                get_s32(&mut t.max_strike_duration, prop::POLICE_MAX_STRIKE_DURATION);
                get_pct(&mut t.min_funding_to_end, prop::POLICE_MIN_FUNDING_TO_END);
                get_s32(&mut t.arrest_threshold, prop::ARREST_THRESHOLD);
                // [choice] fewer than two values: the property counts as absent.
                if let Some(v) = props::f32s(h, prop::ESCAPEE_RANGE) {
                    if v.len() >= 2 {
                        t.escapee_range = (pct(v[0]), pct(v[1]));
                    }
                }
                get_pct(&mut t.end_jailbreak_capacity, prop::END_JAILBREAK_CAPACITY);
                get_pct(&mut t.overflow_factor, prop::OVERFLOW_FACTOR);
            }
        }
        t
    }
}

/// The shipped exemplar of `kind` (§6.7, "Shipped" column), for hosts and
/// tests without the game's data files. Percentages and multipliers are
/// stored as `float32`, durations and thresholds as `sint32`.
pub fn shipped_tuning(kind: ProtectionKind) -> PropertyHolder {
    let curve = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
    let f = |v: f32| Variant::Scalar(Elem::F32(v));
    let s = |v: i32| Variant::Scalar(Elem::I32(v));
    let mut h = PropertyHolder::new();
    match kind {
        ProtectionKind::Fire => {
            h.add(
                prop::FIRE_FUNDING_CURVE,
                curve(&[
                    0.0, 10.0, 50.0, 25.0, 75.0, 45.0, 90.0, 75.0, 100.0, 100.0, 120.0, 110.0,
                ]),
            );
            h.add(
                prop::FIRE_STRIKE_CURVE,
                curve(&[0.0, 100.0, 25.0, 25.0, 50.0, 10.0, 70.0, 0.0]),
            );
            h.add(prop::FIRE_MAX_FUNDING, f(120.0));
            h.add(prop::FIRE_STRIKE_EFFICIENCY, f(0.2));
            h.add(prop::FIRE_MIN_FUNDING_TO_END, f(90.0));
            h.add(prop::FIRE_MAX_STRIKE_DURATION, s(548));
            h.add(prop::FIRE_NO_ROAD_MULTIPLIER, f(0.8));
        }
        ProtectionKind::Police => {
            h.add(
                prop::POLICE_FUNDING_CURVE,
                curve(&[0.0, 0.0, 100.0, 100.0, 120.0, 110.0]),
            );
            h.add(
                prop::JAIL_FUNDING_CURVE,
                curve(&[0.0, 0.0, 10.0, 50.0, 100.0, 100.0, 120.0, 110.0]),
            );
            h.add(
                prop::POLICE_STRIKE_CURVE,
                curve(&[0.0, 100.0, 25.0, 25.0, 50.0, 10.0, 70.0, 0.0]),
            );
            h.add(
                prop::JAILBREAK_CURVE,
                curve(&[0.0, 0.0, 80.0, 0.0, 90.0, 10.0, 100.0, 100.0]),
            );
            h.add(prop::POLICE_MAX_FUNDING, f(120.0));
            h.add(prop::REHAB_RATE, f(10.0));
            h.add(prop::POLICE_STRIKE_EFFICIENCY, f(20.0));
            h.add(prop::POLICE_MAX_STRIKE_DURATION, s(548));
            h.add(prop::POLICE_MIN_FUNDING_TO_END, f(90.0));
            h.add(prop::ARREST_THRESHOLD, s(20));
            h.add(prop::ESCAPEE_RANGE, curve(&[10.0, 50.0]));
            h.add(prop::END_JAILBREAK_CAPACITY, f(80.0));
        }
    }
    h
}
