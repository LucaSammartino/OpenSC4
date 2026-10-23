// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn qct5_1_to_3_null_slots_and_funding_lists() {
    let mut f = world();
    let s = Subject::Occupant(1);
    for id in 0x1026..=0x1028 {
        assert!(!fill_control(&mut f, 11, id, true, s));
        assert!(fill_control(&mut f, 11, id, true, Subject::Cell(Cell::default())));
    }
    for (id, expected) in [
        (0x1002, vec![]),
        (0x1009, vec![]),
        (0x1018, vec![0x101A, 0x101B, 0x101D, 0x101F]),
        (0x1019, vec![]),
        (0x1021, vec![0x1023, 0x1024, 4, 0x3F80_0000]),
        (0x1022, vec![]),
    ] {
        f.events.clear();
        assert!(fill_control(&mut f, 11, id, true, s));
        assert_eq!(f.properties[&11].2, expected);
        assert!(f.events.contains(&Effect::MoneyChild(11, id + 0x100, "$0".into())));
    }
    // Refresh the literal odd IDs through ordinary lookup: they are neither
    // floats nor substitute hospital controls. Notification refresh calls the
    // selected entry directly, rather than recursively walking its children.
    fill_control(&mut f, 11, 0x1021, true, s);
    f.ids.extend([(11, 0x1021), (12, 4), (13, 0x3F80_0000), (14, 0x1001)]);
    f.tree.insert(12, vec![14]);
    f.reads.insert(Read::FireEfficiency, 0.75);
    assert!(dialog_notification(
        &mut f,
        100,
        s,
        Notification {
            message_type: 3,
            from_dialog: true,
            code: 0x8871_13A3,
            value: 0,
            slider: Some(11),
        }
    ));
    assert!(!f.events.contains(&Effect::Meter(14, 0.75)));
}

fn ranked(pairs: &[(u32, i16, f32)], zone: u32) -> Fake {
    let mut f = world();
    f.zone = zone;
    for (kind, desirability, demand) in pairs {
        f.developers.insert(
            *kind,
            Developer {
                desirability: *desirability,
                demand: *demand,
                ..Developer::default()
            },
        );
    }
    f
}

#[test]
fn qct5_4_to_7_developer_ranking_and_sentinel() {
    let mut f = ranked(&[(0x1010, 20, 10.9), (0x1020, 20, 10.1), (0x1030, 19, 999.0)], 1);
    let lot = f.lot(10);
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0x1010);
    f.developers.get_mut(&0x1020).unwrap().demand = 11.0;
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0x1020);
    for d in f.developers.values_mut() {
        d.desirability = -1001;
    }
    assert!(find_best_developer(&f, &lot).is_none());
    f = ranked(&[(0x4400, 0, 0.0), (0x4300, 0, 0.0)], 8);
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0x4400);
    f = ranked(&[(0x1010, -1000, -1000.0)], 1);
    assert!(find_best_developer(&f, &lot).is_none());
    f.developers.get_mut(&0x1010).unwrap().demand = -999.9;
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0x1010);
    // Signed16 ranking beats demand; negative fractional demand truncates to 0.
    f = ranked(&[(0x1010, -1, -0.9), (0x1020, -1, 0.9), (0x1030, -2, 100.0)], 1);
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0x1010);
}

