// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tuning exemplar (§7.1).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

/// The tuning exemplar's key (§7.1, "Utilities").
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC911_E35B,
};

/// Tuning property IDs (§7.1).
pub mod id {
    pub const TRAFFIC_AIR_FACTOR: u32 = 0x6950_1944;
    pub const MAX_AIR_UI: u32 = 0x4913_4BDD;
    pub const MAX_WATER_UI: u32 = 0x0913_4BE2;
    pub const MAX_GARBAGE_UI: u32 = 0x4913_4BD4;
    pub const MAX_RADIATION_UI: u32 = 0x8913_5044;
    pub const MAX_AIR_DELTA: u32 = 0x6A40_6201;
    pub const MAX_WATER_DELTA: u32 = 0x8A40_620B;
    pub const AIR_THRESHOLD: u32 = 0xA913_51A6;
    pub const WATER_THRESHOLD: u32 = 0x0913_51AE;
    pub const GARBAGE_THRESHOLD: u32 = 0x0913_519B;
    pub const RADIATION_THRESHOLD: u32 = 0x2913_51B5;
    pub const DECOMPOSITION: u32 = 0x0913_5479;
    pub const GARBAGE_SCALING: u32 = 0x0913_5482;
    pub const COST_PER_UNIT: u32 = 0xAA1F_8AB7;
    pub const MAX_GARBAGE_ON_VALVE: u32 = 0x4A65_2679;
    pub const LANDFILL_TILE_CAPACITY: u32 = 0x4A65_267A;
    pub const PROP_CURVE: u32 = 0x69A4_37F6;
    pub const WATER_EFFECT_CURVE: u32 = 0x0A10_C772;
    pub const AIR_EFFECT_CURVE: u32 = 0x0A10_C773;
    pub const RADIATION_EFFECT_CURVE: u32 = 0x0A10_C774;
    pub const GARBAGE_EFFECT_CURVE: u32 = 0x0A10_C775;
    pub const TRANSPORT_CURVE: u32 = 0x8C19_C3E9;
    pub const FUNDING_DECAY_CURVE: u32 = 0x0911_E2E8;
    pub const USAGE_DECAY_CURVE: u32 = 0x0911_E2E9;
    pub const EFFICIENCY_CURVE: u32 = 0x6911_E2DF;
}

/// The tuning in effect (§7.1). Curves follow foundations §4.1: the decay
/// curves use the "default" pattern (constant 1), the others "empty".
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub traffic_air_factor: f32,
    pub max_air_ui: u32,
    pub max_water_ui: u32,
    pub max_garbage_ui: u32,
    pub max_radiation_ui: u32,
    pub max_air_delta: u32,
    pub max_water_delta: u32,
    pub air_threshold: u32,
    pub water_threshold: u32,
    pub garbage_threshold: u32,
    pub radiation_threshold: u32,
    pub decomposition_amount: u32,
    pub garbage_scaling: f32,
    /// "Keep": unchanged when the property is absent (0 in a new object).
    pub cost_per_unit: f32,
    pub max_garbage_on_valve: u32,
    pub landfill_tile_capacity: u32,
    pub prop_curve: ResponseCurve,
    /// Water, air, radiation, garbage (the effect array order).
    pub effect_curves: [ResponseCurve; 4],
    pub transport_curve: ResponseCurve,
    pub funding_decay_curve: ResponseCurve,
    pub usage_decay_curve: ResponseCurve,
}

fn constant_one() -> ResponseCurve {
    ResponseCurve::from_points(&[(0.0, 1.0), (1.0, 1.0)])
}

impl Default for Tuning {
    /// The code defaults (§7.1); `cost_per_unit` 0 as in a new object.
    fn default() -> Self {
        Tuning {
            traffic_air_factor: 8.0,
            max_air_ui: 27000,
            max_water_ui: 32000,
            max_garbage_ui: 27000,
            max_radiation_ui: 32000,
            max_air_delta: 50,
            max_water_delta: 50,
            air_threshold: 16000,
            water_threshold: 16000,
            garbage_threshold: 16000,
            radiation_threshold: 1,
            decomposition_amount: 5000,
            garbage_scaling: 0.125,
            cost_per_unit: 0.0,
            max_garbage_on_valve: 655_340,
            landfill_tile_capacity: 32767,
            prop_curve: ResponseCurve::default(),
            effect_curves: Default::default(),
            transport_curve: ResponseCurve::default(),
            funding_decay_curve: constant_one(),
            usage_decay_curve: constant_one(),
        }
    }
}

/// A curve property ([`props::curve`], foundations §4.1/§4.3): absent when
/// missing, non-numeric or a single value (the caller keeps its default);
/// a present property with no points is an empty curve (evaluates to 0).
pub(crate) fn curve_of(h: &PropertyHolder, id: u32) -> Option<ResponseCurve> {
    props::curve(h, id)
}

