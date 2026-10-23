// SPDX-License-Identifier: GPL-3.0-or-later
// Literal discriminating fixtures written before the production module.
use super::*;

fn palette() -> RawPalette {
    RawPalette(std::array::from_fn(|i| 0x7f00_0000 | ((i as u32) * 0x010101)))
}

fn exemplar(source: u32, kind: u32) -> RawExemplar {
    RawExemplar(vec![
        Property::new(SOURCE, RawValue::U32(source)),
        Property::new(RAMP, RawValue::U32List(vec![0, 0, 255, 0xffff_ffff])),
        Property::new(INTERPOLATE, RawValue::Bool(false)),
        Property::new(HIGHLIGHT, RawValue::U32(kind)),
    ])
}

fn selected(source: u32) -> MapView {
    let mut view = MapView::new(0, 7);
    view.activate();
    let progress = view.set_data_view(1, Some(&exemplar(source, 9)));
    assert_eq!(
        progress.outcome,
        SelectionOutcome::Unsupported(Unsupported::RampGeneration)
    );
    assert_eq!(view.current_view(), None);
    view.hook_update_messages().unwrap();
    view
}

#[test]
fn full_eight_bit_domains_use_distinct_supplied_entries() {
    let p = palette();
    for v in 0..=255u8 {
        assert_eq!(u8_index(v), v);
        assert_eq!(p.lookup(u8_index(v)), 0x7f00_0000 | (u32::from(v) * 0x010101));
    }
    for v in -128..=127i16 {
        assert_eq!(i8_index(v as i8), (v + 128) as u8);
    }
    let mut out = [0; 3];
    emit_colors(Values::I8(&[-128, 0, 127]), &p, &mut out).unwrap();
    assert_eq!(out, [0x7f00_0000, 0x7f80_8080, 0x7fff_ffff]);
    emit_colors(Values::U8(&[0, 255, 128]), &p, &mut out).unwrap();
    assert_eq!(out, [0x7f00_0000, 0x7fff_ffff, 0x7f80_8080]);
}

#[test]
fn floats_truncate_before_range_check_and_refuse_unpublished_domains() {
    for (v, index) in [(-0.75, 0), (0.99, 0), (1.99, 1), (127.9, 127), (255.99, 255)] {
        assert_eq!(float_index(v), Ok(index));
    }
    for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(float_index(v), Err(Unsupported::NonFiniteFloat));
    }
    for v in [-1.0, 256.0, f32::MAX, f32::MIN] {
        assert_eq!(float_index(v), Err(Unsupported::FloatOutOfRange));
    }
    let mut out = [42; 2];
    assert_eq!(
        emit_colors(Values::F32(&[1.9, 256.0]), &palette(), &mut out),
        Err(Unsupported::FloatOutOfRange)
    );
    assert_eq!(out, [42, 42]);
}

#[test]
fn contradictory_sixteen_bit_rules_never_become_a_conversion() {
    assert_eq!(u16_index(512, 1024), Err(Unsupported::SixteenBitConflict));
    assert_eq!(i16_index(-512, 1024), Err(Unsupported::SixteenBitConflict));
    assert_eq!(u16_index(0, 256), Err(Unsupported::SixteenBitConflict));
    assert_eq!(i16_index(0, 256), Err(Unsupported::SixteenBitConflict));
    // Independent arithmetic demonstrates QV5; it is not the implementation.
    assert_eq!(128 + ((-512i64 * (0x7fff00 / 1024)) >> 16), 64);
    assert_eq!((0xffff00 / 256, 0x7fff00 / 256), (65535, 32767));
}

#[test]
fn premultiplication_rounds_channels_and_keeps_alpha() {
    assert_eq!(premultiply_argb(0x8001_02ff), 0x8001_0180);
    assert_eq!(premultiply_argb(0x00ff_1234), 0);
    assert_eq!(premultiply_argb(0xff12_3456), 0xff12_3456);
}

