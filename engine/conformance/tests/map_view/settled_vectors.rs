// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn record(id: u64, radius: f32, position: [f32; 3]) -> HighlightRecord {
    HighlightRecord {
        occupant: Some(id),
        radius,
        position,
        effect: None,
    }
}
fn bytes(width: usize, height: usize, values: Vec<u8>) -> RawMapGrid {
    RawMapGrid {
        width,
        height,
        cell_size: [32.0, 16.0],
        values: GridValues::U8(values),
    }
}
fn ubytes(grid: RawMapGrid) -> Vec<u8> {
    match grid.values {
        GridValues::U8(v) => v,
        _ => panic!("unsigned grid expected"),
    }
}

#[test]
fn v17_building_retention_and_signed_pointer_order() {
    let (mut view, _, _, mut render, _) = inputs(1);
    assert!(view
        .reconcile_buildings(
            vec![record(1, 10.0, [1.0; 3]), record(0xfffffff0, 5.0, [0.0; 3])],
            &mut render
        )
        .unwrap());
    assert_eq!(view.retained_highlights()[0].occupant, Some(0xfffffff0));
    assert!(!view
        .reconcile_buildings(
            vec![record(1, 20.0, [2.0; 3]), record(0xfffffff0, 9.0, [3.0; 3])],
            &mut render
        )
        .unwrap());
    assert_eq!(view.retained_highlights()[1].radius, 10.0);
    assert_eq!(view.retained_highlights()[1].position, [1.0; 3]);
    assert_eq!(render.occupants.iter().filter(|(_, on, _)| *on).count(), 2);
    assert_eq!(render.released_occupants.len(), 2); // temporary references only
}

#[test]
fn v18_multiplicity_zero_radius_and_empty_reconciliation() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.reconcile_buildings(
        vec![
            record(1, 0.0, [0.0; 3]),
            record(1, 0.0, [0.0; 3]),
            record(2, 0.0, [0.0; 3]),
        ],
        &mut render,
    )
    .unwrap();
    assert!(view
        .reconcile_buildings(vec![record(2, 0.0, [0.0; 3]), record(1, 0.0, [0.0; 3])], &mut render)
        .unwrap());
    assert_eq!(view.retained_highlights().len(), 2);
    set_property(res.raw.as_mut().unwrap(), HIGHLIGHT, RawValue::U32(0));
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert!(view.retained_highlights().is_empty());
    assert!(!view.reconcile_buildings(vec![], &mut render).unwrap());
    for (kind, building_kind) in [
        (6, BuildingKind::GarbageBuilding),
        (7, BuildingKind::PowerPlant),
        (8, BuildingKind::WaterProducer),
        (9, BuildingKind::TransitSwitch),
    ] {
        assert_eq!(
            collect_highlights(
                kind,
                &[RawBuilding {
                    id: 1,
                    kind: building_kind,
                    coverage: Coverage::default()
                }]
            )
            .unwrap()
            .candidates[0]
                .radius,
            HighlightRadius::Value(0.0)
        );
    }
}

#[test]
fn v19_ramp_endpoints_duplicates_and_supplied_order() {
    let p = build_palettes(&[2, 0, 4, 0xffffffff, 4, 0xff000000, 6, 0xff0000ff]).unwrap();
    assert_eq!(
        &p.straight.0[..7],
        &[0, 0, 0, 0x80808080, 0xff000000, 0xff000080, 0xff0000ff]
    );
    assert!(p.straight.0[6..].iter().all(|v| *v == 0xff0000ff));
    let descending = build_palettes(&[4, 1, 2, 2, 6, 3]).unwrap();
    assert_eq!(descending.straight.lookup(2), 2);
}

#[test]
fn v20_channel_half_rounding_and_premultiplication() {
    for colors in [[0, 0x01010101], [0x01010101, 0]] {
        assert_eq!(
            build_palettes(&[0, colors[0], 2, colors[1]])
                .unwrap()
                .straight
                .lookup(1),
            0x01010101
        );
    }
    assert_eq!(premultiply_argb(0xffffffff), 0xfffefefe);
    assert_eq!(premultiply_argb(0x80808080), 0x80404040);
}

