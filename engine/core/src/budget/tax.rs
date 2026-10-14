// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Property tax (§5.2, §6 `GetTaxIncome(t)`) as a pure computation over a
//! lot source: `ComputeTax` of the budget simulator runs it and stores the
//! result; the plugin's budget shadow (a diagnostic next to the original
//! budget) runs the very same code on the same lots and compares, without
//! touching any budget state.

use crate::x87::{ftol, mul_i64_f32};

use super::tuning::BudgetTuning;
use super::{round_half_even, LotInfo, TAX_GROUPS};

/// The float constant 1/1200 of the tax formula (§5.2).
pub(crate) const INV_1200: f32 = 1.0 / 1200.0;

/// `TaxType[purpose][lotWealth]` (§2); `None` = untaxed. [choice] purposes
/// above 8 are untaxed.
pub fn tax_type(purpose: u32, lot_wealth: u32) -> Option<usize> {
    const T: [[i8; 4]; 9] = [
        [-1, -1, -1, -1], // 0 None
        [-1, 0, 1, 2],    // 1 Residence
        [-1, 3, 4, 5],    // 2 Services
        [-1, -1, 6, 7],   // 3 Office
        [-1, 3, 4, 5],    // 4 Tourism
        [-1, 8, 8, 8],    // 5 Agriculture
        [-1, 9, 9, 9],    // 6 Processing
        [-1, 10, 10, 10], // 7 Manufacturing
        [-1, 11, 11, 11], // 8 HighTech
    ];
    let row = T.get(purpose as usize)?;
    let v = *row.get(lot_wealth as usize)?;
    (v >= 0).then_some(v as usize)
}

/// What `ComputeTax` did with one lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaxLotStatus {
    /// Taxed: added to its group.
    Counted,
    /// No building on the lot.
    NoBuilding,
    /// A building, but the lot is not in a livable state.
    NotLivable,
    /// Livable, but `TaxType[purpose][lotWealth]` is untaxed.
    Untaxed,
}

/// One lot as `ComputeTax` saw it (diagnostics). For a not-livable lot the
/// group and assessed value are what it would have contributed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaxLot {
    pub status: TaxLotStatus,
    /// Tax group (0..11), `None` when untaxed or without a building.
    pub group: Option<usize>,
    /// `assessed` of §5.2 (0 when there is no group).
    pub assessed: i64,
    /// `lotTax` of §5.2 (0 unless counted).
    pub tax: f32,
}

/// The result of one tax scan (§5.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaxScan {
    /// `weightedAssessed[t]`.
    pub weighted_assessed: [i64; TAX_GROUPS],
    /// `groupTax[t]` (float32 running sums).
    pub group_tax: [f32; TAX_GROUPS],
    /// `total` (float32 running sum of the lot taxes in visit order, BQ3:
    /// never formed from the group sums).
    pub total: f32,
    /// Lots visited.
    pub lots: u32,
    /// Lots counted per group.
    pub counted: [u32; TAX_GROUPS],
}

impl TaxScan {
    /// `monthlyTax[t] := _ftol(groupTax[t])`, truncated toward zero to 64
    /// bits. BQ7 (§5.2): never narrowed to 32 bits; only a NaN or a value at
    /// or beyond ±2^63 gives `0x8000000000000000`.
    pub fn monthly_tax(&self) -> [i64; TAX_GROUPS] {
        self.group_tax.map(|g| ftol(f64::from(g)))
    }

    /// `ComputeTax`'s result: `_ftol(total)`, truncated toward zero to 64
    /// bits (§5.2, BQ3). It keeps the fractions the twelve group truncations
    /// drop, so it usually exceeds the sum of [`monthly_tax`](Self::monthly_tax).
    /// BQ7: a total outside the int32 range keeps all 64 bits.
    pub fn total_tax(&self) -> i64 {
        ftol(f64::from(self.total))
    }
}

/// The per-lot assessed value of §5.2:
/// `trunc(float(land + value) × WaningWealth[bWealth − lWealth] × OccWeight[t])`.
fn assessed_value(tuning: &BudgetTuning, lot: &LotInfo, t: usize) -> i64 {
    let land_f = tuning.land_value_curve.eval(f32::from(lot.land_value)) * lot.size_x as f32 * lot.size_z as f32;
    let land = round_half_even(f64::from(land_f)) as i64;
    // [choice] a wealth difference outside 0..2 uses 1.0.
    let waning = lot
        .building_wealth
        .checked_sub(lot.lot_wealth)
        .and_then(|i| tuning.waning_wealth.get(i as usize).copied())
        .unwrap_or(1.0);
    // §5.2 [compat]: left to right, one f32 rounding per product, the 64-bit
    // integers entering exactly (Errata round 3, item 2).
    (mul_i64_f32(land.wrapping_add(lot.building_value), waning) * tuning.occ_weight[t]) as i64
}