#[test]
fn raw_fields_use_only_documented_defaults_and_ranges() {
    let mut raw = exemplar(64, 1);
    let fields = raw.selection_fields().unwrap();
    assert_eq!((fields.scale, fields.layer, fields.block_by_lot), (256, 1, None));
    for (value, expected) in [(0, 256), (1, 256), (2, 2), (0xff_ffff, 0xff_ffff), (0x100_0000, 256)] {
        raw.0.retain(|p| p.id != SCALE);
        raw.0.push(Property::new(SCALE, RawValue::U32(value)));
        assert_eq!(raw.selection_fields().unwrap().scale, expected);
    }
    raw.0.push(Property::new(BLOCK_BY_LOT, RawValue::U32(0)));
    assert_eq!(raw.selection_fields().unwrap().block_by_lot, Some(0));
    raw.0.push(Property::new(LAYER, RawValue::U32(6)));
    assert_eq!(raw.selection_fields().unwrap().layer, 6);
    raw.0.retain(|p| p.id != SOURCE);
    assert_eq!(raw.selection_fields(), Err(Unsupported::MissingProperty(SOURCE)));
    raw.0.push(Property::new(SOURCE, RawValue::U8(1)));
    assert_eq!(raw.selection_fields(), Err(Unsupported::MalformedProperty(SOURCE)));
}

#[test]
fn typed_resource_helpers_do_not_supply_filter_or_traffic_outcomes() {
    let raw = RawExemplar(vec![
        Property::new(OCCUPANT_TYPES, RawValue::U32List(vec![0, 4, 0, 9])),
        Property::new(TERRAIN_MODE, RawValue::U32(1)),
        Property::new(CHILD_FLAG, RawValue::Bool(true)),
        Property::new(TRAVEL_TYPES, RawValue::U32List(vec![1, 5, 255])),
    ]);
    assert_eq!(raw.occupant_types().unwrap(), vec![4, 9]);
    assert_eq!(raw.terrain_mode(TERRAIN_MODE), Ok(3));
    assert_eq!(raw.bool_field(CHILD_FLAG), Ok(true));
    assert_eq!(raw.travel_types().unwrap(), vec![1, 5, 255]);
    assert_eq!(
        raw.bool_field(INTERPOLATE),
        Err(Unsupported::MissingProperty(INTERPOLATE))
    );
    for (mode, target) in [(0, 1), (1, 3), (2, 2)] {
        assert_eq!(
            RawExemplar(vec![Property::new(TERRAIN_MODE, RawValue::U32(mode))]).terrain_mode(TERRAIN_MODE),
            Ok(target)
        );
    }
    assert_eq!(
        RawExemplar(vec![Property::new(TRAVEL_TYPES, RawValue::U32List(vec![256]))]).travel_types(),
        Err(Unsupported::MalformedProperty(TRAVEL_TYPES))
    );
}

#[test]
fn source_selection_discriminates_empty_types_and_native_slots() {
    assert_eq!(select_source(0, 1, false), Ok(SourceSelection::Clear));
    for source in [11, 13, 19, 20, 21, 22, 23, 31, 63] {
        assert_eq!(select_source(source, 1, false), Ok(SourceSelection::DrawNothing));
    }
    assert_eq!(
        select_source(1, 1, false),
        Ok(SourceSelection::Grid(GridRequest {
            provider: Provider::Aura,
            slot: 12,
            argument: None,
            value_type: ValueType::I8,
            own_cells: false
        }))
    );
    assert_eq!(
        select_source(24, 1, false),
        Ok(SourceSelection::Grid(GridRequest {
            provider: Provider::Pollution,
            slot: 5,
            argument: Some(1),
            value_type: ValueType::I16,
            own_cells: false
        }))
    );
    assert!(matches!(
        select_source(29, 1, false),
        Ok(SourceSelection::Grid(GridRequest {
            slot: 18,
            own_cells: true,
            ..
        }))
    ));
    assert_eq!(select_source(76, 1, false), Ok(SourceSelection::TrafficVolumeQV2));
    assert_eq!(select_source(77, 1, false), Err(Unsupported::UnknownSource(77)));
}

