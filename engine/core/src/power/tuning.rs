// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power tuning (§8): code defaults, the shipped "Utilities" exemplar
//! and the reader with the "default" curve pattern (foundations §4.1).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

/// The tuning exemplar (§8): "Utilities", `SimCity_1.dat`.
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC911_E35B,
};

/// Tuning property IDs (§8, with the names the spec gives).
pub mod prop {
    /// "Power radiation radius": R.
    pub const RADIUS: u32 = 0xA911_E2F1;
    /// "Power Funding To Efficiency Curve".
    pub const FUNDING_EFFICIENCY: u32 = 0x6911_E2DF;
    /// "Funding % → decay rate multiplier".
    pub const FUNDING_DECAY: u32 = 0x0911_E2E8;
    /// "Usage % → decay rate multiplier".
    pub const USAGE_DECAY: u32 = 0x0911_E2E9;
    /// "Funding % → pole decay/repair".
    pub const POLE_DECAY: u32 = 0xAA71_3782;
    /// "Power max allowed funding" (f32 × 0.01f).
    pub const MAX_FUNDING: u32 = 0xE911_E2D1;
    /// "Utility Strike Efficiency %" (f32 × 0.01f).
    pub const STRIKE_EFFICIENCY: u32 = 0x0912_2027;
    /// "Power plant max production ratio" (f32 × 0.01f, dead).
    pub const MAX_PRODUCTION_RATIO: u32 = 0xE911_E2FC;
    /// "Power Min Acceptable Funding %" (f32 × 0.01f, dead).
    pub const MIN_ACCEPTABLE_FUNDING: u32 = 0x2912_1E8B;
    /// "Utility Strike Duration" (s32, dead).
    pub const STRIKE_DURATION: u32 = 0x2912_202E;
    /// "Utility Min Funding to End Strike" (f32 × 0.01f, dead).
    pub const MIN_FUNDING_END_STRIKE: u32 = 0x4912_2036;
    /// "Utility Strike Response Curve" (dead).
    pub const STRIKE_RESPONSE: u32 = 0x0911_E2E6;
}

/// Code-default curve points (§8).
pub const DEFAULT_FUNDING_EFFICIENCY: [(f32, f32); 3] = [(0.0, 0.0), (100.0, 100.0), (120.0, 110.0)];
pub const DEFAULT_FUNDING_DECAY: [(f32, f32); 2] = [(0.0, 1.0), (1.0, 1.0)];
pub const DEFAULT_USAGE_DECAY: [(f32, f32); 2] = [(0.0, 1.0), (1.0, 1.0)];
pub const DEFAULT_POLE_DECAY: [(f32, f32); 2] = [(0.0, -0.02), (120.0, 0.01)];
pub const DEFAULT_STRIKE_RESPONSE: [(f32, f32); 2] = [(0.0, 20.0), (100.0, 100.0)];

/// The tuning of §8 (plus the dead values, kept for completeness).
#[derive(Clone, Debug, PartialEq)]
pub struct PowerTuning {
    /// R, the wire radius (§2.2).
    pub radius: u32,
    /// Funding (percent) → efficiency (percent) (§7.1).
    pub funding_efficiency: ResponseCurve,
    /// Department funding (fraction) → decay multiplier (§7.4).
    pub funding_decay: ResponseCurve,
    /// Usage (fraction) → decay multiplier (§7.4).
    pub usage_decay: ResponseCurve,
    /// Funding (percent) → pole decay / repair (§7.6).
    pub pole_decay: ResponseCurve,
    /// `SetMaxAllowedFundingPercentage` for a new city (fraction).
    pub max_funding: f32,
    /// Exposed by slot 47 (fraction).
    pub strike_efficiency: f32,
    /// Dead (§6.3 uses a literal 1.1).
    pub max_production_ratio: f32,
    /// Dead.
    pub min_acceptable_funding: f32,
    /// Dead.
    pub strike_duration: i32,
    /// Dead.
    pub min_funding_end_strike: f32,
    /// Dead.
    pub strike_response: ResponseCurve,
}

impl Default for PowerTuning {
    /// The code defaults (§8).
    fn default() -> Self {
        PowerTuning {
            radius: 2,
            funding_efficiency: ResponseCurve::from_points(&DEFAULT_FUNDING_EFFICIENCY),
            funding_decay: ResponseCurve::from_points(&DEFAULT_FUNDING_DECAY),
            usage_decay: ResponseCurve::from_points(&DEFAULT_USAGE_DECAY),
            pole_decay: ResponseCurve::from_points(&DEFAULT_POLE_DECAY),
            max_funding: 1.2,
            strike_efficiency: 0.5,
            max_production_ratio: 1.1,
            min_acceptable_funding: 0.6,
            strike_duration: 10,
            min_funding_end_strike: 0.9,
            strike_response: ResponseCurve::from_points(&DEFAULT_STRIKE_RESPONSE),
        }
    }
}

