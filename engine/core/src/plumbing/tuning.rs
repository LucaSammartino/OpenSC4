// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tuning (§8, `ReadSimulatorTunableValues`).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};
use crate::x87;

/// The "Utilities" exemplar (§8).
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC911_E35B,
};

/// Tuning property IDs (§8).
pub mod id {
    pub const RADIUS: u32 = 0x68F5_436A;
    pub const FRESH_REACH: u32 = 0x68F5_436B;
    pub const SALT_REACH: u32 = 0x68F5_436C;
    pub const DISTANCE_LOSS: u32 = 0x68F5_436D;
    pub const MAX_PUMPED_POLLUTION: u32 = 0xAA16_1F81;
    pub const BURST_PROBABILITY: u32 = 0x8A23_5C56;
    pub const MAX_FUNDING: u32 = 0xEA40_25A9;
    pub const COST_PER_PIPE: u32 = 0x8A84_778F;
    pub const EFFICIENCY_CURVE: u32 = 0x6A41_0BF2;
    pub const FUNDING_DECAY_CURVE: u32 = 0x0911_E2E8;
    pub const USAGE_DECAY_CURVE: u32 = 0x0911_E2E9;
    pub const PIPE_DECAY_CURVE: u32 = 0xCA71_38C4;
}

/// The tuning values (§3, §8).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// Water pipe effect radius (rings).
    pub radius: u32,
    pub fresh_reach: u32,
    pub salt_reach: u32,
    /// % of capacity kept beyond distance 1.
    pub distance_loss: u32,
    pub max_pumped_pollution: u32,
    pub burst_probability: f32,
    /// The water department's max funding (a fraction).
    pub max_funding: f32,
    pub cost_per_pipe: f32,
    /// `0x6A410BF2`: funding % → efficiency %.
    pub efficiency_curve: ResponseCurve,
    /// `0x0911E2E8`: funding fraction → decay multiplier.
    pub funding_decay_curve: ResponseCurve,
    /// `0x0911E2E9`: usage ratio → decay multiplier.
    pub usage_decay_curve: ResponseCurve,
    /// `0xCA7138C4`: funding % → pipe decay (−) / repair (+) per month.
    pub pipe_decay_curve: ResponseCurve,
}

impl Default for Tuning {
    /// The constructor values (§3): code-default scalars, empty curves.
    fn default() -> Self {
        Tuning {
            radius: 7,
            fresh_reach: 2,
            salt_reach: 1,
            distance_loss: 20,
            max_pumped_pollution: 100,
            burst_probability: 0.3,
            max_funding: 1.2,
            cost_per_pipe: 0.0,
            efficiency_curve: ResponseCurve::default(),
            funding_decay_curve: ResponseCurve::default(),
            usage_decay_curve: ResponseCurve::default(),
            pipe_decay_curve: ResponseCurve::default(),
        }
    }
}

/// The curve "(0,0)(100,100)" that replaces an empty curve after a load.
fn identity_100() -> ResponseCurve {
    ResponseCurve::from_points(&[(0.0, 0.0), (100.0, 100.0)])
}

impl Tuning {
    /// `ReadSimulatorTunableValues` (§8): the three code-default curves
    /// first; without an exemplar nothing else changes; otherwise the
    /// present scalars, then the curves ("keep" loads), then the
    /// "empty → (0,0)(100,100)" rule for `0x6A410BF2`, `0x0911E2E8` and
    /// `0x0911E2E9`.
    pub fn reload(&mut self, exemplar: Option<&PropertyHolder>) {
        self.funding_decay_curve = ResponseCurve::from_points(&[(0.0, 1.0), (1.0, 1.0)]);
        self.usage_decay_curve = ResponseCurve::from_points(&[(0.0, 1.0), (1.0, 1.0)]);
        self.pipe_decay_curve = ResponseCurve::from_points(&[(0.0, -0.02), (120.0, 0.01)]);
        let Some(h) = exemplar else { return };
        let u = |pid: u32, v: &mut u32| {
            if let Some(x) = props::u32_of(h, pid) {
                *v = x;
            }
        };
        let f = |pid: u32, v: &mut f32| {
            if let Some(x) = props::f32_of(h, pid) {
                *v = x;
            }
        };
        u(id::RADIUS, &mut self.radius);
        u(id::FRESH_REACH, &mut self.fresh_reach);
        u(id::SALT_REACH, &mut self.salt_reach);
        u(id::DISTANCE_LOSS, &mut self.distance_loss);
        u(id::MAX_PUMPED_POLLUTION, &mut self.max_pumped_pollution);
        f(id::BURST_PROBABILITY, &mut self.burst_probability);
        if let Some(p) = props::f32_of(h, id::MAX_FUNDING) {
            // The SC4Percentage getter (PL6): value × the `float` 0.01f, one
            // 24-bit rounding, so the shipped 120 gives 1.1999999
            // (0x3F999999), not 1.2.
            self.max_funding = x87::r24(f64::from(p) * f64::from(0.01f32)) as f32;
        }
        f(id::COST_PER_PIPE, &mut self.cost_per_pipe);
        let c = |pid: u32, v: &mut ResponseCurve| {
            if let Some(curve) = props::curve(h, pid) {
                *v = curve;
            }
        };
        c(id::EFFICIENCY_CURVE, &mut self.efficiency_curve);
        c(id::FUNDING_DECAY_CURVE, &mut self.funding_decay_curve);
        c(id::USAGE_DECAY_CURVE, &mut self.usage_decay_curve);
        c(id::PIPE_DECAY_CURVE, &mut self.pipe_decay_curve);
        for curve in [
            &mut self.efficiency_curve,
            &mut self.funding_decay_curve,
            &mut self.usage_decay_curve,
        ] {
            if curve.is_empty() {
                *curve = identity_100();
            }
        }
    }

    /// The shipped "Utilities" exemplar's plumbing properties (§8), for
    /// hosts and tests without the game's data files.
    pub fn shipped_exemplar() -> PropertyHolder {
        let mut h = PropertyHolder::new();
        let u = |v: u32| Variant::Scalar(Elem::U32(v));
        let f = |v: f32| Variant::Scalar(Elem::F32(v));
        let curve = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
        h.add(id::RADIUS, u(7));
        h.add(id::FRESH_REACH, u(256));
        h.add(id::SALT_REACH, u(256));
        h.add(id::DISTANCE_LOSS, u(100));
        h.add(id::MAX_PUMPED_POLLUTION, u(512));
        h.add(id::BURST_PROBABILITY, f(0.03));
        h.add(id::MAX_FUNDING, f(120.0));
        h.add(id::COST_PER_PIPE, f(0.1));
        h.add(
            id::EFFICIENCY_CURVE,
            curve(&[0.0, 0.0, 50.0, 30.0, 90.0, 80.0, 100.0, 100.0, 120.0, 110.0]),
        );
        h.add(id::FUNDING_DECAY_CURVE, curve(&[0.0, 1.1, 1.0, 1.0, 1.2, 0.9]));
        h.add(
            id::USAGE_DECAY_CURVE,
            curve(&[
                0.0, 0.1, 0.25, 0.2, 0.5, 0.45, 0.75, 0.85, 0.9, 1.25, 1.0, 1.55, 1.1, 2.0,
            ]),
        );
        h.add(
            id::PIPE_DECAY_CURVE,
            curve(&[0.0, -0.01, 20.0, -0.005, 50.0, 0.0, 100.0, 0.02, 110.0, 0.05]),
        );
        h
    }
}