#[test]
fn every_developer_type_and_published_layer_is_discriminated() {
    let types = [
        0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130, 0x3320, 0x3330, 0x4100, 0x4200, 0x4300, 0x4400,
    ];
    for (i, developer_type) in types.into_iter().enumerate() {
        for (layer, grid) in [
            (2, DeveloperGrid::Unsigned16Slot14),
            (3, DeveloperGrid::Signed16Slot13),
            (4, DeveloperGrid::Signed16Slot13),
            (6, DeveloperGrid::DebugEntriesTimes12),
        ] {
            assert_eq!(
                select_source(64 + i as u32, layer, false),
                Ok(SourceSelection::Developer {
                    developer_type,
                    layer,
                    grid
                })
            );
        }
        assert_eq!(
            select_source(64 + i as u32, 1, true),
            Ok(SourceSelection::Developer {
                developer_type,
                layer: 6,
                grid: DeveloperGrid::DebugEntriesTimes12
            })
        );
    }
    assert_eq!(select_source(64, 1, false), Err(Unsupported::DeveloperLayer(1)));
    assert_eq!(select_source(64, 5, false), Err(Unsupported::DeveloperLayer5QV4));
}

#[test]
fn inactive_current_and_none_boundaries_preserve_owned_state() {
    let mut view = MapView::new(123, 7);
    assert_eq!(view.set_data_view(5, None).outcome, SelectionOutcome::Inactive);
    assert_eq!(view.update(), UpdateOutcome::Inactive);
    view.activate();
    view.current_view = Some(5); // fixture of a known current value; no native default is inferred.
    assert_eq!(view.set_data_view(5, None).outcome, SelectionOutcome::CurrentView);
    assert_eq!(view.current_view(), Some(5));
    let selected = view.set_data_view(6, Some(&exemplar(8, 1)));
    assert_eq!(
        selected.steps,
        vec![
            SelectionStep::DropUpdateSubscriptions,
            SelectionStep::ReadExemplar(ExemplarKey {
                resource_type: 0x6534_284a,
                group: 0x690f_693f,
                instance: 0x4a0b_6806
            }),
            SelectionStep::StoreFields
        ]
    );
    assert_eq!(view.source(), 8);
    assert_eq!(view.current_view(), Some(5));
    let before = view.config().cloned();
    assert_eq!(
        view.set_data_view(0, None).outcome,
        SelectionOutcome::Unsupported(Unsupported::FilterResetQV1)
    );
    assert_eq!(view.source(), 8);
    assert_eq!(view.config(), before.as_ref());
    assert_eq!(view.last_redraw_ms(), 123);
}

#[test]
fn missing_exemplar_drops_subscriptions_before_returning_missing() {
    let mut view = selected(8);
    assert_eq!(view.update_subscription().unwrap().message, 0xfa6d_d0bb);
    let result = view.set_data_view(9, None);
    assert_eq!(result.outcome, SelectionOutcome::MissingExemplar);
    assert_eq!(result.steps[0], SelectionStep::DropUpdateSubscriptions);
    assert_eq!(view.update_subscription(), None);
    assert_eq!(view.source(), 8);
}

#[test]
fn messages_and_source_changes_follow_exact_subscription_table() {
    let cases = [
        (1, 0x2a09_b21a),
        (2, 0x87cc_75b3),
        (4, 0x88ea_4dff),
        (7, 0x6a07_c2ee),
        (8, 0xfa6d_d0bb),
        (24, 0x5a6d_d1aa),
        (25, 0xda6d_d2cb),
        (26, 0xda6d_d35d),
        (9, 0x6924_7dc7),
        (76, 0x6924_7dc7),
        (10, 0x6946_dc34),
        (29, 0x09b6_d58e),
    ];
    for (source, message) in cases {
        let s = subscription(source);
        assert_eq!(s.message, message);
        assert_eq!(s.priority, if source == 76 { 0xc0 } else { 0 });
    }
    for source in [0, 3, 5, 6, 11, 20, 27, 30, 64, 75] {
        assert_eq!(subscription(source).message, ANIMATION_TICK);
    }
    let mut view = selected(8);
    view.notify(0x5a6d_d1aa, None).unwrap();
    assert!(!view.dirty());
    view.notify(0xfa6d_d0bb, None).unwrap();
    assert!(view.dirty());
    view.dirty = false;
    view.set_data_view(2, Some(&exemplar(24, 9)));
    let effects = view.hook_update_messages().unwrap();
    assert_eq!(
        effects,
        vec![
            Effect::RegisterAnimation(AnimationPriority::Explicit(0)),
            Effect::Subscribe(0x5a6d_d1aa)
        ]
    );
    view.notify(0xfa6d_d0bb, None).unwrap();
    assert!(!view.dirty());
    view.notify(0x5a6d_d1aa, None).unwrap();
    assert!(view.dirty());
}