/// A curve property: `None` when absent, not numeric, or a single value (a
/// scalar or a one-element array; foundations §4.1 [choice],
/// [`props::curve_of_variant`]); otherwise the curve of its points (empty
/// when it has none).
pub(crate) fn curve_of(v: Option<&Variant>) -> Option<ResponseCurve> {
    crate::props::curve_of_variant(v?)
}

/// `value × 0.01f`: the `float` constant 0.01f (slightly below 0.01), one
/// rounding (§8). Shipped 120 → 1.1999999 (`0x3F999999`), 20 →
/// 0.19999999, 60 → 0.59999996.
pub(crate) fn percent(v: f32) -> f32 {
    v * 0.01f32
}

impl PowerTuning {
    /// Reads the tuning (§8): every read first restores the code defaults
    /// (curves rebuilt from their default points), then loads each property
    /// present in the exemplar (`None` = no exemplar: code defaults).
    pub fn read(&mut self, h: Option<&PropertyHolder>) {
        *self = PowerTuning::default();
        let Some(h) = h else { return };
        if let Some(v) = props::u32_of(h, prop::RADIUS) {
            self.radius = v;
        }
        for (id, slot) in [
            (prop::FUNDING_EFFICIENCY, &mut self.funding_efficiency),
            (prop::FUNDING_DECAY, &mut self.funding_decay),
            (prop::USAGE_DECAY, &mut self.usage_decay),
            (prop::POLE_DECAY, &mut self.pole_decay),
            (prop::STRIKE_RESPONSE, &mut self.strike_response),
        ] {
            if let Some(c) = curve_of(h.get(id)) {
                *slot = c;
            }
        }
        for (id, slot) in [
            (prop::MAX_FUNDING, &mut self.max_funding),
            (prop::STRIKE_EFFICIENCY, &mut self.strike_efficiency),
            (prop::MAX_PRODUCTION_RATIO, &mut self.max_production_ratio),
            (prop::MIN_ACCEPTABLE_FUNDING, &mut self.min_acceptable_funding),
            (prop::MIN_FUNDING_END_STRIKE, &mut self.min_funding_end_strike),
        ] {
            if let Some(v) = props::f32_of(h, id) {
                *slot = percent(v);
            }
        }
        if let Some(v) = props::i64_of(h, prop::STRIKE_DURATION) {
            self.strike_duration = v as i32;
        }
    }
}

/// The shipped "Utilities" exemplar's power properties (§8), for hosts and
/// tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let curve = |pts: &[(f32, f32)]| {
        Variant::array(
            mem::FLOAT32,
            pts.iter().flat_map(|&(x, y)| [Elem::F32(x), Elem::F32(y)]).collect(),
        )
    };
    h.add(prop::RADIUS, Variant::Scalar(Elem::U32(2)));
    h.add(
        prop::FUNDING_EFFICIENCY,
        curve(&[(0.0, 0.0), (50.0, 25.0), (90.0, 80.0), (100.0, 100.0), (120.0, 110.0)]),
    );
    h.add(prop::FUNDING_DECAY, curve(&[(0.0, 1.1), (1.0, 1.0), (1.2, 0.9)]));
    h.add(
        prop::USAGE_DECAY,
        curve(&[
            (0.0, 0.1),
            (0.25, 0.2),
            (0.5, 0.45),
            (0.75, 0.85),
            (0.9, 1.25),
            (1.0, 1.55),
            (1.1, 2.0),
        ]),
    );
    h.add(
        prop::POLE_DECAY,
        curve(&[(0.0, -0.02), (25.0, -0.005), (50.0, 0.0), (100.0, 0.025), (110.0, 0.05)]),
    );
    let f = |v: f32| Variant::Scalar(Elem::F32(v));
    h.add(prop::MAX_FUNDING, f(120.0));
    h.add(prop::STRIKE_EFFICIENCY, f(20.0));
    h.add(prop::MAX_PRODUCTION_RATIO, f(110.0));
    h.add(prop::MIN_ACCEPTABLE_FUNDING, f(60.0));
    h.add(prop::STRIKE_DURATION, Variant::Scalar(Elem::I32(12)));
    h.add(prop::MIN_FUNDING_END_STRIKE, f(90.0));
    h.add(
        prop::STRIKE_RESPONSE,
        curve(&[(0.0, 0.0), (20.0, 45.0), (50.0, 75.0), (100.0, 100.0)]),
    );
    h
}