#[test]
fn developer_all_zone_orders_and_developed_record_path() {
    for (zone, kinds) in [
        (1, vec![0x1010, 0x1020, 0x1030]),
        (2, vec![0x1010, 0x1020, 0x1030]),
        (3, vec![0x1010, 0x1020, 0x1030]),
        (4, vec![0x3110, 0x3120, 0x3130, 0x3320, 0x3330]),
        (5, vec![0x3110, 0x3120, 0x3130, 0x3320, 0x3330]),
        (6, vec![0x3110, 0x3120, 0x3130, 0x3320, 0x3330]),
        (7, vec![0x4100]),
        (8, vec![0x4400, 0x4300]),
        (9, vec![0x4200, 0x4400, 0x4300]),
    ] {
        let mut f = ranked(&kinds.iter().map(|k| (*k, 1, 1.0)).collect::<Vec<_>>(), zone);
        for expected in kinds {
            assert_eq!(find_best_developer(&f, &f.lot(10)).unwrap().0, expected);
            f.developers.remove(&expected);
        }
        assert!(find_best_developer(&f, &f.lot(10)).is_none());
    }
    let mut f = ranked(&[(0x1010, 100, 1000.0), (0xCAFE, -2000, -2000.0)], 1);
    let mut lot = f.lot(10);
    lot.building_type = 42;
    f.development.insert(42, BuildingDevelopment { purpose: 5, wealth: 3 });
    f.developer_types.insert((5, 3), 0xCAFE);
    assert_eq!(find_best_developer(&f, &lot).unwrap().0, 0xCAFE);
    f.development.get_mut(&42).unwrap().purpose = 0;
    assert!(find_best_developer(&f, &lot).is_none());
    lot.building_type = 0;
    f.zone = 10;
    assert!(find_best_developer(&f, &lot).is_none());
}

#[test]
fn qct5_8_9_wealth_geometry_refresh_and_independent_fraction() {
    let mut f = world();
    let s = Subject::Occupant(1);
    let rect = MeterWindow {
        left: 10,
        top: 20,
        right: 100,
        bottom: 30,
        reference_left: None,
    };
    f.meters.insert(11, rect);
    f.lots.get_mut(&10).unwrap().building_type = 42;
    f.lots.get_mut(&10).unwrap().wealth = 1;
    f.development.insert(42, BuildingDevelopment { purpose: 1, wealth: 1 });
    for _ in 0..2 {
        fill_control(&mut f, 11, 0x1053, true, s);
        assert_eq!(
            f.meters[&11],
            MeterWindow {
                left: 69,
                reference_left: Some(10),
                ..rect
            }
        );
        assert!(f.events.contains(&Effect::Meter(11, 1.0)));
    }
    f.events.clear();
    f.development.get_mut(&42).unwrap().wealth = 3;
    fill_control(&mut f, 11, 0x1053, true, s);
    assert_eq!(f.meters[&11].left, 10);
    assert!(f.events.contains(&Effect::Meter(11, 1.0 / 3.0)));
    f.meters.clear();
    f.events.clear();
    fill_control(&mut f, 11, 0x1053, true, s);
    assert!(!f.events.iter().any(|e| matches!(e, Effect::MeterGeometry(..))));
    f.development.get_mut(&42).unwrap().wealth = 0;
    fill_control(&mut f, 11, 0x1053, true, s);
    assert!(f.events.contains(&Effect::Meter(11, 1.0)));
    f.lots.clear();
    assert!(fill_control(&mut f, 11, 0x1053, true, s));
}

#[test]
fn qct5_10_commute_unsigned_zero_substitution_and_thresholds() {
    let mut f = ranked(&[(0x1010, 0, 0.0)], 1);
    let d = f.developers.get_mut(&0x1010).unwrap();
    d.curves.insert(
        (6, 0),
        ResponseCurve {
            knots: vec![(0.0, 0.9), (255.0, 255.9)],
            slopes: vec![1.0],
        },
    );
    d.thresholds.insert(6, (129, 200));
    for (sample, index, auxiliary) in [
        (0, 0, Some(127)),
        (128, 2, Some(-128)),
        (255, 0, Some(127)),
        (129, 1, None),
    ] {
        f.commute = sample;
        assert_eq!(
            developer_rating(&f, &f.lot(10), 6),
            Some(SubjectiveRating { index, auxiliary })
        );
        f.events.clear();
        assert!(fill_control(&mut f, 11, 0x1057, true, Subject::Occupant(1)));
        assert!(f.events.contains(&Effect::Text(
            11,
            loc(&f, SubjectiveRating { index, auxiliary }.label())
        )));
    }
    f.developers.get_mut(&0x1010).unwrap().curves.clear();
    assert_eq!(developer_rating(&f, &f.lot(10), 6).unwrap().index, 1);
}