#[test]
fn notifications_dirty_only_when_the_raw_value_changes() {
    let mut view = MapView::new(0, 7);
    view.notify(VALUE_CHANGED, Some(7)).unwrap();
    assert!(!view.dirty());
    view.notify(VALUE_CHANGED, Some(8)).unwrap();
    assert!(view.dirty());
    view.dirty = false;
    view.notify(VALUE_CHANGED, Some(8)).unwrap();
    assert!(!view.dirty());
}

#[test]
fn ordinary_and_traffic_tick_thresholds_retain_dirty_on_missing_renderer() {
    let mut view = selected(8);
    view.notify(0xfa6d_d0bb, None).unwrap();
    view.notify(0xfa6d_d0bb, None).unwrap();
    let before = view.tick(999, &[]).unwrap();
    assert!(before.polled_highlights);
    assert!(!before.redraw_due_for_known_dirty);
    let due = view.tick(1000, &[]).unwrap();
    assert!(due.redraw_due_for_known_dirty);
    assert_eq!(
        due.update,
        Some(UpdateOutcome::Unsupported(Unsupported::RampGeneration))
    );
    assert!(view.dirty());
    assert_eq!(view.last_redraw_ms(), 0);
    let mut traffic = selected(76);
    assert!(!traffic.dirty());
    assert!(!traffic.tick(249, &[]).unwrap().redraw_due_for_known_dirty);
    assert!(traffic.dirty());
    assert_eq!(
        traffic.tick(250, &[]).unwrap().update,
        Some(UpdateOutcome::Unsupported(Unsupported::TrafficQV2))
    );
    assert!(traffic.dirty());
    assert_eq!(traffic.last_redraw_ms(), 0);
}

#[test]
fn raw_highlights_select_native_radius_without_claiming_comparison() {
    let buildings = [
        RawBuilding {
            id: 1,
            kind: BuildingKind::School,
            coverage: Coverage {
                residential: Some(100.0),
                fire_half: Some(999.0),
                police_half: None,
            },
        },
        RawBuilding {
            id: 2,
            kind: BuildingKind::FireStation,
            coverage: Coverage {
                residential: Some(999.0),
                fire_half: Some(50.0),
                police_half: None,
            },
        },
        RawBuilding {
            id: 3,
            kind: BuildingKind::TransitSwitch,
            coverage: Coverage::default(),
        },
    ];
    assert_eq!(
        collect_highlights(1, &buildings).unwrap().candidates,
        vec![HighlightCandidate {
            id: 1,
            radius: HighlightRadius::Value(100.0)
        }]
    );
    assert_eq!(
        collect_highlights(3, &buildings).unwrap().candidates,
        vec![HighlightCandidate {
            id: 2,
            radius: HighlightRadius::Value(50.0)
        }]
    );
    assert_eq!(
        collect_highlights(9, &buildings).unwrap().candidates,
        vec![HighlightCandidate {
            id: 3,
            radius: HighlightRadius::None
        }]
    );
    assert_eq!(highlight_request(5).unwrap().provider, Provider::Crime);
    let mut view = selected(1);
    for _ in 0..2 {
        assert_eq!(
            view.update_highlights(&buildings),
            Err(Unsupported::HighlightComparison)
        );
        assert!(!view.dirty());
    }
    assert_eq!(view.highlights().unwrap().candidates.len(), 1);
}

#[test]
fn overlay_registers_once_and_never_guesses_geometry_or_cell_inclusion() {
    let cell = ColoredCell {
        origin: [12.0, 34.0],
        extent: [16.0, 16.0],
        altitude: 7.0,
        argb: 0x8001_0180,
    };
    let mut overlay = OverlayState::default();
    assert!(overlay.plan(false, &[cell]).unwrap().is_empty());
    assert_eq!(
        overlay.plan(true, &[cell]).unwrap(),
        vec![
            OverlayEffect::Register {
                renderer_slot: 32,
                layer: 5,
                priority: 1000
            },
            OverlayEffect::Mark(cell)
        ]
    );
    assert_eq!(overlay.plan(true, &[cell]).unwrap(), vec![OverlayEffect::Mark(cell)]);
    assert!(overlay.plan(false, &[]).unwrap().is_empty());
    assert_eq!(overlay.plan(true, &[cell]).unwrap(), vec![OverlayEffect::Mark(cell)]);
}

