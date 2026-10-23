// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn network(flags: u32, sides: u32, connections: u8) -> Network {
    Network {
        flags,
        sides,
        connections,
    }
}
fn cells(a: i32, b: i32, c: i32, d: i32) -> Rect {
    Rect {
        x0: a,
        z0: b,
        x1: c,
        z1: d,
    }
}

#[test]
fn ct16_picker_shapes_flags_and_failure_outputs() {
    // Includes CT16-a/b/c and both alignments of every wide network.
    for (kind, cfg) in [(0, 0xEBF9_A440), (2, 0x8BF9_B356), (6, 0xABF9_A45C), (12, 0x2BF9_B222)] {
        for (sides, rotation, rect, p) in if kind == 0 {
            vec![
                (0x00020002, 1, cells(10, 20, 11, 20), point(168.0, 320.0)),
                (0x02000200, 2, cells(10, 20, 10, 21), point(160.0, 328.0)),
            ]
        } else {
            vec![
                (0x00020402, 1, cells(10, 19, 11, 20), point(168.0, 312.0)),
                (0x00020002, 1, cells(10, 20, 11, 21), point(168.0, 328.0)),
                (0x02000200, 2, cells(10, 20, 11, 21), point(168.0, 312.0)),
                (0x02000204, 2, cells(9, 20, 10, 21), point(152.0, 328.0)),
            ]
        } {
            let mut h = Fake::default();
            h.networks.insert((10, 20), network(1 << kind, sides, 0));
            let pick = pick_toll(&mut h, point(163.0, 325.0));
            assert!(pick.coverage.valid);
            assert_eq!(
                (
                    pick.configuration,
                    pick.rotation,
                    pick.coverage.rect,
                    pick.coverage.point
                ),
                (cfg, rotation, rect, p)
            );
            assert!(h
                .calls
                .iter()
                .all(|c| matches!(c, Call::TollNetwork(_, _, 0x1045, true))));
        }
    }
    for (flags, sides) in [
        (1, 0x00020001),
        (1, 0x00020003),
        (1, 0x00000202),
        (1, 0x00020202),
        (0x41, 0x00020002),
        (8, 0x00020002),
    ] {
        let mut h = Fake::default();
        h.networks.insert((10, 20), network(flags, sides, 0));
        assert!(!pick_toll(&mut h, point(160.0, 320.0)).coverage.valid);
    }
    // Bits outside 1F4F are ignored by the picker, but 400000 fails the search.
    let mut h = Fake::default();
    h.networks.insert((10, 20), network(0x400041 & !0x40, 0x00020002, 0));
    let pick = pick_toll(&mut h, point(160.0, 320.0));
    assert!(!pick.coverage.valid);
    assert_eq!((pick.rotation, pick.coverage.rect), (1, cells(10, 20, 11, 20)));
    h.networks.clear();
    let pick = pick_toll(&mut h, point(160.0, 320.0));
    assert_eq!(
        (
            pick.configuration,
            pick.rotation,
            pick.coverage.rect,
            pick.coverage.point
        ),
        (0xEBF9_A440, 2, cells(10, 20, 10, 21), point(160.0, 328.0))
    );
}

#[test]
fn ct16_depth_two_independent_branches_and_cycles() {
    let mut h = Fake::default();
    h.networks.insert((0, 0), network(1, 0, 4));
    h.networks.insert((1, 0), network(1, 0, 4));
    h.networks.insert((2, 0), network(0x400001, 0, 4));
    assert!(!seek_compatible(&mut h, 0, 0, 0, 0));
    h.networks.insert((2, 0), network(1, 0, 4));
    h.networks.insert((3, 0), network(0x400001, 0, 0));
    h.calls.clear();
    assert!(seek_compatible(&mut h, 0, 0, 0, 0));
    assert!(!h.calls.iter().any(|c| matches!(c, Call::TollNetwork(3, 0, ..))));
    h.networks.insert((1, 0), network(1, 0, 1));
    h.calls.clear();
    assert!(seek_compatible(&mut h, 0, 0, 0, 0));
    assert_eq!(
        h.calls
            .iter()
            .filter(|c| **c == Call::TollNetwork(0, 0, 0x1045, true))
            .count(),
        2
    );
    h.networks.insert((1, 0), network(0x41, 0, 0));
    assert!(!seek_compatible(&mut h, 0, 0, 0, 0));
    h.networks.clear();
    assert!(seek_compatible(&mut h, 0, 0, 0, 0));
}

