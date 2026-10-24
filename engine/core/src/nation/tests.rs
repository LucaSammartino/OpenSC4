// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Mutex;

fn date(month: i32) -> EconomyDate {
    EconomyDate {
        sim_year: 2000,
        sim_month: month,
        start_year: 2000,
        start_month: 1,
    }
}

fn properties(ids: &[u32], curves: &[&[f32]]) -> TuningProperties {
    TuningProperties {
        demand_ids: Some(ids.to_vec()),
        curves: curves
            .iter()
            .enumerate()
            .map(|(i, values)| (FIRST_CURVE_PROPERTY + i as u32, values.to_vec()))
            .collect(),
        difficulty: Some(vec![0.06, 0.03, 0.01]),
    }
}

#[test]
fn demand_lookup_fallback_unsigned_date_and_rounding() {
    let p = properties(&[0, 7], &[&[0.0, 0.0, 12.0, 1.0], &[0.0, 0.1, 3.0, 1.1]]);
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(999, date(7), None), Ok(0.5));
    let curve = ResponseCurve::from_flat(&p.curves[&(FIRST_CURVE_PROPERTY + 1)]).unwrap();
    let rounded = curve.eval(1.0);
    assert_eq!(
        tuning.get_economy_modifier(7, date(2), Some(0)).unwrap().to_bits(),
        (rounded + 0.06f32).to_bits()
    );
    assert_eq!(date(0).unsigned_months(), u32::MAX);
    assert_eq!(tuning.get_economy_modifier(999, date(0), None), Ok(1.0));
    assert_eq!(tuning.get_economy_modifier(999, date(0), Some(2)), Ok(1.01));
    let year_boundary = EconomyDate {
        sim_year: 2001,
        sim_month: 1,
        start_year: 2000,
        start_month: 12,
    };
    assert_eq!(year_boundary.unsigned_months(), 1);
    let overflow = EconomyDate {
        sim_year: i32::MIN,
        sim_month: 0,
        start_year: i32::MAX,
        start_month: 0,
    };
    assert_eq!(overflow.unsigned_months(), 12);
}

#[test]
fn missing_empty_short_and_duplicate_tuning() {
    for input in [None, Some(TuningProperties::default())] {
        let tuning = EconomyTuning::from_properties(input.as_ref());
        assert_eq!(tuning.get_economy_modifier(4, date(1), Some(0)), Ok(0.06));
    }
    let empty = properties(&[], &[]);
    let tuning = EconomyTuning::from_properties(Some(&empty));
    assert_eq!(tuning.difficulty_modifiers(), &[0.0; 3]);
    assert_eq!(
        tuning.get_economy_modifier(0, date(1), None),
        Err(NationError::MissingCurve(0))
    );

    let mut p = properties(&[0, 7, 7], &[&[0.0, 1.0], &[0.0, 2.0], &[0.0, 3.0]]);
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(7, date(1), None), Ok(3.0));
    p.curves.remove(&(FIRST_CURVE_PROPERTY + 2));
    p.difficulty = None;
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(7, date(1), Some(1)), Ok(0.03));
    p.curves.insert(FIRST_CURVE_PROPERTY + 2, vec![5.0]);
    p.difficulty = Some(vec![0.5]);
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(7, date(1), None), Ok(0.0));
    assert_eq!(tuning.get_economy_modifier(7, date(1), Some(0)), Ok(0.5));
    assert_eq!(
        tuning.get_economy_modifier(7, date(1), Some(1)),
        Err(NationError::InvalidDifficulty(1))
    );
    // No city: even an empty difficulty vector is never indexed.
    p.difficulty = Some(vec![]);
    assert_eq!(
        EconomyTuning::from_properties(Some(&p)).get_economy_modifier(7, date(1), None),
        Ok(0.0)
    );
    let p = properties(&[7], &[&[0.0, 4.0]]);
    assert_eq!(
        EconomyTuning::from_properties(Some(&p)).get_economy_modifier(8, date(1), None),
        Err(NationError::MissingCurve(8))
    );
}

#[test]
fn property_indices_are_plain_addition_and_inputs_are_deterministic() {
    let ids: Vec<_> = (0..13).collect();
    let mut p = properties(&ids, &[]);
    for i in 0..13 {
        p.curves.insert(FIRST_CURVE_PROPERTY + i, vec![0.0, i as f32]);
    }
    let copy = p.clone();
    let a = EconomyTuning::from_properties(Some(&p));
    let b = EconomyTuning::from_properties(Some(&copy));
    assert_eq!(a, b);
    for id in ids {
        assert_eq!(a.get_economy_modifier(id, date(5), None), Ok(id as f32));
    }
    assert_eq!(p, copy); // Loading does not mutate the source properties.
}

#[derive(Default)]
struct Messages(Vec<(bool, u32)>);
impl MessageSubscriptions for Messages {
    fn subscribe(&mut self, message: u32) {
        self.0.push((true, message));
    }
    fn unsubscribe(&mut self, message: u32) {
        self.0.push((false, message));
    }
}

#[derive(Clone, Debug)]
struct Region {
    id: usize,
    calls: Arc<Mutex<Vec<(usize, bool)>>>,
}
impl RegionActivation for Region {
    fn activate(&mut self) {
        self.calls.lock().unwrap().push((self.id, true));
    }
    fn deactivate(&mut self) {
        self.calls.lock().unwrap().push((self.id, false));
    }
}

