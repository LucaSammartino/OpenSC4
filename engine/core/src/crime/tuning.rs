// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tuning of §8: code defaults, the shipped exemplar and the "default"
//! curve-loading pattern (foundations §4.1).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};

use super::prop;

/// Code defaults of the curves (§8).
const EQ_DEFAULT: &[(f32, f32)] = &[(0.0, 0.1), (255.0, 0.0)];
const JOBLESS_DEFAULT: &[(f32, f32)] = &[(1.0, 0.0), (2.0, 0.1), (5.0, 0.5)];
const CRIMES_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (1e6, 50000.0)];
const ATTENUATION_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (255.0, 1.0)];

/// The tuning in effect (§8).
#[derive(Clone, Debug, PartialEq)]
pub struct CrimeTuning {
    /// Crime grid tract size S (`0x47E2C200`, u32).
    pub tract_size: u32,
    /// Crime grid decay factor (`0x8A0CBAAC`).
    pub decay: f32,
    /// Reoffend multiplier for released criminals (`0x8A0CBAAD`).
    pub reoffend: f32,
    /// EQ → base criminality, per wealth.
    pub eq_curves: [ResponseCurve; 3],
    /// Jobless ratio → criminality, per wealth.
    pub jobless_curves: [ResponseCurve; 3],
    /// Criminality → crimes.
    pub crimes_curve: ResponseCurve,
    /// (police − crime effect) → attenuation.
    pub attenuation_curve: ResponseCurve,
    /// Average crime / 255 → prop share (no code default).
    pub prop_request_curve: ResponseCurve,
}

impl Default for CrimeTuning {
    /// The scalar defaults and the code-default curves; the prop request
    /// curve is empty (a new simulator's curves).
    fn default() -> Self {
        let eq = || ResponseCurve::from_points(EQ_DEFAULT);
        let jobless = || ResponseCurve::from_points(JOBLESS_DEFAULT);
        CrimeTuning {
            tract_size: 2,
            decay: 0.5,
            reoffend: 1.0,
            eq_curves: [eq(), eq(), eq()],
            jobless_curves: [jobless(), jobless(), jobless()],
            crimes_curve: ResponseCurve::from_points(CRIMES_DEFAULT),
            attenuation_curve: ResponseCurve::from_points(ATTENUATION_DEFAULT),
            prop_request_curve: ResponseCurve::default(),
        }
    }
}

/// A curve property (foundations §4.1, [`props::curve`]): `None` when
/// absent, not numeric or a single value ([choice]); otherwise its points
/// (an empty curve when it has no complete point).
fn curve_prop(h: &PropertyHolder, id: u32) -> Option<ResponseCurve> {
    props::curve(h, id)
}

impl CrimeTuning {
    /// Reads the tuning (§8): the three scalars are reset to their defaults
    /// and then read from the exemplar; each curve with a code default is
    /// rebuilt from it and replaced by its property when present
    /// ("default" pattern); the prop request curve is replaced only when
    /// its property is present ("keep"). Without the exemplar ([choice]):
    /// code-default curves, the prop request curve kept.
    pub fn read(&mut self, exemplar: Option<&PropertyHolder>) {
        let defaults = CrimeTuning::default();
        self.tract_size = defaults.tract_size;
        self.decay = defaults.decay;
        self.reoffend = defaults.reoffend;
        self.eq_curves = defaults.eq_curves;
        self.jobless_curves = defaults.jobless_curves;
        self.crimes_curve = defaults.crimes_curve;
        self.attenuation_curve = defaults.attenuation_curve;
        let Some(h) = exemplar else { return };
        if let Some(v) = props::u32_of(h, prop::TRACT_SIZE) {
            self.tract_size = v;
        }
        if let Some(v) = props::f32_of(h, prop::DECAY) {
            self.decay = v;
        }
        if let Some(v) = props::f32_of(h, prop::REOFFEND) {
            self.reoffend = v;
        }
        let load = |curve: &mut ResponseCurve, id: u32| {
            if let Some(c) = curve_prop(h, id) {
                *curve = c;
            }
        };
        for w in 0..3 {
            load(&mut self.eq_curves[w], prop::EQ_CURVES[w]);
            load(&mut self.jobless_curves[w], prop::JOBLESS_CURVES[w]);
        }
        load(&mut self.crimes_curve, prop::CRIMES_CURVE);
        load(&mut self.attenuation_curve, prop::ATTENUATION_CURVE);
        load(&mut self.prop_request_curve, prop::PROP_REQUEST_CURVE);
    }
}

