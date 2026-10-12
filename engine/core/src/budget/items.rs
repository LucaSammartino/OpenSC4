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
    /// Fraction (property value / 100; 1.0 when the entry is missing).
    pub local_funding: f32,
}

/// `GetBudgetItemInfo` (§6): `None` if the department array is missing or
/// empty; short arrays use the defaults of §2.
pub fn budget_items(host: &dyn BudgetHost, holder: u64) -> Option<Vec<BudgetItem>> {
    let departments = host.get_u32s(holder, prop::DEPARTMENT)?;
    if departments.is_empty() {
        return None;
    }
    let lines = host.get_u32s(holder, prop::LINE).unwrap_or_default();
    let purposes = host.get_u32s(holder, prop::PURPOSE).unwrap_or_default();
    let costs = host.get_i64s(holder, prop::COST).unwrap_or_default();
    let funding = host.get_f32s(holder, prop::LOCAL_FUNDING).unwrap_or_default();
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
                local_funding: funding.get(i).copied().unwrap_or(100.0) / 100.0,
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
    let Some(mut lines) = host.get_u32s(holder, prop::LINE) else {
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
    let Some(mut costs) = host.get_i64s(holder, prop::COST) else {
        return false;
    };
    let Some(slot) = costs.get_mut(index as usize) else {
        return false;
    };
    *slot = cost;
    host.set_i64s(holder, prop::COST, &costs)
}

/// `ChangeBudgetItemLocalFunding` (§6): entry `index` := `fraction × 100`,
/// growing the array to `index + 1` with 0.0 first [compat] (other items that
/// had no entry become 0 % funded).
pub fn change_item_local_funding(host: &dyn BudgetHost, holder: u64, index: u32, fraction: f32) -> bool {
    let mut funding = host.get_f32s(holder, prop::LOCAL_FUNDING).unwrap_or_default();
    let i = index as usize;
    if funding.len() < i + 1 {
        funding.resize(i + 1, 0.0);
    }
    funding[i] = fraction * 100.0;
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