#[test]
fn ct2_flushes_after_processing_on_miss_repeat_and_null() {
    let mut h = Fake {
        hit: None,
        ..Fake::default()
    };
    let mut t = tool(&mut h);
    for _ in 0..2 {
        h.calls.clear();
        assert!(t.mouse_move(&mut h, mouse(), 0));
        assert_eq!(h.calls.last(), Some(&Call::Effect(Effect::FlushInputQueue)));
        assert_eq!(effects(&h).iter().filter(|e| **e == Effect::FlushInputQueue).count(), 1);
    }
    h.wm = false;
    h.calls.clear();
    assert!(t.mouse_move(&mut h, mouse(), 0));
    assert!(!effects(&h).contains(&Effect::FlushInputQueue));
}

#[test]
fn ct22_finished_clears_then_recalculates_without_other_mutations() {
    let mut h = Fake {
        visual: true,
        ..Fake::default()
    };
    let mut t = tool(&mut h);
    for raised in [true, false] {
        t.coverage_raised = raised;
        h.calls.clear();
        assert!(!t.message(&mut h, 0x49D5_B83F));
        assert!(!t.coverage_raised);
        assert_eq!(effects(&h), vec![Effect::RecalculateCoverage]);
        assert_eq!((t.ring, t.plops_left, t.configuration), (Some(299), 1000, 10));
    }
    t.view = false;
    t.coverage_raised = true;
    h.calls.clear();
    t.message(&mut h, 0x49D5_B83F);
    assert!(!t.coverage_raised);
    assert!(h.calls.is_empty());
}

#[test]
fn ct23_clearing_gates_commit_without_effects_and_exact_zone_flags() {
    for failure in 0..4 {
        let mut h = Fake::default();
        let mut t = tool(&mut h);
        t.cost = 500;
        if failure == 0 {
            h.demolition.0 = false;
        }
        if failure == 1 {
            h.zone_results = [false].into();
        }
        if failure == 2 {
            h.zone_results = [true, false].into();
        }
        let result = t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false);
        assert_eq!(result.built, failure == 3);
        let zones: Vec<_> = h
            .calls
            .iter()
            .filter_map(|c| if let Call::Zone(z) = c { Some(z) } else { None })
            .collect();
        assert_eq!(zones.len(), [0, 1, 2, 2][failure]);
        for (i, z) in zones.iter().enumerate() {
            assert_eq!(
                (
                    z.commit,
                    z.skip_funds,
                    z.check_slope,
                    z.skip_size,
                    z.check_lot_inside,
                    z.outputs
                ),
                (true, i == 1, true, true, false, [None; 3])
            );
        }
        if failure < 3 {
            assert!(!h.calls.iter().any(|c| matches!(c, Call::Lot(..) | Call::Build(..))));
            assert!(effects(&h).is_empty());
        }
    }
}

fn visual_tool() -> (Fake, PlaceLot) {
    let mut h = Fake {
        visual: true,
        ..Fake::default()
    };
    let t = tool(&mut h);
    (h, t)
}

#[test]
fn ct17_attributes_validity_visibility_and_hidden_cleanup() {
    let (mut h, mut t) = visual_tool();
    t.radius = 32.0;
    t.constraint_effects = [vec![210], vec![211]];
    h.running.extend([299, 210, 211]);
    for (valid, highlight) in [(true, 3), (false, 4)] {
        h.calls.clear();
        t.feedback(&mut h, valid, true, point(40.0, 40.0));
        assert!(effects(&h).contains(&Effect::OccupantDraw(
            90,
            5,
            Attributes {
                flag: 3,
                highlight,
                rgb: [1.0; 3],
                alpha: 0.4
            }
        )));
        assert!(h.calls.contains(&Call::EffectCreate(
            if valid {
                "building_footprint_ok"
            } else {
                "building_footprint_notok"
            }
            .into()
        )));
        assert_eq!(t.cursor_id, if valid { 0x417E_E36B } else { CURSOR });
    }
    h.running.extend(t.preview_effects.iter().flatten());
    h.calls.clear();
    let old = t.preview_effects;
    t.feedback(&mut h, false, false, point(40.0, 40.0));
    let e = effects(&h);
    assert!(e.contains(&Effect::GridVisible(false)));
    for token in [Some(299), old[0], old[1]].into_iter().flatten() {
        assert!(e.contains(&Effect::StopEffect(token, 0)));
    }
    assert!(e.contains(&Effect::RemovePreview(90)));
    assert!(e.contains(&Effect::OccupantDraw(
        90,
        5,
        Attributes {
            flag: 2,
            highlight: 0,
            rgb: [1.0; 3],
            alpha: 0.0
        }
    )));
    assert!(t.constraint_effects.iter().all(Vec::is_empty));
    for token in [210, 211] {
        assert!(e.contains(&Effect::ReleaseCreatedEffect(token)));
    }
}

