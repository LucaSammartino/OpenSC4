// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The "Neighbor Deals" tuning exemplar (§12).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

/// The exemplar `0x6534284A / 0x7A4A8458 / 0x6A469021` (§12).
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0x7A4A_8458,
    instance: 0x6A46_9021,
};

/// The shipped deal-price scarcity curve `0x0A469353` (§8.3).
pub const DEAL_PRICE_CURVE: [f32; 8] = [0.0, 1.0, 0.1, 0.3, 0.3, 0.1, 1.0, 0.0];
/// The shipped garbage cost-per-unit curve `0x4B61FFC9` (§8.3).
pub const GARBAGE_COST_CURVE: [f32; 6] = [0.0, 1e-5, 0.0075, 0.0075, 1.0, 0.075];

/// Property IDs (§12).
pub mod prop {
    pub const DEAL_PRICE_CURVE: u32 = 0x0A46_9353;
    pub const GARBAGE_COST_CURVE: u32 = 0x4B61_FFC9;
    pub const BASE_POWER: u32 = 0xEA46_932C;
    pub const BASE_WATER: u32 = 0x6A46_9337;
    pub const BASE_GARBAGE: u32 = 0x8A46_933D;
    pub const AMOUNT_SCALE: u32 = 0xEA4A_D2FF;
    pub const MIN_MARKUP: u32 = 0x6A46_9342;
    pub const MAX_MARKUP: u32 = 0xCA46_9346;
    pub const GRACE_DAYS: u32 = 0x6B58_8FAD;
    pub const PROBATION_DAYS: u32 = 0x8B58_C59E;
    pub const MAX_TERM_YEARS: u32 = 0xEA46_934F;
    pub const MIN_TERM_YEARS: u32 = 0xAA46_934B;
    pub const PROPOSAL_COST_FACTOR: u32 = 0xCA46_931F;
    pub const PROPOSAL_MIN_BUDGET: u32 = 0xEA4A_E9EF;
}

/// The values the simulator reads, with each reader's default (§12).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub base_power: f32,
    pub base_water: f32,
    pub base_garbage: f32,
    /// `0xEA4AD2FF`, default 1.
    pub amount_scale: f32,
    /// `0x6A469342` and `0xCA469346`, read as **f64** by `GetDealCost` (§12):
    /// the f32 value widened, default 1.
    pub min_markup: f64,
    pub max_markup: f64,
    /// `0x6B588FAD`, `0x8B58C59E` (days), default 0.
    pub grace_days: f32,
    pub probation_days: f32,
    /// `0xEA46934F` as present: `None` when missing. The readers' defaults
    /// differ (§12): 5 in proposals, 0 in the deal simulation, and
    /// `UpdateCommodityStats` treats it as 0 *[choice]*.
    pub max_term_years: Option<f32>,
    /// `0xAA46934B`, default 1.
    pub min_term_years: f32,
    /// `0xCA46931F`, default 1 (read as f32).
    pub proposal_cost_factor: f32,
    /// `0xEA4AE9EF`, default 0.
    pub proposal_min_budget: f32,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            base_power: 0.0,
            base_water: 0.0,
            base_garbage: 0.0,
            amount_scale: 1.0,
            min_markup: 1.0,
            max_markup: 1.0,
            grace_days: 0.0,
            probation_days: 0.0,
            max_term_years: None,
            min_term_years: 1.0,
            proposal_cost_factor: 1.0,
            proposal_min_budget: 0.0,
        }
    }
}

impl Tuning {
    pub fn read(h: Option<&PropertyHolder>) -> Tuning {
        let d = Tuning::default();
        let Some(h) = h else { return d };
        let f = |id: u32, def: f32| props::f32_of(h, id).unwrap_or(def);
        Tuning {
            base_power: f(prop::BASE_POWER, d.base_power),
            base_water: f(prop::BASE_WATER, d.base_water),
            base_garbage: f(prop::BASE_GARBAGE, d.base_garbage),
            amount_scale: f(prop::AMOUNT_SCALE, d.amount_scale),
            min_markup: props::f32_of(h, prop::MIN_MARKUP).map_or(d.min_markup, f64::from),
            max_markup: props::f32_of(h, prop::MAX_MARKUP).map_or(d.max_markup, f64::from),
            grace_days: f(prop::GRACE_DAYS, d.grace_days),
            probation_days: f(prop::PROBATION_DAYS, d.probation_days),
            max_term_years: props::f32_of(h, prop::MAX_TERM_YEARS),
            min_term_years: f(prop::MIN_TERM_YEARS, d.min_term_years),
            proposal_cost_factor: f(prop::PROPOSAL_COST_FACTOR, d.proposal_cost_factor),
            proposal_min_budget: f(prop::PROPOSAL_MIN_BUDGET, d.proposal_min_budget),
        }
    }

    /// The base amount of a deal type (§3); 0 for unknown types.
    pub fn base_amount(&self, deal_type: u32) -> f32 {
        match super::deal_type::commodity(deal_type) {
            Some(1) => self.base_power,
            Some(2) => self.base_water,
            Some(3) => self.base_garbage,
            _ => 0.0,
        }
    }
}

/// The curve stored under `id`, if present.
pub fn curve_of(h: Option<&PropertyHolder>, id: u32) -> Option<ResponseCurve> {
    props::curve(h?, id)
}

/// An `f32` array property value.
pub fn f32_array(values: &[f32]) -> Variant {
    Variant::Array {
        elem: mem::FLOAT32,
        items: values.iter().map(|&v| Elem::F32(v)).collect(),
    }
}

/// The shipped exemplar (§12).
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let f = |v: f32| Variant::Scalar(Elem::F32(v));
    h.add(prop::DEAL_PRICE_CURVE, f32_array(&DEAL_PRICE_CURVE));
    h.add(prop::GARBAGE_COST_CURVE, f32_array(&GARBAGE_COST_CURVE));
    h.add(prop::BASE_POWER, f(1000.0));
    h.add(prop::BASE_WATER, f(1000.0));
    h.add(prop::BASE_GARBAGE, f(1000.0));
    h.add(prop::AMOUNT_SCALE, f(2.5));
    h.add(prop::MIN_MARKUP, f(0.1));
    h.add(prop::MAX_MARKUP, f(0.5));
    h.add(prop::GRACE_DAYS, f(90.0));
    h.add(prop::PROBATION_DAYS, f(90.0));
    h.add(prop::MAX_TERM_YEARS, f(1.0));
    h.add(prop::MIN_TERM_YEARS, f(1.0));
    h.add(prop::PROPOSAL_COST_FACTOR, f(60.0));
    h.add(prop::PROPOSAL_MIN_BUDGET, f(5000.0));
    h
}