#[test]
fn raw_grid_emission_reuses_existing_tract_access_without_resampling() {
    use crate::grid::{CityDims, SimGrid, TractGrid};
    let grid = SimGrid::<u8>::create(&CityDims::square(2), 99, 1);
    grid.set(0, 0, 0);
    grid.set(0, 1, 255);
    grid.set(1, 0, 128);
    grid.set(1, 1, 1);
    let mut output = [0; 4];
    emit_grid(&*grid, &palette(), &mut output).unwrap();
    assert_eq!(output, [0x7f00_0000, 0x7fff_ffff, 0x7f80_8080, 0x7f01_0101]);
    assert_eq!(
        emit_grid(&*grid, &palette(), &mut output[..3]),
        Err(Unsupported::BufferLength)
    );
    let grid = SimGrid::<f32>::create(&CityDims::square(2), 100, 1);
    grid.set(0, 0, 1.9);
    grid.set(1, 1, f32::NAN);
    output.fill(123);
    assert_eq!(
        emit_grid(&*grid, &palette(), &mut output),
        Err(Unsupported::NonFiniteFloat)
    );
    assert_eq!(output, [123; 4]);
}

#[test]
fn every_raw_source_slot_and_empty_range_has_a_literal_fixture() {
    let cases = [
        (2, Provider::LandValue, 8, ValueType::U8, None),
        (3, Provider::Police, 5, ValueType::U8, None),
        (4, Provider::Crime, 5, ValueType::U8, None),
        (5, Provider::Fire, 5, ValueType::U8, None),
        (6, Provider::Flammability, 5, ValueType::U8, None),
        (7, Provider::Power, 35, ValueType::U8, None),
        (8, Provider::Pollution, 5, ValueType::I16, Some(0)),
        (9, Provider::Traffic, 10, ValueType::U8, None),
        (10, Provider::Plumbing, 7, ValueType::U8, None),
        (12, Provider::Aura, 14, ValueType::I16, None),
        (25, Provider::Pollution, 5, ValueType::I16, Some(2)),
        (26, Provider::Pollution, 5, ValueType::I16, Some(3)),
        (27, Provider::Residential, 21, ValueType::F32, None),
        (28, Provider::Residential, 22, ValueType::F32, None),
        (30, Provider::Residential, 41, ValueType::U8, None),
    ];
    for (source, provider, slot, value_type, argument) in cases {
        assert_eq!(
            select_source(source, 1, false),
            Ok(SourceSelection::Grid(GridRequest {
                provider,
                slot,
                argument,
                value_type,
                own_cells: false
            }))
        );
    }
    for source in [11].into_iter().chain(13..=23).chain(31..=63) {
        assert_eq!(select_source(source, 1, false), Ok(SourceSelection::DrawNothing));
    }
    for source in [11, 20, 21, 22, 23] {
        assert_eq!(selected(source).update(), UpdateOutcome::DrawNothing);
    }
}

#[test]
fn init_and_activation_have_only_published_primitive_ordering() {
    let mut view = MapView::new(0, 0);
    let init = view.init().unwrap();
    assert_eq!(init.boundary, Unsupported::NativeAllocation);
    assert_eq!(
        init.effects,
        vec![
            Effect::WindowGeneratorInit,
            Effect::Subscribe(0x26d3_1ec1),
            Effect::Subscribe(0x26d3_1ec2),
            Effect::Subscribe(0xa6b7_9602),
            Effect::CreateMarkedCellView { native_bytes: 0x50 },
            Effect::InitMarkedCellView,
            Effect::IgnoreDepthBuffer,
            Effect::UseWaterAltitude,
        ]
    );
    assert_eq!(view.init(), Err(Unsupported::RepeatedInit));
    assert_eq!(
        view.notify(ACTIVATE, None).unwrap(),
        vec![Effect::RegisterAnimation(AnimationPriority::UnspecifiedActivation)]
    );
    assert!(view.active());
    assert_eq!(view.update(), UpdateOutcome::ClearMapIntent);
    assert_eq!(
        view.notify(DEACTIVATE, None).unwrap(),
        vec![Effect::UnregisterAnimation]
    );
    assert!(!view.active());
    assert_eq!(view.update(), UpdateOutcome::Inactive);
    fn send_sync<T: Send + Sync>() {}
    send_sync::<MapView>();
}

