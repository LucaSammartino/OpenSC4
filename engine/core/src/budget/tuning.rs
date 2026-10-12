// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Budget tuning (§9): "Budget Tuning Parameters" exemplar
//! `0x6534284A / 0xE7E2C2DB / 0x0BFF6824`.

use crate::curve::ResponseCurve;

use super::TAX_GROUPS;

/// Tuning exemplar key.
pub const TUNING_TYPE: u32 = 0x6534_284A;
pub const TUNING_GROUP: u32 = 0xE7E2_C2DB;
pub const TUNING_INSTANCE: u32 = 0x0BFF_6824;

mod p {
    pub const INITIAL_FUNDS: u32 = 0x6A9C_7718;
    pub const MAX_TAX_RATE: u32 = 0x890E_0A99;
    pub const MIN_ALLOWABLE_FUNDS: u32 = 0x090E_0A9F;
    pub const BOND_INCREMENT: u32 = 0x490E_0AA6;
    pub const MAX_BONDS: u32 = 0x090E_0AAC;
    pub const MAX_BOND_AMOUNT: u32 = 0x890E_0AB4;
    pub const BOND_INTEREST_RATE: u32 = 0x8A3F_B98D;
    pub const BOND_PAYBACK_MONTHS: u32 = 0x0A3F_B9A8;
    pub const INITIAL_TAX_RATE: u32 = 0x0C03_2488;
    pub const DIFFICULTY_TAX_MULTIPLIER: u32 = 0xEBFF_69E7;
    pub const WANING_WEALTH: u32 = 0x8A48_0DF7;
    pub const LV_CURVE: u32 = 0x4AC5_19B8;
    /// TaxRateOccWeight per tax group, in group order (§9).
    pub const OCC_WEIGHTS: [u32; 12] = [
        0xA918_8EF5, // R$
        0xE918_C920, // R$$
        0x4918_C925, // R$$$
        0xA918_C929, // CS$
        0x4918_C92C, // CS$$
        0x4918_C930, // CS$$$ (see §13.1)
        0x8918_C937, // CO$$
        0x4918_C93A, // CO$$$
        0x69A3_6E40, // I-R
        0xA918_C942, // I-D
        0x8918_C93E, // I-M
        0xA9A3_6E47, // I-HT
    ];
}

/// Typed access to the tuning exemplar's properties. Integer values are
/// returned as i64, floating values as f32 (with conversion between them).
pub trait PropertySource {
    fn get_i64s(&self, prop: u32) -> Option<Vec<i64>>;
    fn get_f32s(&self, prop: u32) -> Option<Vec<f32>>;
}

/// No exemplar: every property is missing.
pub struct NoProperties;