#[test]
fn v21_center_aligned_resampling_and_two_stage_rounding() {
    assert_eq!(
        ubytes(resample_grid(&bytes(1, 2, vec![0, 100]), [1, 4]).unwrap()),
        [0, 25, 75, 100]
    );
    let mut signed = bytes(1, 2, vec![]);
    signed.values = GridValues::I8(vec![-100, 100]);
    assert_eq!(ubytes(resample_grid(&signed, [1, 4]).unwrap()), [28, 78, 178, 228]);
    assert_eq!(
        ubytes(resample_grid(&bytes(1, 1, vec![71]), [3, 4]).unwrap()),
        vec![71; 12]
    );
    assert_eq!(
        ubytes(resample_grid(&bytes(2, 2, vec![0, 1, 1, 2]), [3, 3]).unwrap())[4],
        0 // QV16: vector 21 now requires the literal truncated-step result.
    );
    assert_eq!(
        ubytes(resample_grid(&bytes(2, 2, vec![0, 1, 1, 2]), [1, 1]).unwrap()),
        [2] // Exact half on both axes distinguishes two-stage rounding.
    );
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(2);
    set_property(res.raw.as_mut().unwrap(), INTERPOLATE, RawValue::Bool(true));
    sims.grid = bytes(1, 2, vec![0, 100]);
    sims.cells = [1, 4];
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    let p = &view.owned_palettes().unwrap().premultiplied;
    assert_eq!(render.bitmaps[0], [0, 25, 75, 100].map(|i| p.lookup(i)));
    assert_eq!(render.bitmap_cell_sizes, [[256.0, 32.0]]);
}

#[test]
fn v22_float_low_byte_and_masked_invalid() {
    let values = [
        -1.75,
        -0.75,
        255.75,
        256.75,
        511.75,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        9223372036854775808.0,
    ];
    assert_eq!(
        values.map(|v| float_index(v).unwrap()),
        [255, 0, 255, 0, 255, 0, 0, 0, 0]
    );
}

#[test]
fn v23_overlay_alpha_inclusion_coordinates_and_empty_replacement() {
    let mut p = RawPalette([0; 256]);
    p.0[..4].copy_from_slice(&[0x00ffffff, 0x01000000, 0xff000000, 0]);
    let grid = bytes(2, 2, vec![0, 1, 2, 3]);
    let cells = marked_cells(&grid, &p).unwrap();
    assert_eq!(cells.coordinates, [[0, 1], [1, 0]]);
    assert_eq!(cells.colors, [0x01000000, 0xff000000]);
    assert_eq!(cells.cell_size, [32.0, 16.0]); // first cell world [0,32] x [16,32]
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(2);
    view.set_overlay_enabled(true);
    sims.grid = grid;
    set_property(
        res.raw.as_mut().unwrap(),
        RAMP,
        RawValue::U32List(vec![0, 0x00ffffff, 255, 0x00ffffff]),
    );
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(render.cells.len(), 1);
    assert!(render.cells[0].coordinates.is_empty() && render.cells[0].colors.is_empty());
    let mut signed = bytes(1, 2, vec![]);
    signed.values = GridValues::I8(vec![-128, -127]);
    assert_eq!(marked_cells(&signed, &p).unwrap().coordinates, [[0, 1]]);
}

#[test]
fn v24_lot_masks_means_and_block_presence() {
    for (input, expected) in [
        (vec![0, 0, 1, 1], vec![1; 4]),
        (vec![1, 2], vec![2; 2]),
        (vec![0, 0], vec![1; 2]),
    ] {
        for lot_type in [3, 35] {
            let mut grid = bytes(1, input.len(), input.clone());
            let lot = MapLot {
                lot_type,
                cells: [0, 0, 0, input.len() - 1],
            };
            block_lots(&mut grid, 4, &[lot]).unwrap();
            assert_eq!(ubytes(grid.clone()), input);
            block_lots(&mut grid, 8, &[lot]).unwrap();
            assert_eq!(ubytes(grid), expected);
        }
    }
    let raw = RawExemplar(vec![Property::new(BLOCK_BY_LOT, RawValue::U32(0))]);
    assert!(raw.selection_fields().unwrap().interpolate);
    let mut grid = bytes(1, 3, vec![0, 0, 4]);
    block_lots(
        &mut grid,
        8,
        &[
            MapLot {
                lot_type: 3,
                cells: [0, 0, 0, 1],
            },
            MapLot {
                lot_type: 3,
                cells: [0, 0, 0, 2],
            },
        ],
    )
    .unwrap();
    assert_eq!(ubytes(grid), [1, 1, 1]); // 4/3, read original zeros after first lot.
}

