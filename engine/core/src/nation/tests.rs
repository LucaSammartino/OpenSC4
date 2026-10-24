// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn date(month: i32) -> EconomyDate {
    EconomyDate {
        sim_year: 2000,
        sim_month: month,
        start_year: 2000,
        start_month: 1,
    }
}

#[test]
fn economy_lookup_unsigned_date_and_error_precedence() {
    let mut p = TuningProperties {
        demand_ids: Some(vec![0, 7, 7]),
        curves: BTreeMap::from([
            (FIRST_CURVE_PROPERTY, vec![0.0, 0.0, 12.0, 1.0]),
            (FIRST_CURVE_PROPERTY + 1, vec![0.0, 2.0]),
            (FIRST_CURVE_PROPERTY + 2, vec![0.0, 3.0]),
        ]),
        difficulty: Some(vec![0.25]),
    };
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(7, date(1), None), Ok(3.0));
    assert_eq!(tuning.get_economy_modifier(8, date(7), None), Ok(0.5));
    assert_eq!(date(0).unsigned_months(), u32::MAX);
    assert_eq!(tuning.get_economy_modifier(8, date(0), Some(0)), Ok(1.25));
    assert_eq!(
        tuning.get_economy_modifier(7, date(1), Some(1)),
        Err(NationError::InvalidDifficulty(1))
    );
    p.demand_ids = Some(vec![7]);
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(
        tuning.get_economy_modifier(8, date(1), Some(99)),
        Err(NationError::MissingCurve(8))
    );
    p.demand_ids = Some(vec![]);
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(
        tuning.get_economy_modifier(7, date(1), None),
        Err(NationError::MissingCurve(7))
    );
}

#[test]
fn defaults_empty_curves_explicit_empty_difficulty_and_property_indices() {
    for p in [None, Some(TuningProperties::default())] {
        assert_eq!(
            EconomyTuning::from_properties(p.as_ref()).get_economy_modifier(0, date(1), Some(0)),
            Ok(0.06)
        );
    }
    let mut p = TuningProperties {
        demand_ids: Some((0..13).collect()),
        curves: (0..13)
            .map(|i| (FIRST_CURVE_PROPERTY + i, vec![0.0, i as f32]))
            .collect(),
        difficulty: Some(vec![]),
    };
    let before = p.clone();
    let tuning = EconomyTuning::from_properties(Some(&p));
    for id in 0..13 {
        assert_eq!(tuning.get_economy_modifier(id, date(1), None), Ok(id as f32));
        assert_eq!(
            tuning.get_economy_modifier(id, date(1), Some(0)),
            Err(NationError::InvalidDifficulty(0))
        );
    }
    assert_eq!(p, before);
    assert_eq!(tuning, EconomyTuning::from_properties(Some(&p)));
    p.curves.insert(FIRST_CURVE_PROPERTY, vec![]);
    p.difficulty = Some(vec![0.25]);
    assert_eq!(
        EconomyTuning::from_properties(Some(&p)).get_economy_modifier(0, date(1), Some(0)),
        Ok(0.25)
    );
}

#[test]
fn getters_are_thread_safe_and_difficulty_survives_reload_with_empty_ids() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ProcessDirectories>();
    send_sync::<EconomyTuning>();
    let p = TuningProperties {
        demand_ids: Some(vec![]),
        ..Default::default()
    };
    let tuning = EconomyTuning::reload(Some(&p), vec![0.2, 0.4]);
    assert_eq!(tuning.difficulty_modifiers(), &[0.2, 0.4, 0.0]);
}