impl PropertySource for NoProperties {
    fn get_i64s(&self, _: u32) -> Option<Vec<i64>> {
        None
    }
    fn get_f32s(&self, _: u32) -> Option<Vec<f32>> {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BudgetTuning {
    pub initial_funds: [i64; 3],
    /// Percent.
    pub max_tax_rate: f32,
    pub min_allowable_funds: i64,
    pub bond_increment: i64,
    pub max_bonds: i32,
    pub max_bond_amount: i64,
    /// Fraction (the property is in percent and multiplied by 0.01).
    pub bond_interest_rate: f32,
    pub bond_payback_months: u32,
    /// Percent.
    pub initial_tax_rate: [f32; 3],
    pub difficulty_tax_multiplier: [f32; 3],
    /// Indexed by building wealth minus lot wealth (0..2).
    pub waning_wealth: [f32; 3],
    /// Land value → simoleons per tile.
    pub land_value_curve: ResponseCurve,
    pub occ_weight: [f32; TAX_GROUPS],
}

impl Default for BudgetTuning {
    /// The code defaults of §9.
    fn default() -> Self {
        BudgetTuning {
            // [choice] the original has no code default; use the shipped values.
            initial_funds: [500_000, 200_000, 100_000],
            max_tax_rate: 25.0,
            min_allowable_funds: -100_000,
            bond_increment: 1000,
            max_bonds: 5,
            max_bond_amount: 10_000,
            bond_interest_rate: 0.085,
            bond_payback_months: 120,
            // [choice] the original has no code default; use the shipped values.
            initial_tax_rate: [9.0, 9.0, 9.0],
            // [compat] a deliberately absurd default.
            difficulty_tax_multiplier: [100_000.0; 3],
            waning_wealth: [1.0; 3],
            land_value_curve: ResponseCurve::from_points(&[(0.0, 0.0), (255.0, 10.0)]),
            occ_weight: [1.0; TAX_GROUPS],
        }
    }
}

fn first_i64(src: &dyn PropertySource, id: u32) -> Option<i64> {
    src.get_i64s(id).and_then(|v| v.first().copied())
}

fn first_f32(src: &dyn PropertySource, id: u32) -> Option<f32> {
    src.get_f32s(id).and_then(|v| v.first().copied())
}

/// Overwrites the leading entries of `dst` with those present in `values`.
/// [choice] a shorter array keeps the defaults for the missing entries.
fn overlay<T: Copy, const N: usize>(dst: &mut [T; N], values: Option<Vec<T>>) {
    if let Some(v) = values {
        for (d, s) in dst.iter_mut().zip(v) {
            *d = s;
        }
    }
}

impl BudgetTuning {
    /// Reads the exemplar over the code defaults (§9). With
    /// `fix_cs3_weight == false` (the default) the original bug of §13.1 is
    /// reproduced: the CS$$$ weight stays 1.0 and the CO$$$ weight is
    /// `0x4918C93A` if present, else `0x4918C930`, else 1.0.
    pub fn from_properties(src: &dyn PropertySource, fix_cs3_weight: bool) -> BudgetTuning {
        let mut t = BudgetTuning::default();
        overlay(&mut t.initial_funds, src.get_i64s(p::INITIAL_FUNDS));
        if let Some(v) = first_f32(src, p::MAX_TAX_RATE) {
            t.max_tax_rate = v;
        }
        if let Some(v) = first_i64(src, p::MIN_ALLOWABLE_FUNDS) {
            t.min_allowable_funds = v;
        }
        if let Some(v) = first_i64(src, p::BOND_INCREMENT) {
            t.bond_increment = v;
        }
        if let Some(v) = first_i64(src, p::MAX_BONDS) {
            t.max_bonds = v as i32;
        }
        if let Some(v) = first_i64(src, p::MAX_BOND_AMOUNT) {
            t.max_bond_amount = v;
        }
        if let Some(v) = first_f32(src, p::BOND_INTEREST_RATE) {
            t.bond_interest_rate = v * 0.01;
        }
        if let Some(v) = first_i64(src, p::BOND_PAYBACK_MONTHS) {
            t.bond_payback_months = v as u32;
        }
        overlay(&mut t.initial_tax_rate, src.get_f32s(p::INITIAL_TAX_RATE));
        overlay(
            &mut t.difficulty_tax_multiplier,
            src.get_f32s(p::DIFFICULTY_TAX_MULTIPLIER),
        );
        overlay(&mut t.waning_wealth, src.get_f32s(p::WANING_WEALTH));
        if let Some(curve) = src.get_f32s(p::LV_CURVE).and_then(|v| ResponseCurve::from_flat(&v)) {
            // [choice] a present but empty curve keeps the default.
            t.land_value_curve = curve;
        }
        for (g, id) in p::OCC_WEIGHTS.iter().enumerate() {
            if let Some(v) = first_f32(src, *id) {
                t.occ_weight[g] = v;
            }
        }
        if !fix_cs3_weight {
            // §13.1: the reader stores 0x4918C930 into the CO$$$ slot, then
            // 0x4918C93A overwrites it; CS$$$ is never loaded.
            const CS3: usize = 5;
            const CO3: usize = 7;
            t.occ_weight[CS3] = 1.0;
            t.occ_weight[CO3] = first_f32(src, p::OCC_WEIGHTS[CO3])
                .or_else(|| first_f32(src, p::OCC_WEIGHTS[CS3]))
                .unwrap_or(1.0);
        }
        t
    }

    /// The `difficulty`-indexed value of a per-difficulty array. [choice] An
    /// out-of-range difficulty uses the last entry.
    pub(crate) fn by_difficulty<T: Copy>(arr: &[T; 3], difficulty: u32) -> T {
        arr[(difficulty as usize).min(2)]
    }
}