#[test]
fn malformed_resource_and_helper_inputs_are_refused_without_native_failure_values() {
    let mut raw = exemplar(1, 9);
    raw.0.push(Property::new(SOURCE, RawValue::U32(2)));
    assert_eq!(raw.selection_fields(), Err(Unsupported::DuplicateProperty(SOURCE)));
    raw = exemplar(1, 9);
    raw.0.retain(|p| p.id != RAMP);
    raw.0.push(Property::new(RAMP, RawValue::U32List(vec![1])));
    assert_eq!(raw.selection_fields(), Err(Unsupported::MalformedProperty(RAMP)));
    let invalid = RawExemplar(vec![Property::new(TERRAIN_MODE, RawValue::U32(3))]);
    assert_eq!(
        invalid.terrain_mode(TERRAIN_MODE),
        Err(Unsupported::MalformedProperty(TERRAIN_MODE))
    );
    let invalid = RawExemplar(vec![Property::new(TRAVEL_TYPES, RawValue::U32List(vec![0; 9]))]);
    assert_eq!(
        invalid.travel_types(),
        Err(Unsupported::MalformedProperty(TRAVEL_TYPES))
    );
    assert_eq!(exemplar_key(u32::MAX), Err(Unsupported::ResourceKeyOverflow));
    let mut output = [9];
    assert_eq!(
        emit_colors(Values::U8(&[1, 2]), &palette(), &mut output),
        Err(Unsupported::BufferLength)
    );
    for values in [Values::U16(&[512]), Values::I16(&[-512])] {
        assert_eq!(
            emit_colors(values, &palette(), &mut output),
            Err(Unsupported::SixteenBitConflict)
        );
        assert_eq!(output, [9]);
    }
}

#[test]
fn same_view_and_inactive_paths_do_not_drop_existing_state_or_subscriptions() {
    let mut view = selected(8);
    view.current_view = Some(1);
    view.notify(0xfa6d_d0bb, None).unwrap();
    let before = view.config().cloned();
    let subscription = view.update_subscription();
    assert!(view.set_data_view(1, None).effects.is_empty());
    assert_eq!(view.update_subscription(), subscription);
    view.deactivate();
    assert!(view.set_data_view(2, Some(&exemplar(9, 3))).steps.is_empty());
    assert_eq!(view.config(), before.as_ref());
    assert_eq!(view.source(), 8);
    assert!(view.dirty());
    assert_eq!(view.hook_update_messages(), Err(Unsupported::RepeatedHook));
}

#[test]
fn empty_source_animation_notifications_dirty_without_inventing_highlight_changes() {
    let mut view = selected(20);
    assert!(!view.dirty());
    let progress = view.tick(999, &[]).unwrap();
    assert!(progress.polled_highlights);
    assert!(view.dirty());
    assert_eq!(progress.highlight_boundary, Some(Unsupported::HighlightComparison));
    assert!(!progress.redraw_due_for_known_dirty);
    assert_eq!(view.tick(1000, &[]).unwrap().update, Some(UpdateOutcome::DrawNothing));
    assert!(view.dirty());
    assert_eq!(view.last_redraw_ms(), 0);
    let mut view = selected(8);
    let progress = view.tick(1000, &[]).unwrap();
    assert!(progress.polled_highlights);
    assert!(!progress.redraw_due_for_known_dirty);
    assert_eq!(progress.highlight_boundary, Some(Unsupported::HighlightComparison));
    assert_eq!(progress.update, None); // known dirty absent, not proof of native equality.
    let mut view = MapView::new(500, 7);
    assert_eq!(view.tick(499, &[]), Err(Unsupported::ClockRegression));
    assert_eq!(
        view.notify(VALUE_CHANGED, None),
        Err(Unsupported::MissingNotificationValue)
    );
}