/// The shipped "Crime Simulator" exemplar (§8, `SimCity_1.dat`): the tract
/// size is absent.
pub fn shipped_tuning() -> PropertyHolder {
    let curve = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
    let mut h = PropertyHolder::new();
    h.add(
        prop::PROP_REQUEST_CURVE,
        curve(&[0.0, 0.0, 0.1, 2.0, 0.2, 4.0, 0.4, 6.0, 1.0, 10.0]),
    );
    h.add(prop::EQ_CURVES[0], curve(&[0.0, 0.4, 100.0, 0.3, 200.0, 0.0]));
    h.add(prop::EQ_CURVES[1], curve(&[0.0, 0.15, 100.0, 0.11, 200.0, 0.0]));
    h.add(prop::EQ_CURVES[2], curve(&[0.0, 0.1, 100.0, 0.07, 200.0, 0.0]));
    h.add(
        prop::JOBLESS_CURVES[0],
        curve(&[-1.0, 0.1, -0.7, 0.07, 0.0, 0.0, 0.7, -0.07, 1.0, -0.1]),
    );
    h.add(
        prop::JOBLESS_CURVES[1],
        curve(&[-1.0, 0.07, -0.7, 0.05, 0.0, 0.0, 0.7, -0.05, 1.0, -0.07]),
    );
    h.add(
        prop::JOBLESS_CURVES[2],
        curve(&[-1.0, 0.05, -0.7, 0.03, 0.0, 0.0, 0.7, -0.03, 1.0, -0.05]),
    );
    h.add(
        prop::CRIMES_CURVE,
        curve(&[
            0.0, 0.0, 200.0, 1.0, 5000.0, 5.0, 10000.0, 10.0, 20000.0, 25.0, 200000.0, 40.0, 400000.0, 55.0, 600000.0,
            60.0, 900000.0, 65.0,
        ]),
    );
    h.add(
        prop::ATTENUATION_CURVE,
        curve(&[0.0, 0.0, 100.0, 0.05, 200.0, 0.2, 255.0, 0.5]),
    );
    h.add(prop::DECAY, Variant::Scalar(Elem::F32(0.8)));
    h.add(prop::REOFFEND, Variant::Scalar(Elem::F32(0.66)));
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-6 * b.abs().max(1e-3)
    }

    #[test]
    fn defaults_and_shipped() {
        let mut t = CrimeTuning::default();
        assert_eq!((t.tract_size, t.decay, t.reoffend), (2, 0.5, 1.0));
        assert!(close(t.eq_curves[0].eval(127.5), 0.05));
        assert!(close(t.jobless_curves[2].eval(3.5), 0.3));
        assert!(close(t.crimes_curve.eval(500_000.0), 25_000.0));
        assert!(close(t.attenuation_curve.eval(51.0), 0.2));
        assert!(t.prop_request_curve.is_empty());
        t.read(Some(&shipped_tuning()));
        assert_eq!((t.tract_size, t.decay, t.reoffend), (2, 0.8, 0.66));
        assert!(close(t.eq_curves[0].eval(25.0), 0.375));
        assert!(close(t.jobless_curves[0].eval(0.5), -0.05));
        assert!(close(t.attenuation_curve.eval(150.0), 0.125));
        assert!(close(t.prop_request_curve.eval(0.2), 4.0));
        // Missing exemplar: scalars and defaulted curves reset, prop curve kept.
        t.read(None);
        assert_eq!((t.tract_size, t.decay, t.reoffend), (2, 0.5, 1.0));
        assert!(close(t.attenuation_curve.eval(51.0), 0.2));
        assert!(close(t.prop_request_curve.eval(0.2), 4.0));
        // Missing property: the default; empty array: an empty curve; a
        // scalar or a one-element array: absent (foundations §4.1 [choice]).
        let mut h = PropertyHolder::new();
        h.add(prop::CRIMES_CURVE, Variant::array(mem::FLOAT32, vec![]));
        h.add(prop::EQ_CURVES[0], Variant::array(mem::FLOAT32, vec![Elem::F32(1.0)]));
        h.add(prop::ATTENUATION_CURVE, Variant::Scalar(Elem::F32(3.0)));
        h.add(prop::TRACT_SIZE, Variant::Scalar(Elem::U32(4)));
        let mut t = CrimeTuning::default();
        t.read(Some(&shipped_tuning()));
        t.read(Some(&h));
        assert_eq!(t.tract_size, 4);
        assert!(t.crimes_curve.is_empty());
        assert_eq!(t.eq_curves[0], CrimeTuning::default().eq_curves[0]);
        assert!(close(t.attenuation_curve.eval(51.0), 0.2));
        assert!(close(t.eq_curves[1].eval(127.5), 0.05));
        assert!(close(t.prop_request_curve.eval(0.2), 4.0), "keep");
    }
}