#[test]
fn subjective_fillers_select_and_evaluate_developer_curves() {
    let mut f = ranked(&[(0x1010, 1, 1.0)], 1);
    let d = f.developers.get_mut(&0x1010).unwrap();
    for i in 0..6 {
        d.thresholds.insert(i, (10, 20));
        for part in if i == 0 { vec![0, 1, 3] } else { vec![0] } {
            d.curves
                .insert((i, part), constant_curve(if i == 0 { 4.0 } else { 20.0 }));
        }
    }
    for (id, label) in [
        (0x1054, 0x0A5F_E260),
        (0x1075, 0x4A5F_E266),
        (0x1059, 0x4A5F_E266),
        (0x105A, 0x4A5F_E266),
        (0x1055, 0x4A5F_E266),
        (0x1074, 0x4A5F_E266),
    ] {
        fill_control(&mut f, 11, id, true, Subject::Occupant(1));
        assert!(f.events.contains(&Effect::Text(11, loc(&f, label))));
    }
}

#[test]
fn qct5_11_constructed_curves_clamp_and_use_stored_slopes() {
    let curve = ResponseCurve {
        knots: vec![(0.0, 0.0), (10.0, 100.0), (20.0, 0.0)],
        slopes: vec![10.0, -10.0],
    };
    for (input, output) in [
        (-5.0, 0),
        (0.0, 0),
        (5.0, 50),
        (10.0, 100),
        (15.0, 50),
        (20.0, 0),
        (25.0, 0),
    ] {
        assert_eq!(goofy_stat(&curve, input), Some(output));
    }
    let stored = ResponseCurve {
        knots: vec![(0.0, 0.0), (10.0, 100.0)],
        slopes: vec![3.0],
    };
    assert_eq!(stored.evaluate(5.0), Some(15.0));
    let precision = ResponseCurve {
        knots: vec![(0.0, 16_777_216.0), (10.0, 0.0)],
        slopes: vec![1.0],
    };
    assert_eq!(precision.evaluate(1.0), Some(16_777_217.0));
    let mut f = world();
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA60_E745, Property::Curve(curve));
    f.reads.insert(Read::MayorRating, 15.0);
    fill_control(&mut f, 11, 0x106B, true, Subject::Occupant(1));
    assert!(f.events.contains(&Effect::Text(11, "50".into())));
}

#[test]
fn qct5_12_airports_use_one_complete_resource_key() {
    let mut f = world();
    f.resources.insert(
        LOST_LUGGAGE_KEY,
        BTreeMap::from([(0xAA60_E745, Property::Curve(constant_curve(37.9)))]),
    );
    for cell in [Cell { x: 1, z: 2 }, Cell { x: 9, z: 8 }] {
        assert_eq!(airport_lost_luggage(&f, cell), Some(37));
        assert!(fill_control(&mut f, 11, 0x1014, true, Subject::Cell(cell)));
        assert!(f.events.contains(&Effect::Text(11, "37".into())));
    }
    f.resources.get_mut(&LOST_LUGGAGE_KEY).unwrap().clear();
    assert_eq!(airport_lost_luggage(&f, Cell::default()), Some(256));
    f.resources.clear();
    assert!(!fill_control(&mut f, 11, 0x1014, true, Subject::Occupant(1)));
}