/// `ComputeTax` (§5.2) over the lots that `lots` feeds to its visitor, with the
/// difficulty multiplier `mult` and the group rates `rates` (percent). Pure:
/// it reads nothing but its arguments. `on_lot` sees every lot with what was
/// done with it.
///
/// *[compat]* (BQ3): each counted lot's tax is added to the float32 running
/// `total` first, then to its group, lot by lot in visit order. The total
/// depends on that order (its additions round), so it matches the original
/// to the simoleon only when `lots` visits the lot manager's master list in
/// list order (§5.2 "The lot set and its order").
pub fn scan_tax(
    tuning: &BudgetTuning,
    mult: f32,
    rates: &[f32; TAX_GROUPS],
    lots: impl FnOnce(&mut dyn FnMut(&LotInfo)),
    on_lot: &mut dyn FnMut(&LotInfo, &TaxLot),
) -> TaxScan {
    let mut s = TaxScan::default();
    let mut visit = |lot: &LotInfo| {
        s.lots += 1;
        if !lot.has_building {
            on_lot(
                lot,
                &TaxLot {
                    status: TaxLotStatus::NoBuilding,
                    group: None,
                    assessed: 0,
                    tax: 0.0,
                },
            );
            return;
        }
        let group = tax_type(lot.purpose, lot.lot_wealth);
        let assessed = group.map_or(0, |t| assessed_value(tuning, lot, t));
        if !lot.livable {
            on_lot(
                lot,
                &TaxLot {
                    status: TaxLotStatus::NotLivable,
                    group,
                    assessed,
                    tax: 0.0,
                },
            );
            return;
        }
        let Some(t) = group else {
            on_lot(
                lot,
                &TaxLot {
                    status: TaxLotStatus::Untaxed,
                    group: None,
                    assessed: 0,
                    tax: 0.0,
                },
            );
            return;
        };
        s.weighted_assessed[t] = s.weighted_assessed[t].wrapping_add(assessed);
        // §5.2 [compat]: float(assessed) × multiplier × rate × INV1200, left to right.
        let lot_tax = mul_i64_f32(assessed, mult) * rates[t] * INV_1200;
        // BQ3 [compat]: the total first, then the group; each a float32 add.
        s.total += lot_tax;
        s.group_tax[t] += lot_tax;
        s.counted[t] += 1;
        on_lot(
            lot,
            &TaxLot {
                status: TaxLotStatus::Counted,
                group: Some(t),
                assessed,
                tax: lot_tax,
            },
        );
    };
    lots(&mut visit);
    s
}

/// `GetTaxIncome(t)` (§6): `trunc(float(weightedAssessed) × multiplier × rate ×
/// INV1200)`, left to right, one float32 rounding per product.
pub fn tax_income_of(weighted_assessed: i64, mult: f32, rate: f32) -> i64 {
    (mul_i64_f32(weighted_assessed, mult) * rate * INV_1200) as i64
}

