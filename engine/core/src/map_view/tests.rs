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
    assert_eq!(progress.outcome, SelectionOutcome::Unsupported(Unsupported::RampGeneration));
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
    assert_eq!(emit_colors(Values::F32(&[1.9, 256.0]), &palette(), &mut out), Err(Unsupported::FloatOutOfRange));
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
    assert_eq!(raw.bool_field(INTERPOLATE), Err(Unsupported::MissingProperty(INTERPOLATE)));
    for (mode, target) in [(0, 1), (1, 3), (2, 2)] {
        assert_eq!(RawExemplar(vec![Property::new(TERRAIN_MODE, RawValue::U32(mode))]).terrain_mode(TERRAIN_MODE), Ok(target));
    }
    assert_eq!(RawExemplar(vec![Property::new(TRAVEL_TYPES, RawValue::U32List(vec![256]))]).travel_types(), Err(Unsupported::MalformedProperty(TRAVEL_TYPES)));
}

#[test]
fn source_selection_discriminates_empty_types_and_native_slots() {
    assert_eq!(select_source(0, 1, false), Ok(SourceSelection::Clear));
    for source in [11, 13, 19, 20, 21, 22, 23, 31, 63] {
        assert_eq!(select_source(source, 1, false), Ok(SourceSelection::DrawNothing));
    }
    assert_eq!(select_source(1, 1, false), Ok(SourceSelection::Grid(GridRequest { provider: Provider::Aura, slot: 12, argument: None, value_type: ValueType::I8, own_cells: false })));
    assert_eq!(select_source(24, 1, false), Ok(SourceSelection::Grid(GridRequest { provider: Provider::Pollution, slot: 5, argument: Some(1), value_type: ValueType::I16, own_cells: false })));
    assert!(matches!(select_source(29, 1, false), Ok(SourceSelection::Grid(GridRequest { slot:18, own_cells:true, .. }))));
    assert_eq!(select_source(76, 1, false), Ok(SourceSelection::TrafficVolumeQV2));
    assert_eq!(select_source(77, 1, false), Err(Unsupported::UnknownSource(77)));
}

#[test]
fn every_developer_type_and_published_layer_is_discriminated() {
    let types = [0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130, 0x3320, 0x3330, 0x4100, 0x4200, 0x4300, 0x4400];
    for (i, developer_type) in types.into_iter().enumerate() {
        for (layer, grid) in [(2, DeveloperGrid::Unsigned16Slot14), (3, DeveloperGrid::Signed16Slot13), (4, DeveloperGrid::Signed16Slot13), (6, DeveloperGrid::DebugEntriesTimes12)] {
            assert_eq!(select_source(64 + i as u32, layer, false), Ok(SourceSelection::Developer { developer_type, layer, grid }));
        }
        assert_eq!(select_source(64 + i as u32, 1, true), Ok(SourceSelection::Developer { developer_type, layer:6, grid:DeveloperGrid::DebugEntriesTimes12 }));
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
    assert_eq!(selected.steps, vec![SelectionStep::DropUpdateSubscriptions, SelectionStep::ReadExemplar(ExemplarKey { resource_type:0x6534_284a, group:0x690f_693f, instance:0x4a0b_6806 }), SelectionStep::StoreFields]);
    assert_eq!(view.source(), 8);
    assert_eq!(view.current_view(), Some(5));
    let before = view.config().cloned();
    assert_eq!(view.set_data_view(0, None).outcome, SelectionOutcome::Unsupported(Unsupported::FilterResetQV1));
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
    let cases = [(1,0x2a09_b21a), (2,0x87cc_75b3), (4,0x88ea_4dff), (7,0x6a07_c2ee), (8,0xfa6d_d0bb), (24,0x5a6d_d1aa), (25,0xda6d_d2cb), (26,0xda6d_d35d), (9,0x6924_7dc7), (76,0x6924_7dc7), (10,0x6946_dc34), (29,0x09b6_d58e)];
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
    assert_eq!(effects, vec![Effect::RegisterAnimation(AnimationPriority::Explicit(0)), Effect::Subscribe(0x5a6d_d1aa)]);
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
    assert_eq!(due.update, Some(UpdateOutcome::Unsupported(Unsupported::RampGeneration)));
    assert!(view.dirty());
    assert_eq!(view.last_redraw_ms(), 0);
    let mut traffic = selected(76);
    assert!(!traffic.dirty());
    assert!(!traffic.tick(249, &[]).unwrap().redraw_due_for_known_dirty);
    assert!(traffic.dirty());
    assert_eq!(traffic.tick(250, &[]).unwrap().update, Some(UpdateOutcome::Unsupported(Unsupported::TrafficQV2)));
    assert!(traffic.dirty());
    assert_eq!(traffic.last_redraw_ms(), 0);
}

#[test]
fn raw_highlights_select_native_radius_without_claiming_comparison() {
    let buildings = [
        RawBuilding { id:1, kind:BuildingKind::School, coverage:Coverage { residential:Some(100.0), fire_half:Some(999.0), police_half:None } },
        RawBuilding { id:2, kind:BuildingKind::FireStation, coverage:Coverage { residential:Some(999.0), fire_half:Some(50.0), police_half:None } },
        RawBuilding { id:3, kind:BuildingKind::TransitSwitch, coverage:Coverage::default() },
    ];
    assert_eq!(collect_highlights(1, &buildings).unwrap().candidates, vec![HighlightCandidate { id:1, radius:HighlightRadius::Value(100.0) }]);
    assert_eq!(collect_highlights(3, &buildings).unwrap().candidates, vec![HighlightCandidate { id:2, radius:HighlightRadius::Value(50.0) }]);
    assert_eq!(collect_highlights(9, &buildings).unwrap().candidates, vec![HighlightCandidate { id:3, radius:HighlightRadius::None }]);
    assert_eq!(collect_highlights(5, &buildings), Err(Unsupported::CrimeHotspot));
    let mut view = selected(1);
    for _ in 0..2 {
        assert_eq!(view.update_highlights(&buildings), Err(Unsupported::HighlightComparison));
        assert!(!view.dirty());
    }
    assert_eq!(view.highlights().unwrap().candidates.len(), 1);
}

#[test]
fn overlay_registers_once_and_never_guesses_geometry_or_cell_inclusion() {
    let cell = ColoredCell { origin:[12.0, 34.0], extent:[16.0, 16.0], altitude:7.0, argb:0x8001_0180 };
    let mut overlay = OverlayState::default();
    assert!(overlay.plan(false, &[cell]).unwrap().is_empty());
    assert_eq!(overlay.plan(true, &[cell]).unwrap(), vec![OverlayEffect::Register { renderer_slot:32, layer:5, priority:1000 }, OverlayEffect::Mark(cell)]);
    assert_eq!(overlay.plan(true, &[cell]).unwrap(), vec![OverlayEffect::Mark(cell)]);
    assert!(overlay.plan(false, &[]).unwrap().is_empty());
    assert_eq!(overlay.plan(true, &[cell]).unwrap(), vec![OverlayEffect::Mark(cell)]);
}