#[test]
fn qct5_13_14_neighbor_priority_scripts_counts_and_open_rows() {
    let mut f = Fake::default();
    let mut o = Occupant {
        network_interface: true,
        network_types: vec![0, 4],
        ..Default::default()
    };
    f.neighbors.push((0, 5));
    assert_eq!(neighbor_plan(&f, &o).unwrap().script.instance, 0x4A56_6D6E);
    o.network_interface = false;
    o.pole_interface = true;
    f.neighbors = vec![(0, 4), (3, 4)];
    let plan = neighbor_plan(&f, &o).unwrap();
    assert_eq!(plan.script, Script::query(0x8A5E_7BD2));
    assert_eq!(plan.rows[1].offset, 0x200);
    f.objects.insert(1, o.clone());
    assert_eq!(
        tool(Mode::Query).do_neighbor_dialog(&mut f, 1),
        Err(Open::NeighborDialogCt14)
    );
    assert!(f.events.contains(&Effect::NeighborPlan(plan)));
    assert!(f.scripts.is_empty()); // CT14 row writers remain unavailable.
    for (network, kind, one, two) in [
        (11, 10, 0xCA56_6F94, 0xAA5E_14CC),
        (0, 2, 0xCA56_6F94, 0xAA5E_14CC),
        (6, 8, 0xCA56_6F94, 0xAA5E_14CC),
        (12, 12, 0xCA56_6F94, 0xAA5E_14CC),
        (2, 3, 0xCA56_6F94, 0xAA5E_14CC),
        (7, 6, 0xEA56_6A49, 0x4A5E_7ED3),
        (8, 7, 0xEA56_6A49, 0x4A5E_7ED3),
        (9, 11, 0xEA56_6A49, 0x4A5E_7ED3),
        (1, 1, 0xEA56_6A49, 0x4A5E_7ED3),
        (4, 5, 0x4A56_6D6E, 0x2A5E_7490),
    ] {
        o.network_interface = true;
        o.network_types = vec![network];
        for count in 0..=4 {
            f.neighbors = (0..count).map(|dir| (dir, kind)).collect();
            let result = neighbor_plan(&f, &o);
            if count == 1 || count == 2 {
                assert_eq!(result.unwrap().script.instance, if count == 1 { one } else { two });
            } else {
                assert!(result.is_none());
            }
        }
    }
    o.cell = Cell { x: 10, z: 10 };
    assert!(neighbor_plan(&f, &o).is_none());
}

#[test]
fn qct5_15_debug_setting_requires_all_live_keys() {
    let mut f = Fake {
        keys: vec![0x10, 0x11, 0x12],
        ..Default::default()
    };
    assert!(!f.debug_query_enabled());
    f.debug = true;
    f.keys.pop();
    assert!(!f.debug_query_enabled());
    f.keys.push(0x12);
    assert!(f.debug_query_enabled());
    for key in [0x10, 0x11, 0x12] {
        f.keys = vec![0x10, 0x11, 0x12];
        f.keys.retain(|k| *k != key);
        assert!(!f.debug_query_enabled());
    }
}

#[test]
fn qct5_16_cell_writer_order_and_terrain_building_fallback() {
    let mut f = world();
    f.terrain = Some(Vec3::default());
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0x2A49_9F85, Property::U32(77));
    let out = tool(Mode::Query).describe_pick(&mut f, Pixel::default(), true);
    assert_eq!(f.scripts, vec![Script::query(77)]); // terrain miss-pick found lot building
    let c = Cell::default();
    let phases: Vec<_> = f
        .events
        .iter()
        .filter_map(|e| if let Effect::DebugPhase(p) = e { Some(*p) } else { None })
        .collect();
    assert_eq!(
        phases,
        vec![
            DebugPhase::ResidentialQueryData(1),
            DebugPhase::Building(1),
            DebugPhase::Population(c),
            DebugPhase::Demand(c),
            DebugPhase::EqHq(c),
            DebugPhase::Cell(c)
        ]
    );
    assert!(out.open.contains(&Open::DebugWritersCt14));
    assert_eq!(out.extra, "Query extra: none"); // ordering is known; bodies remain open.
}