/// The difficulty multiplier of `tuning` for `difficulty` (0..2, clamped).
pub fn difficulty_multiplier(tuning: &BudgetTuning, difficulty: u32) -> f32 {
    BudgetTuning::by_difficulty(&tuning.difficulty_tax_multiplier, difficulty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lot(purpose: u32, bw: u32, lw: u32, value: i64) -> LotInfo {
        LotInfo {
            has_building: true,
            livable: true,
            purpose,
            building_wealth: bw,
            lot_wealth: lw,
            building_value: value,
            land_value: 0,
            size_x: 1,
            size_z: 1,
        }
    }

    fn shipped() -> BudgetTuning {
        BudgetTuning {
            difficulty_tax_multiplier: [1.2, 1.1, 1.0],
            waning_wealth: [1.0, 0.75, 0.5],
            land_value_curve: crate::curve::ResponseCurve::from_points(&[(0.0, 0.0), (255.0, 0.0)]),
            ..BudgetTuning::default()
        }
    }

    #[test]
    fn scan_reports_every_lot_and_sums_by_group() {
        let tuning = shipped();
        let mut lots = vec![
            lot(2, 1, 1, 10_000), // CS$ (group 3), counted
            lot(2, 2, 1, 4_000),  // CS$ with waning 0.75 -> 3000
            lot(3, 1, 1, 5_000),  // office at wealth 1: untaxed
            lot(1, 1, 1, 7_000),  // R$, not livable below
        ];
        lots[3].livable = false;
        lots.push(LotInfo::default()); // no building
        let rates = [9.0f32; TAX_GROUPS];
        let mut seen = Vec::new();
        let s = scan_tax(
            &tuning,
            1.0,
            &rates,
            |v| {
                for l in &lots {
                    v(l);
                }
            },
            &mut |_, r| seen.push(*r),
        );
        assert_eq!(s.lots, 5);
        assert_eq!(s.weighted_assessed[3], 13_000);
        assert_eq!(s.counted[3], 2);
        assert_eq!(s.weighted_assessed[0], 0);
        let st: Vec<TaxLotStatus> = seen.iter().map(|r| r.status).collect();
        assert_eq!(
            st,
            [
                TaxLotStatus::Counted,
                TaxLotStatus::Counted,
                TaxLotStatus::Untaxed,
                TaxLotStatus::NotLivable,
                TaxLotStatus::NoBuilding
            ]
        );
        // The not-livable lot reports what it would have added.
        assert_eq!(seen[3].group, Some(0));
        assert_eq!(seen[3].assessed, 7_000);
        assert_eq!(seen[3].tax, 0.0);
        // 9 % of 13,000 / 12 = 97.5 -> monthlyTax 97; GetTaxIncome(t) agrees.
        assert_eq!(s.monthly_tax()[3], 97);
        assert_eq!(tax_income_of(s.weighted_assessed[3], 1.0, 9.0), 97);
        assert_eq!(s.total_tax(), 97);
    }

    /// §5.2 worked example (BQ3): medium (1.1), 9 %, four lots assessed
    /// 104410 and 362120 in R$, then 258840 and 70630 in CS$. The list order
    /// totals 6567.0 (`0x45CD3800`), the reverse order 6566.9995
    /// (`0x45CD37FF`); the lines are R$ 3848 and CS$ 2718 either way.
    #[test]
    fn total_is_a_running_sum_in_lot_order() {
        let tuning = shipped();
        let lots = [
            lot(1, 1, 1, 104_410),
            lot(1, 1, 1, 362_120),
            lot(2, 1, 1, 258_840),
            lot(2, 1, 1, 70_630),
        ];
        let rates = [9.0f32; TAX_GROUPS];
        let run = |order: &[usize]| {
            let mut taxes = Vec::new();
            let s = scan_tax(
                &tuning,
                1.1,
                &rates,
                |v| {
                    for &i in order {
                        v(&lots[i]);
                    }
                },
                &mut |_, r| taxes.push(r.tax.to_bits()),
            );
            (s, taxes)
        };
        let (fwd, taxes) = run(&[0, 1, 2, 3]);
        assert_eq!(taxes, [0x4457_587B, 0x453A_B7D7, 0x4505_76E1, 0x4411_ACA4]);
        assert_eq!(fwd.total.to_bits(), 0x45CD_3800);
        assert_eq!(fwd.total_tax(), 6567);
        let (rev, _) = run(&[3, 2, 1, 0]);
        assert_eq!(rev.total.to_bits(), 0x45CD_37FF);
        assert_eq!(rev.total_tax(), 6566);
        for s in [&fwd, &rev] {
            assert_eq!((s.monthly_tax()[0], s.monthly_tax()[3]), (3848, 2718));
            assert_eq!(s.group_tax[0].to_bits(), fwd.group_tax[0].to_bits());
        }
        // The total is not the sum of the lines (6566).
        let lines: i64 = fwd.monthly_tax().iter().sum();
        assert_eq!(lines, 6566);
    }

    /// BQ7 (§5.2): the total and every group's tax are 64-bit `_ftol`
    /// results; values above 2^31 are not cut to their low 32 bits.
    #[test]
    fn tax_beyond_int32_keeps_64_bits() {
        let mut scan = TaxScan {
            total: 3.0e9,
            ..TaxScan::default()
        };
        scan.group_tax[0] = 5.0e9;
        scan.group_tax[1] = -3.0e9;
        assert_eq!(scan.total_tax(), 3_000_000_000);
        assert_eq!(scan.monthly_tax()[0], 5_000_000_000);
        assert_eq!(scan.monthly_tax()[1], -3_000_000_000);
        // Only NaN and |x| >= 2^63 give 0x8000000000000000.
        scan.total = f32::NAN;
        scan.group_tax[2] = 1.0e19;
        assert_eq!(scan.total_tax(), i64::MIN);
        assert_eq!(scan.monthly_tax()[2], i64::MIN);
    }

    #[test]
    fn tax_income_operation_order() {
        // §15 test 2: (998857 × 1.2f) × 7 × INV1200 = 6992.0, not 6991.
        assert_eq!(tax_income_of(998_857, 1.2, 7.0), 6992);
    }
}