#[test]
fn ct17_centered_model_and_prop_transforms() {
    let (mut h, mut t) = visual_tool();
    t.previews = [90, 91]
        .into_iter()
        .map(|token| Preview {
            token,
            local: Vec3 { x: 3.0, y: 9.0, z: 5.0 },
            rotation: 1,
            registered: false,
            shown_rotation: 0,
            prop: token == 91,
        })
        .collect();
    for (rotation, x, z, orientation) in [
        (0, 97.0, 195.0, 1),
        (1, 105.0, 197.0, 2),
        (2, 103.0, 205.0, 3),
        (3, 95.0, 203.0, 0),
    ] {
        t.rotation = rotation;
        h.calls.clear();
        t.feedback(
            &mut h,
            true,
            true,
            Vec3 {
                x: 100.0,
                y: 7.0,
                z: 200.0,
            },
        );
        for token in [90, 91] {
            assert!(effects(&h).contains(&Effect::OccupantTransform(token, Vec3 { x, y: 7.0, z }, orientation)));
            if rotation != 0 {
                assert!(effects(&h).contains(&Effect::OrientationMessage(token, 0x99EF_1145, 0)));
            }
        }
    }
}

#[test]
fn ct17_constraint_rotations_cells_colours_reuse_and_surplus() {
    let (mut h, mut t) = visual_tool();
    h.cfg.as_mut().unwrap().size = [4, 3];
    h.entries = [5, 6]
        .into_iter()
        .map(|kind| Entry {
            kind,
            position: point(-8.0, -8.0),
            maximum: point(24.0, 8.0),
            ..Entry::default()
        })
        .collect();
    for (rotation, param) in [
        (0, vec![12.0, 22.0, 14.0, 23.0]),
        (1, vec![10.0, 22.0, 11.0, 24.0]),
        (2, vec![10.0, 20.0, 12.0, 21.0]),
        (3, vec![12.0, 20.0, 13.0, 22.0]),
    ] {
        t.rotation = rotation;
        // Pick a point producing minimum (10,20) for the rotated dimensions.
        let p = if rotation % 2 == 0 {
            point(192.0, 344.0)
        } else {
            point(184.0, 352.0)
        };
        h.calls.clear();
        t.constraint_feedback(&mut h, p);
        for array in &t.constraint_effects {
            assert_eq!(array.len(), 1);
            assert!(effects(&h).contains(&Effect::EffectParameter(array[0], 0xC, param.clone())));
        }
        if rotation == 0 {
            for name in ["selection_blue", "selection_red"] {
                assert!(h.calls.contains(&Call::EffectCreate(name.into())));
            }
        } else {
            assert!(!h.calls.iter().any(|c| matches!(c, Call::EffectCreate(_))));
        }
    }
    h.running.extend(t.constraint_effects.iter().flatten());
    h.entries.clear();
    h.calls.clear();
    t.constraint_feedback(&mut h, point(192.0, 344.0));
    for a in &t.constraint_effects {
        assert!(effects(&h).contains(&Effect::StopEffect(a[0], 0)));
    }
    // Missing configuration is a no-op; invalid coverage stops the same surplus.
    h.cfg = None;
    h.calls.clear();
    t.constraint_feedback(&mut h, point(0.0, 0.0));
    assert!(h.calls.is_empty());
}

