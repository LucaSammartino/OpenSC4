// SPDX-License-Identifier: GPL-3.0-or-later
//! nation.md §5: synthetic inputs only; no game files or global process tuning.
use fc4_core::nation::{EconomyDate, EconomyTuning, TuningProperties, FIRST_CURVE_PROPERTY};
use std::collections::BTreeMap;

fn shipped() -> EconomyTuning {
    EconomyTuning::from_properties(Some(&TuningProperties {
        demand_ids: Some(vec![0]),
        curves: BTreeMap::from([(FIRST_CURVE_PROPERTY, vec![0.0, 0.0])]),
        difficulty: Some(vec![0.06, 0.03, 0.01]),
    }))
}

#[test]
fn nation_section5_vector1_shipped_difficulty_and_no_city() {
    let tuning = shipped();
    let date = EconomyDate {
        sim_year: 2000,
        sim_month: 1,
        start_year: 2000,
        start_month: 1,
    };
    for demand in [0, 0x1010, 0x1810, u32::MAX] {
        for (difficulty, expected) in [0.06f32, 0.03, 0.01].into_iter().enumerate() {
            assert_eq!(
                tuning.get_economy_modifier(demand, date, Some(difficulty)),
                Ok(expected)
            );
        }
        assert_eq!(tuning.get_economy_modifier(demand, date, None), Ok(0.0));
    }
}

#[test]
fn nation_section5_vector2_month_before_start_is_unsigned() {
    let date = EconomyDate {
        sim_year: 1999,
        sim_month: 12,
        start_year: 2000,
        start_month: 1,
    };
    assert_eq!(date.unsigned_months(), u32::MAX);
    assert_eq!(shipped().get_economy_modifier(0, date, Some(0)), Ok(0.06));
    // A rising synthetic curve distinguishes unsigned conversion from clamping
    // the negative input to the first point; the shipped flat curve cannot.
    let rising = EconomyTuning::from_properties(Some(&TuningProperties {
        demand_ids: Some(vec![0]),
        curves: BTreeMap::from([(FIRST_CURVE_PROPERTY, vec![0.0, 0.0, 12.0, 1.0])]),
        difficulty: Some(vec![0.06, 0.03, 0.01]),
    }));
    assert_eq!(rising.get_economy_modifier(0, date, None), Ok(1.0));
}