#[test]
fn owner_boundaries_cannot_be_completed_by_supplied_helper_inputs() {
    let mut view = selected(64);
    assert_eq!(
        view.update(),
        UpdateOutcome::Unsupported(Unsupported::DeveloperLayer(1))
    );
    view.set_developer_debug_flag(true);
    assert_eq!(
        view.update(),
        UpdateOutcome::Unsupported(Unsupported::DeveloperDebugLayout)
    );
    let mut raw = exemplar(2, 1);
    raw.0.retain(|p| p.id != INTERPOLATE);
    raw.0.push(Property::new(INTERPOLATE, RawValue::Bool(true)));
    view.set_data_view(2, Some(&raw));
    assert_eq!(
        view.update(),
        UpdateOutcome::Unsupported(Unsupported::GridInterpolation)
    );
    raw.0.retain(|p| p.id != INTERPOLATE);
    raw.0.push(Property::new(INTERPOLATE, RawValue::Bool(false)));
    raw.0.push(Property::new(BLOCK_BY_LOT, RawValue::U32(0)));
    view.set_data_view(2, Some(&raw));
    assert_eq!(view.update(), UpdateOutcome::Unsupported(Unsupported::LotBlockingQV4));
    view.set_data_view(3, Some(&exemplar(4, 4)));
    assert_eq!(view.update(), UpdateOutcome::Unsupported(Unsupported::CrimeMarkersQV4));
}

#[test]
fn all_supported_highlight_kinds_select_distinct_raw_inputs_and_slots() {
    let cases = [
        (
            1,
            BuildingKind::School,
            Provider::Residential,
            47,
            HighlightRadius::Value(11.0),
        ),
        (
            2,
            BuildingKind::Hospital,
            Provider::Residential,
            49,
            HighlightRadius::Value(11.0),
        ),
        (
            3,
            BuildingKind::FireStation,
            Provider::Fire,
            22,
            HighlightRadius::Value(22.0),
        ),
        (
            4,
            BuildingKind::PoliceStation,
            Provider::Police,
            21,
            HighlightRadius::Value(33.0),
        ),
        (
            6,
            BuildingKind::GarbageBuilding,
            Provider::Pollution,
            46,
            HighlightRadius::Unspecified,
        ),
        (
            7,
            BuildingKind::PowerPlant,
            Provider::Power,
            25,
            HighlightRadius::Unspecified,
        ),
        (
            8,
            BuildingKind::WaterProducer,
            Provider::Plumbing,
            28,
            HighlightRadius::Unspecified,
        ),
        (
            9,
            BuildingKind::TransitSwitch,
            Provider::Traffic,
            28,
            HighlightRadius::None,
        ),
    ];
    let records: Vec<_> = cases
        .iter()
        .enumerate()
        .map(|(i, (_, kind, _, _, _))| RawBuilding {
            id: i as u64,
            kind: *kind,
            coverage: Coverage {
                residential: Some(11.0),
                fire_half: Some(22.0),
                police_half: Some(33.0),
            },
        })
        .collect();
    for (i, (kind, _, provider, building_slot, radius)) in cases.into_iter().enumerate() {
        let request = highlight_request(kind).unwrap();
        assert_eq!((request.provider, request.building_slot), (provider, building_slot));
        assert_eq!(
            collect_highlights(kind, &records).unwrap().candidates,
            vec![HighlightCandidate { id: i as u64, radius }]
        );
    }
    assert_eq!(
        highlight_request(3).unwrap().coverage,
        CoverageRequest::FireSlot21 { level: 0.5 }
    );
    assert_eq!(
        highlight_request(4).unwrap().coverage,
        CoverageRequest::PoliceSlot20 { level: 0.5 }
    );
    let missing = [RawBuilding {
        id: 1,
        kind: BuildingKind::School,
        coverage: Coverage::default(),
    }];
    assert_eq!(collect_highlights(1, &missing), Err(Unsupported::HighlightRadius(1)));
    assert_eq!(collect_highlights(0, &records), Err(Unsupported::HighlightKind(0)));
}

#[test]
fn overlay_geometry_refusal_is_atomic_and_transparent_cells_are_not_filtered() {
    let mut overlay = OverlayState::default();
    let cell = ColoredCell {
        origin: [0.0, 0.0],
        extent: [16.0, 16.0],
        altitude: 1.0,
        argb: 0,
    };
    let bad = ColoredCell {
        altitude: f32::NAN,
        ..cell
    };
    assert_eq!(overlay.plan(true, &[cell, bad]), Err(Unsupported::GridGeometry));
    assert!(!overlay.registered());
    let effects = overlay.plan(true, &[cell]).unwrap();
    assert_eq!(effects.last(), Some(&OverlayEffect::Mark(cell)));
}