#[test]
fn v25_layer_five_signed_population_unsigned_capacity() {
    let pop = WordGrid {
        width: 2,
        height: 2,
        shift: 1,
        words: vec![3, 7, 65535, 9],
    };
    let cap = WordGrid {
        words: vec![8, 0, 2, 4],
        ..pop.clone()
    };
    assert_eq!(
        capacity_percentage(&pop, &cap, [4, 4]).unwrap().values,
        GridValues::I16(vec![37, 0, -50, 225])
    );
    let fine = WordGrid {
        width: 2,
        height: 2,
        shift: 0,
        words: vec![65535, 0, 0, 0],
    };
    let cap = WordGrid {
        width: 1,
        height: 1,
        shift: 1,
        words: vec![2],
    };
    assert_eq!(
        capacity_percentage(&fine, &cap, [2, 2]).unwrap().values,
        GridValues::I16(vec![-50])
    );
    let coarse = WordGrid {
        width: 1,
        height: 1,
        shift: 2,
        words: vec![3],
    };
    assert_eq!(
        capacity_percentage(
            &coarse,
            &WordGrid {
                words: vec![8],
                ..coarse.clone()
            },
            [4, 4]
        )
        .unwrap()
        .values,
        GridValues::I16(vec![37; 4])
    );
}

#[test]
fn v26_crime_placement_rotation_clipping_and_order() {
    assert_eq!(incident_center([11, 7], 64, 128), [5, 3]);
    assert_eq!(blend_incident(0xff000000, 0xffffffff), 0xfffefefe);
    assert_eq!(blend_incident(0xff000000, 0x80808080), 0xff404040);
    let mut pixels = vec![0xff000000; 64 * 8];
    let icon = IncidentIcon {
        handle: 1,
        width: 2,
        height: 2,
        pixels: vec![0xffffffff; 4],
    };
    blit_incident(&mut pixels, [64, 8], &icon, [5, 3], 0).unwrap();
    assert_eq!(pixels.iter().filter(|p| **p != 0xff000000).count(), 4);
    assert_eq!(pixels[4 * 8 + 2], 0xfffefefe);
    let second = IncidentIcon {
        pixels: vec![0x80808080; 4],
        ..icon.clone()
    };
    blit_incident(&mut pixels, [64, 8], &second, [5, 3], 0).unwrap();
    assert_eq!(pixels[4 * 8 + 2], blend_incident(0xfffefefe, 0x80808080));
    for turn in 0..4 {
        blit_incident(&mut pixels, [64, 8], &icon, [-1, -1], turn).unwrap();
    }
}

#[test]
fn v27_travel_narrowing_and_absence_retention() {
    for (list, expected) in [
        (vec![256, 511, 258], vec![0, 255, 2]),
        (vec![7; 9], vec![7; 8]),
        (vec![5; 256], vec![]),
        (vec![5; 257], vec![5]),
    ] {
        let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
        set_property(res.raw.as_mut().unwrap(), TRAVEL_TYPES, RawValue::U32List(list));
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(view.retained_travel_types(), expected);
        res.raw.as_mut().unwrap().0.retain(|p| p.id != TRAVEL_TYPES);
        view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(view.retained_travel_types(), expected);
    }
}

