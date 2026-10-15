// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Budget items: the five parallel array properties on a building (§2) and
//! the `GetBudgetItemInfo` / `ChangeBudgetItem*` family of §6.

use super::{prop, BudgetHost};

/// The SDK's `cISC4BudgetSimulator::BudgetItem` (§2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BudgetItem {
    pub department_id: u32,
    pub line_item: u32,
    pub purpose: u32,
    pub cost: i64,
    /// Position in the property arrays.
    pub index: i32,
    /// Fraction: float32(property value × 0.01f), 1.0 when the entry is
    /// missing (budget.md §6 `GetBudgetItemInfo`, RG13).
    pub local_funding: f32,
}

/// The float32 `0.01f` (`0x3C23D70A`) that `GetBudgetItemInfo` multiplies the
/// stored percentage by (budget.md §6, RG13: a multiplication, not `pct/100`).
pub const PERCENT_TO_FRACTION: f32 = 0.01;

/// `GetBudgetItemInfo`'s read of one local-funding entry (RG13):
/// float32(pct × 0.01f), one rounding. The round trip through
/// [`change_item_local_funding`] (float32(q × 100.0f)) is inexact: 0.43 reads
/// back as 0.42999998, 0.79999995 as 0.7999999 (REH §4.2 table).
pub fn local_funding_fraction(pct: f32) -> f32 {
    pct * PERCENT_TO_FRACTION
}

/// `ChangeBudgetItemLocalFunding`'s store (RG13): float32(fraction × 100.0f).
pub fn local_funding_percent(fraction: f32) -> f32 {
    fraction * 100.0
}

/// A vector-helper read of one item array (data-formats §5.2, EX39): a
/// property stored as a scalar has count 0 and gives an **empty** vector, as
/// the original's `vec<…>` helpers do; only the converting getters keep a
/// scalar's value. Two shipped landfill tiles (`0x02F10000`, `0x034E0000`)
/// store the cost `0xEA54D286` as a Sint64 scalar, which the budget simulator
/// reads with `vec<s64>`: no cost entry. *[literal reading]* The rule is applied
/// to all five item arrays (§2 lists each as an array); on shipped data only
/// the cost is ever a scalar. A host that cannot tell (`None`) keeps the value.
fn item_array<T>(host: &dyn BudgetHost, holder: u64, prop: u32, values: Option<Vec<T>>) -> Option<Vec<T>> {
    let mut v = values?;
    if host.property_is_array(holder, prop) == Some(false) {
        v.clear();
    }
    Some(v)
}

/// `GetBudgetItemInfo` (§6): `None` if the department array is missing or
/// empty; short arrays use the defaults of §2. A scalar reads as an empty
/// array ([`item_array`], EX39).
pub fn budget_items(host: &dyn BudgetHost, holder: u64) -> Option<Vec<BudgetItem>> {
    let u32s = |p| item_array(host, holder, p, host.get_u32s(holder, p));
    let departments = u32s(prop::DEPARTMENT)?;
    if departments.is_empty() {
        return None;
    }
    let lines = u32s(prop::LINE).unwrap_or_default();
    let purposes = u32s(prop::PURPOSE).unwrap_or_default();
    let costs = item_array(host, holder, prop::COST, host.get_i64s(holder, prop::COST)).unwrap_or_default();
    let funding = item_array(
        host,
        holder,
        prop::LOCAL_FUNDING,
        host.get_f32s(holder, prop::LOCAL_FUNDING),
    )
    .unwrap_or_default();
    Some(
        departments
            .iter()
            .enumerate()
            .map(|(i, &department_id)| BudgetItem {
                department_id,
                line_item: lines.get(i).copied().unwrap_or(0),
                purpose: purposes.get(i).copied().unwrap_or(0),
                cost: costs.get(i).copied().unwrap_or(0),
                index: i as i32,
                // RG13: × 0.01f (100.0 × 0.01f rounds to exactly 1.0).
                local_funding: local_funding_fraction(funding.get(i).copied().unwrap_or(100.0)),
            })
            .collect(),
    )
}