#[test]
fn ct17_preview_categories_order_centering_props_and_selection_failures() {
    let mut h = Fake {
        visual: true,
        preview: PreviewInfo {
            police: true,
            education: true,
            prop_preview: true,
            ..PreviewInfo::default()
        },
        ..Fake::default()
    };
    h.radii.insert(1, (99.0, 42));
    h.candidates_building = vec![9, 7];
    h.building_entry.as_mut().unwrap().position = point(19.0, 21.0);
    h.entries = vec![
        Entry {
            kind: 1,
            flags: 1,
            identifiers: vec![1],
            ..Entry::default()
        },
        Entry {
            kind: 1,
            identifiers: vec![2, 3],
            ..Entry::default()
        },
        Entry {
            kind: 1,
            identifiers: vec![5],
            ..Entry::default()
        },
        Entry {
            kind: 1,
            identifiers: vec![4],
            position: point(19.0, 21.0),
            ..Entry::default()
        },
    ];
    h.denied_props = vec![1005];
    let mut t = PlaceLot::new(0, 10, 0, false);
    assert!(t.init(&mut h));
    assert!(h.calls.contains(&Call::PreviewBuilding(9)));
    assert!(!h.calls.contains(&Call::PreviewProp(1)) && !h.calls.contains(&Call::PreviewProp(2)));
    assert_eq!(t.previews.len(), 2);
    for p in &t.previews {
        assert_eq!(p.local, point(3.0, 5.0));
    }
    assert_eq!(t.radius, 32.0);
    assert_eq!(
        t.coverage_names,
        Some(["PlopMode_Education_Plop".into(), "PlopMode_Education_Inactive".into()])
    );
    assert!(effects(&h).contains(&Effect::CoverageCategories(9)));
    for token in [90, 1004] {
        for key in [0x0292_44DB, 0x29F0_13E6] {
            assert!(effects(&h).contains(&Effect::OccupantProperty(token, key, true)));
        }
    }
    assert!(effects(&h).contains(&Effect::AcquirePreview(1004)));
    assert!(!effects(&h).contains(&Effect::AcquirePreview(1005)));
    assert!(effects(&h).contains(&Effect::ReleaseCreatedPreview(1005)));
    h.calls.clear();
    t.shutdown(&mut h);
    assert!(effects(&h).contains(&Effect::ReleasePreview(1004)));
    assert!(effects(&h).contains(&Effect::ReleaseCreatedPreview(90)));
    assert!(effects(&h).contains(&Effect::ReleaseCreatedEffect(299)));
    assert!(effects(&h).contains(&Effect::ReleaseTexture(500)));
    for failure in 0..5 {
        let mut h = Fake::default();
        if failure == 0 {
            h.building_entry = None;
        }
        if failure == 1 {
            h.candidates_building.clear();
        }
        if failure == 2 {
            h.profile = false;
        }
        if failure == 3 {
            h.building_creation = false;
        }
        if failure == 4 {
            h.preview.prop_preview = false;
            h.entries = vec![Entry {
                kind: 1,
                identifiers: vec![4],
                ..Entry::default()
            }];
        }
        let mut t = PlaceLot::new(0, 10, 0, false);
        t.init(&mut h);
        assert_eq!(t.previews.len(), usize::from(failure == 4));
    }
}

#[test]
fn ct17_all_categories_preview_views_and_configuration_release() {
    let mut h = Fake {
        visual: true,
        groups: vec![0x1401, 0x1302],
        preview: PreviewInfo {
            police: true,
            fire: true,
            health: true,
            education: true,
            prop_preview: false,
        },
        ..Fake::default()
    };
    for bit in [1, 2, 4, 8] {
        h.radii.insert(bit, (bit as f32 * 10.0, bit));
    }
    let mut t = tool(&mut h);
    assert_eq!((t.radius, t.radius_auxiliary, t.coverage_view), (80.0, 8, 0xA));
    h.groups = vec![0x1302];
    t.change_configuration(&mut h, 11);
    assert_eq!(t.coverage_view, 0xF2);
    assert!(effects(&h).contains(&Effect::CoverageCategories(15)));
    assert!(effects(&h).contains(&Effect::ReleaseCreatedPreview(90)));
    t.coverage_view = 0xF2;
    h.groups.clear();
    t.change_configuration(&mut h, 12);
    assert_eq!(t.coverage_view, 0xF2);
    h.calls.clear();
    h.cfg = None;
    t.build_preview(&mut h);
    assert!(h.calls.is_empty());
}