#[test]
fn v28_absence_and_terrain() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    let palette = view.owned_palettes().cloned();
    res.raw = Some(RawExemplar(vec![Property::new(CHILD_FLAG, RawValue::Bool(false))]));
    view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(view.owned_palettes(), palette.as_ref());
    let config = view.config().unwrap();
    assert_eq!((config.source, config.layer, config.highlight_kind), (0, 1, 0));
    assert!(!config.interpolate && config.block_by_lot.is_none());
    assert_eq!(
        HighlightStyle::from_exemplar(res.raw.as_ref().unwrap()).unwrap(),
        HighlightStyle::default()
    );
    for (e3, f1, expected) in [(1, Some(99), 3), (99, None, 1)] {
        let mut raw = RawExemplar(vec![Property::new(TERRAIN_MODE, RawValue::U32(e3))]);
        if let Some(value) = f1 {
            raw.0.push(Property::new(TERRAIN_MODE_SECOND, RawValue::U32(value)));
        }
        assert_eq!(raw.terrain_mode(TERRAIN_MODE_SECOND), Ok(expected));
    }
}

#[test]
fn v29_wrapped_identity() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    assert_eq!(
        view.select_with(0xb5f49800, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::Selected { highlight_boundary: None })
    );
    assert!(res.keys.is_empty());
    view.select_with(0xb5f49801, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(res.keys.last().unwrap().instance, 1);
}

#[test]
fn v30_init_latch_and_accumulated_hooks() {
    let mut view = MapView::new(0, 0);
    let mut win = Windows {
        missing_server: true,
        ..Windows::default()
    };
    assert_eq!(view.init_with(&mut win), Err(OwnerError::MissingMessageServer));
    assert_eq!(view.init_with(&mut win), Ok(()));
    assert_eq!(
        win.effects,
        vec![WindowEffect::Lifecycle(Effect::WindowGeneratorInit); 2]
    );
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    view.unhook_update_messages();
    let first = view.hook_update_messages().unwrap();
    let second = view.hook_update_messages().unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 3);
    assert_eq!(&second[1..], &vec![Effect::Subscribe(0x2a09b21a); 2]);
}

#[test]
fn v31_clock_wrap_regression_and_thresholds() {
    for (last, now, due) in [(0xffffff00u32, 0x2e8u32, true), (1000, 999, true), (0, 999, false)] {
        let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
        render.clock = last;
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        view.notify(subscription(1).message, None).unwrap();
        render.clock = now.wrapping_add(7);
        assert_eq!(view.tick_with(u64::from(now), &mut render, &mut sims).unwrap().due, due);
        if due {
            assert_eq!(view.last_redraw_ms(), u64::from(now.wrapping_add(7)));
        }
    }
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(76);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims)
        .unwrap_err();
    assert!(!view.tick_with(249, &mut render, &mut sims).unwrap().due);
    assert!(view.tick_with(250, &mut render, &mut sims).unwrap().due);
}

#[test]
fn v32_update_commit_points() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    render.clock = 10;
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    view.notify(subscription(1).message, None).unwrap();
    render.missing_child = true;
    render.clock = 20;
    view.draw_with(&mut render, &mut sims).unwrap();
    assert!(!view.dirty());
    assert_eq!(view.last_redraw_ms(), 20);
    assert_eq!(render.bitmaps.len(), 1);
    render.missing_child = false;
    res.raw = Some(raw(64));
    sims.missing_developer = true;
    render.clock = 30;
    view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(view.last_redraw_ms(), 20);
    assert_eq!(render.bitmaps.len(), 1);
    res.raw = Some(raw(20));
    view.select_with(3, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(render.bitmaps.last(), Some(&vec![0]));
    assert_eq!(view.last_redraw_ms(), 30);
    res.raw = Some(raw(0));
    render.clock = 40;
    view.select_with(4, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(render.clears, 1);
    assert_eq!(view.last_redraw_ms(), 40);
    view.deactivate();
    render.clock = 50;
    assert_eq!(view.draw_with(&mut render, &mut sims), Ok(OwnerUpdate::Inactive));
    assert_eq!(view.last_redraw_ms(), 40);
}

#[test]
fn v33_notification_compares_both_records() {
    let mut render = Render {
        clock: 123,
        ..Render::default()
    };
    let mut view = MapView::from_clock(&mut render);
    assert_eq!(view.current_view(), Some(0));
    assert_eq!(view.last_redraw_ms(), 123);
    view.notify_values(5, 5);
    assert!(!view.dirty());
    view.notify_values(5, 6);
    assert!(view.dirty());
}