fn catalog() -> RegionCatalog<Region> {
    let calls = Arc::new(Mutex::new(Vec::new()));
    RegionCatalog {
        region_directory: "Regions".into(),
        download_directory: "Downloads".into(),
        regions: (0..2)
            .map(|id| Region {
                id,
                calls: Arc::clone(&calls),
            })
            .collect(),
        active_index: Some(0),
    }
}

#[test]
fn catalog_getters_and_ordered_activation() {
    let mut nation = Nation::new(catalog()).unwrap();
    assert_eq!(nation.get_region_directory(), "Regions");
    assert_eq!(nation.get_region_download_directory(), "Downloads");
    assert_eq!(nation.get_region_count(), 2);
    assert_eq!(nation.get_region(1).unwrap().id, 1);
    let calls = Arc::clone(&nation.get_active_region().unwrap().calls);
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert!(matches!(nation.get_region(2), Err(NationError::InvalidRegionIndex(2))));
    assert_eq!(nation.set_active_region(2), Err(NationError::InvalidRegionIndex(2)));
    assert!(calls.lock().unwrap().is_empty());
    assert_eq!(nation.get_active_region_index(), Some(0));
    nation.set_active_region(1).unwrap();
    assert_eq!(nation.get_active_region_index(), Some(1));
    assert_eq!(nation.get_active_region().unwrap().id, 1);
    nation.set_active_region(1).unwrap();
    assert_eq!(*calls.lock().unwrap(), [(0, false), (1, true), (1, false), (1, true)]);
    let mut invalid = catalog();
    invalid.active_index = Some(2);
    assert!(matches!(Nation::new(invalid), Err(NationError::InvalidRegionIndex(2))));
    let mut inactive = catalog();
    inactive.active_index = None;
    let mut nation = Nation::new(inactive).unwrap();
    assert!(nation.get_active_region().is_none());
    nation.set_active_region(1).unwrap();
    assert_eq!(*nation.get_region(1).unwrap().calls.lock().unwrap(), [(1, true)]);
    let mut empty = catalog();
    empty.regions.clear();
    empty.active_index = None;
    let nation = Nation::new(empty).unwrap();
    assert_eq!(nation.get_region_count(), 0);
    assert!(nation.get_active_region().is_none());
}

#[test]
fn lifecycle_messages_and_once_per_process_tuning() {
    let mut nation = Nation::new(catalog()).unwrap();
    let process = ProcessEconomy::default();
    let mut messages = Messages::default();
    assert_eq!(
        nation.get_economy_modifier(0, date(1), None),
        Err(NationError::NotInitialized)
    );
    nation.shutdown(&mut messages);
    assert!(messages.0.is_empty());
    nation.init(&process, || None, &mut messages);
    nation.init(&process, || panic!("must not reload"), &mut messages);
    assert_eq!(messages.0, SUBSCRIPTIONS.map(|id| (true, id)));
    for id in [POST_CITY_INIT, PRE_CITY_SHUTDOWN, 0, 0xFFFF_FFFF] {
        assert_eq!(nation.do_message(id), Ok(MessageOutcome::Ignored));
    }
    assert_eq!(
        nation.do_message(RESET_TUTORIAL_CITIES),
        nation.reset_tutorial_cities().map(|()| MessageOutcome::TutorialReset)
    );
    nation.shutdown(&mut messages);
    nation.shutdown(&mut messages);
    assert_eq!(messages.0.len(), 6);
    assert_eq!(&messages.0[3..], &SUBSCRIPTIONS.map(|id| (false, id)));
    nation.init(&process, || panic!("shutdown must not clear tuning"), &mut messages);
    let mut second = Nation::new(catalog()).unwrap();
    second.init(
        &process,
        || panic!("new nation must not reload process tuning"),
        &mut Messages::default(),
    );
    assert!(Arc::ptr_eq(
        nation.tuning.as_ref().unwrap(),
        second.tuning.as_ref().unwrap()
    ));
    assert_eq!(nation.get_economy_modifier(9, date(1), Some(0)), Ok(0.06));
    assert_eq!(nation.get_region_count(), 2);
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert_eq!(nation.get_region_directory(), "Regions");
}

#[test]
fn process_tuning_and_getters_are_thread_safe() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Nation<Region>>();
    send_sync::<ProcessEconomy>();
    let process = ProcessEconomy::default();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| process.load_once(|| None));
        let b = scope.spawn(|| process.load_once(|| None));
        assert!(Arc::ptr_eq(&a.join().unwrap(), &b.join().unwrap()));
    });
}

#[test]
fn unsettled_filesystem_operations_fail_without_catalog_mutation() {
    let mut nation = Nation::new(catalog()).unwrap();
    assert_eq!(
        nation.discover_regions(),
        Err(NationError::Unsupported(UnsupportedOperation::DiscoverRegions))
    );
    assert_eq!(
        nation.get_index_of_current_region_setting_in_preferences(),
        Err(NationError::Unsupported(UnsupportedOperation::PreferenceLookup))
    );
    assert_eq!(
        nation.delete_region(0),
        Err(NationError::Unsupported(UnsupportedOperation::DeleteRegion))
    );
    let region = nation.get_region(1).unwrap().clone();
    assert_eq!(
        nation.add_new_region(region),
        Err(NationError::Unsupported(UnsupportedOperation::AddNewRegion))
    );
    assert_eq!(
        nation.reset_tutorial_cities(),
        Err(NationError::Unsupported(UnsupportedOperation::ResetTutorialCities))
    );
    assert_eq!(nation.get_region_count(), 2);
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert!(nation.get_region(0).unwrap().calls.lock().unwrap().is_empty());
}