impl Tuning {
    /// Re-reads the exemplar (§7.1): every value back to its code default and
    /// every curve emptied (decay curves: constant 1) first, then the present
    /// properties. Without an exemplar the defaults stay. `cost_per_unit`
    /// keeps its previous value when absent.
    pub fn reload(&mut self, exemplar: Option<&PropertyHolder>) {
        let cost = self.cost_per_unit;
        *self = Tuning {
            cost_per_unit: cost,
            ..Tuning::default()
        };
        let Some(h) = exemplar else { return };
        let u = |id: u32, v: &mut u32| {
            if let Some(x) = props::u32_of(h, id) {
                *v = x;
            }
        };
        let f = |id: u32, v: &mut f32| {
            if let Some(x) = props::f32_of(h, id) {
                *v = x;
            }
        };
        f(id::TRAFFIC_AIR_FACTOR, &mut self.traffic_air_factor);
        u(id::MAX_AIR_UI, &mut self.max_air_ui);
        u(id::MAX_WATER_UI, &mut self.max_water_ui);
        u(id::MAX_GARBAGE_UI, &mut self.max_garbage_ui);
        u(id::MAX_RADIATION_UI, &mut self.max_radiation_ui);
        u(id::MAX_AIR_DELTA, &mut self.max_air_delta);
        u(id::MAX_WATER_DELTA, &mut self.max_water_delta);
        u(id::AIR_THRESHOLD, &mut self.air_threshold);
        u(id::WATER_THRESHOLD, &mut self.water_threshold);
        u(id::GARBAGE_THRESHOLD, &mut self.garbage_threshold);
        u(id::RADIATION_THRESHOLD, &mut self.radiation_threshold);
        u(id::DECOMPOSITION, &mut self.decomposition_amount);
        f(id::GARBAGE_SCALING, &mut self.garbage_scaling);
        f(id::COST_PER_UNIT, &mut self.cost_per_unit);
        u(id::MAX_GARBAGE_ON_VALVE, &mut self.max_garbage_on_valve);
        u(id::LANDFILL_TILE_CAPACITY, &mut self.landfill_tile_capacity);
        let c = |id: u32, v: &mut ResponseCurve| {
            if let Some(curve) = curve_of(h, id) {
                *v = curve;
            }
        };
        c(id::PROP_CURVE, &mut self.prop_curve);
        let [w, a, r, g] = &mut self.effect_curves;
        c(id::WATER_EFFECT_CURVE, w);
        c(id::AIR_EFFECT_CURVE, a);
        c(id::RADIATION_EFFECT_CURVE, r);
        c(id::GARBAGE_EFFECT_CURVE, g);
        c(id::TRANSPORT_CURVE, &mut self.transport_curve);
        c(id::FUNDING_DECAY_CURVE, &mut self.funding_decay_curve);
        c(id::USAGE_DECAY_CURVE, &mut self.usage_decay_curve);
    }
}

/// The shipped "Utilities" exemplar's pollution properties (§7.1), for
/// hosts and tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let u = |v: u32| Variant::Scalar(Elem::U32(v));
    let f = |v: f32| Variant::Scalar(Elem::F32(v));
    let curve = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
    h.add(id::TRAFFIC_AIR_FACTOR, f(0.25));
    h.add(id::MAX_AIR_UI, u(1024));
    h.add(id::MAX_WATER_UI, u(1024));
    h.add(id::MAX_GARBAGE_UI, u(32000));
    h.add(id::MAX_RADIATION_UI, u(1024));
    h.add(id::MAX_AIR_DELTA, u(150));
    h.add(id::MAX_WATER_DELTA, u(150));
    h.add(id::AIR_THRESHOLD, u(512));
    h.add(id::WATER_THRESHOLD, u(512));
    h.add(id::GARBAGE_THRESHOLD, u(1500));
    h.add(id::RADIATION_THRESHOLD, u(1));
    h.add(id::DECOMPOSITION, u(2000));
    h.add(id::GARBAGE_SCALING, f(0.01));
    h.add(id::COST_PER_UNIT, f(0.005));
    h.add(id::MAX_GARBAGE_ON_VALVE, u(65536));
    h.add(id::LANDFILL_TILE_CAPACITY, u(30000));
    h.add(id::PROP_CURVE, curve(&[0.0, 0.0, 0.0625, 5.0, 0.125, 10.0]));
    h.add(
        id::WATER_EFFECT_CURVE,
        curve(&[0.0, 0.0, 0.002, 0.05, 0.005, 0.15, 0.01, 0.4, 0.02, 1.0]),
    );
    h.add(
        id::AIR_EFFECT_CURVE,
        curve(&[0.0, 0.0, 0.005, 0.05, 0.01, 0.25, 0.02, 0.5, 0.03, 0.6]),
    );
    h.add(id::RADIATION_EFFECT_CURVE, curve(&[0.0, 0.0, 0.001, 1.0, 0.01, 1.0]));
    h.add(
        id::GARBAGE_EFFECT_CURVE,
        curve(&[
            0.0, 0.0, 0.02, 0.02, 0.03, 0.06, 0.04, 0.2, 0.05, 0.4, 0.06, 0.53, 0.085, 0.75, 0.13, 1.0,
        ]),
    );
    h.add(id::TRANSPORT_CURVE, curve(&[0.0, 0.0, 65536.0, 3.2768e8]));
    h.add(id::FUNDING_DECAY_CURVE, curve(&[0.0, 1.1, 1.0, 1.0, 1.2, 0.9]));
    h.add(
        id::USAGE_DECAY_CURVE,
        curve(&[
            0.0, 0.1, 0.25, 0.2, 0.5, 0.45, 0.75, 0.85, 0.9, 1.25, 1.0, 1.55, 1.1, 2.0,
        ]),
    );
    h.add(
        id::EFFICIENCY_CURVE,
        curve(&[0.0, 0.0, 50.0, 25.0, 90.0, 80.0, 100.0, 100.0, 120.0, 110.0]),
    );
    h
}