/// `GetBudgetItemForPurpose` (§6): the first item with that purpose.
pub fn item_for_purpose(host: &dyn BudgetHost, holder: u64, purpose: u32) -> Option<BudgetItem> {
    budget_items(host, holder)?.into_iter().find(|i| i.purpose == purpose)
}

/// `ChangeBudgetItemLine` (§6): sets entry `index` of the line array (only if
/// it exists) and writes the whole array back. Returns whether it was set.
pub fn change_item_line(host: &dyn BudgetHost, holder: u64, index: u32, line: u32) -> bool {
    let Some(mut lines) = item_array(host, holder, prop::LINE, host.get_u32s(holder, prop::LINE)) else {
        return false;
    };
    let Some(slot) = lines.get_mut(index as usize) else {
        return false;
    };
    *slot = line;
    host.set_u32s(holder, prop::LINE, &lines)
}

/// `ChangeBudgetItemCost` (§6), like `change_item_line` for the cost array.
pub fn change_item_cost(host: &dyn BudgetHost, holder: u64, index: u32, cost: i64) -> bool {
    let Some(mut costs) = item_array(host, holder, prop::COST, host.get_i64s(holder, prop::COST)) else {
        return false;
    };
    let Some(slot) = costs.get_mut(index as usize) else {
        return false;
    };
    *slot = cost;
    host.set_i64s(holder, prop::COST, &costs)
}

/// `ChangeBudgetItemLocalFunding` (§6): entry `index` := float32(`fraction ×
/// 100.0f`) (RG13), growing the array to `index + 1` with 0.0 first [compat]
/// (other items that had no entry become 0 % funded).
pub fn change_item_local_funding(host: &dyn BudgetHost, holder: u64, index: u32, fraction: f32) -> bool {
    let mut funding = item_array(
        host,
        holder,
        prop::LOCAL_FUNDING,
        host.get_f32s(holder, prop::LOCAL_FUNDING),
    )
    .unwrap_or_default();
    let i = index as usize;
    if funding.len() < i + 1 {
        funding.resize(i + 1, 0.0);
    }
    funding[i] = local_funding_percent(fraction);
    host.set_f32s(holder, prop::LOCAL_FUNDING, &funding)
}

/// `CopyBudgetItemProperties` (§6): each of the five properties present on
/// `src` is copied (whole array) onto `dst`.
pub fn copy_item_properties(host: &dyn BudgetHost, src: u64, dst: u64) -> bool {
    for p in [
        prop::DEPARTMENT,
        prop::LINE,
        prop::PURPOSE,
        prop::COST,
        prop::LOCAL_FUNDING,
    ] {
        host.copy_property(src, dst, p);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RG13 (REH §4.2 table, budget.md §6): stored as float32(q × 100.0f),
    /// read back as float32(pct × 0.01f), so the round trip is inexact.
    #[test]
    fn local_funding_round_trip_is_times_point_zero_one() {
        let cases: [(u32, u32, u32); 4] = [
            // (q written, stored pct, q read back), as float32 bits
            (0x3EDC_28F6, 43.0f32.to_bits(), 0x3EDC_28F5), // 0.43 -> 0.42999998
            (0x3F4C_CCCC, 0x429F_FFFF, 0x3F4C_CCCB),       // 0.79999995 -> 0.7999999
            (0x3F00_0000, 50.0f32.to_bits(), 0x3F00_0000), // 0.5 survives
            (0x3F99_999A, 0x42F0_0001, 0x3F99_999A),       // 1.2 survives
        ];
        for (q, pct, back) in cases {
            let stored = local_funding_percent(f32::from_bits(q));
            assert_eq!(stored.to_bits(), pct, "q {q:#x}");
            assert_eq!(local_funding_fraction(stored).to_bits(), back, "q {q:#x}");
        }
        assert_eq!(local_funding_fraction(43.0), 0.42999998);
        // Not pct / 100, which is exact here.
        assert_eq!(43.0f32 / 100.0, 0.43);
        // The missing-entry default is exactly 1.0.
        assert_eq!(local_funding_fraction(100.0), 1.0);
    }
}