#[test]
fn ct17_message_rebuild_and_absent_resources_do_not_substitute_visuals() {
    let (mut h, mut t) = visual_tool();
    t.feedback(&mut h, true, true, point(40.0, 40.0));
    t.placeable = true;
    t.feedback_flag = true;
    t.position = point(40.0, 40.0);
    let previous = t.preview_effects;
    h.calls.clear();
    t.message(&mut h, 0xA6B7_9602);
    assert!(previous
        .iter()
        .flatten()
        .all(|token| effects(&h).contains(&Effect::ReleaseCreatedEffect(*token))));
    assert!(t.preview_effects.iter().all(Option::is_some));
    h.visual = false;
    t.stop_previews(&mut h);
    h.calls.clear();
    t.feedback(&mut h, false, true, point(40.0, 40.0));
    assert_eq!(t.preview_effects, [None; 2]);
    let names: Vec<_> = h
        .calls
        .iter()
        .filter_map(|c| {
            if let Call::EffectCreate(name) = c {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(names, vec!["building_footprint_notok", "Lot_Direction_Arrow"]);
    t.previews.clear();
    t.building_preview = false;
    h.calls.clear();
    t.feedback(&mut h, true, false, point(40.0, 40.0));
    assert!(!h
        .calls
        .iter()
        .any(|c| matches!(c, Call::PreviewBuilding(_) | Call::PreviewProp(_))));
}

#[test]
fn ct17_arrow_precision_restart_footprint_texture_and_grid_modes() {
    let (mut h, mut t) = visual_tool();
    h.cfg.as_mut().unwrap().size = [4, 3];
    h.city_hidden = true;
    for (rotation, x, z) in [
        (0, 100.0, 182.0),
        (1, 118.0, 200.0),
        (2, 100.0, 218.0),
        (3, 82.0, 200.0),
    ] {
        t.rotation = rotation;
        h.calls.clear();
        t.feedback(
            &mut h,
            true,
            true,
            Vec3 {
                x: 100.0,
                y: 7.0,
                z: 200.0,
            },
        );
        let arrow = t.preview_effects[1].unwrap();
        assert!(effects(&h).contains(&Effect::EffectTransform(
            arrow,
            Vec3 { x, y: 7.0, z },
            f64::from_bits(0xBFF9_21F9_F01B_866E) * f64::from(rotation),
            1.0
        )));
        assert!(effects(&h).contains(&Effect::StartEffect(arrow, 0)));
        assert!(effects(&h).contains(&Effect::GridVisible(false)));
        let foot = t.preview_effects[0].unwrap();
        assert!(effects(&h).contains(&Effect::EffectTexture(foot, 6, 500)));
        assert!(effects(&h).contains(&Effect::EffectTransform(
            foot,
            Vec3 {
                x: 100.0,
                y: 7.0,
                z: 200.0
            },
            f64::from(f32::from_bits(0x3FC9_0FD0) * f32::from(rotation)),
            64.0 * f32::from_bits(0x3F7F_FF58)
        )));
        if rotation > 0 {
            assert!(!h.calls.contains(&Call::EffectCreate("Lot_Direction_Arrow".into())));
        }
    }
    t.radius = 0.0;
    h.calls.clear();
    t.feedback(&mut h, true, true, point(40.0, 40.0));
    assert!(!effects(&h).contains(&Effect::StartEffect(299, 0)));
}

#[test]
fn ct16_move_commit_changes_configuration_and_keeps_foreign_services() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    t.toll_booth = true;
    h.networks.insert((2, 2), network(0x40, 0x00020002, 0));
    t.mouse_move(&mut h, mouse(), 0);
    assert_eq!((t.configuration, t.rotation), (0xABF9_A45C, 1));
    assert!(t.placeable);
    t.cost = 500;
    h.calls.clear();
    let result = t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false);
    assert!(result.built);
    assert_eq!(result.rect, cells(2, 2, 3, 3));
    assert!(h
        .calls
        .contains(&Call::Lot(cells(2, 2, 3, 3), 1, true, false, 0xABF9_A45C)));
}
