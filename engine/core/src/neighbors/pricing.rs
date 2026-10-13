// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `GetDealCost` and `GetDealMaxAmount` (§8.1) as pure functions over the
//! two parties' regional-city statistics. All arithmetic is `double`, as the
//! regional city's getters return it (slots 54–63); the curve is evaluated
//! with a `float32` input and output (foundations §4).

/// One party's commodity statistics (`GetDemanded`, `GetExported`,
/// `GetProduced`, `GetImported`, `GetCostPerUnit`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CommodityStats {
    pub demanded: f64,
    pub exported: f64,
    pub produced: f64,
    pub imported: f64,
    pub cost_per_unit: f64,
}

/// The tuning and deal-list inputs of §8.1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceInputs {
    /// The type's base amount (§3).
    pub base: f64,
    /// `0xEA4AD2FF`.
    pub scale: f64,
    /// `0x6A469342` and `0xCA469346`, read as f64 (§12).
    pub min_markup: f64,
    pub max_markup: f64,
    /// The amount of the first deal `FindDeals(type, C)` returns (0 if none).
    pub existing: f64,
}

/// The smallest amount `GetDealCost` prices (§8.1).
pub const MIN_AMOUNT: f64 = 1e-8;

/// `GetDealCost(type, amount, C)` (§8.1) with `buyer` / `seller` already
/// chosen by the type: a `double`, which callers truncate.
pub fn deal_cost(
    buyer: &CommodityStats,
    seller: &CommodityStats,
    amount: f64,
    p: &PriceInputs,
    curve: &dyn Fn(f32) -> f32,
) -> f64 {
    if amount < MIN_AMOUNT {
        return 0.0;
    }
    let need = (p.base / p.scale + 1.0).max(buyer.demanded + buyer.exported);
    let have = MIN_AMOUNT.max(seller.imported + seller.produced);
    let x1 = (1.0 - (need - (buyer.produced + buyer.imported - p.existing))) / amount;
    let x2 = ((have - amount) - (seller.demanded + seller.exported - p.existing)) / have;
    let p_min = seller.cost_per_unit * (1.0 + p.min_markup);
    let p_max = seller.cost_per_unit * (1.0 + p.max_markup);
    let c1 = f64::from(curve(x1 as f32));
    let c2 = f64::from(curve(x2 as f32));
    let raw = p_min + (c1 + c2) / 2.0 * (p_max - p_min);
    // `max(pMin, …)` then `min(pMax, …)`.
    let unit = raw.max(p_min).min(p_max);
    unit * amount
}

/// `GetDealMaxAmount(type, C)` (§8.1) with `buyer` / `seller` already chosen.
pub fn deal_max_amount(buyer: &CommodityStats, seller: &CommodityStats, p: &PriceInputs) -> f64 {
    let need = (p.base / p.scale + 1.0).max(buyer.demanded + buyer.exported);
    let shortfall = p.scale * need - (buyer.produced + buyer.imported - p.existing);
    let surplus = (seller.produced + seller.imported) - (seller.demanded + seller.exported - p.existing);
    0f64.max(shortfall.min(surplus))
}
