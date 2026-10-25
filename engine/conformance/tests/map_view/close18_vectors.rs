// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn v35_display_flags_default_opacity_none_and_filter_only_override() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    render.zone_default = 96;
    res.raw
        .as_mut()
        .unwrap()
        .0
        .retain(|p| p.id != TERRAIN_MODE && p.id != TERRAIN_MODE_SECOND);
    for (view_id, e3, e6, level, plumbing, water, alpha) in [
        (1, None, None, 1, false, true, 96),
        (2, Some(1), Some(255), 3, true, false, 255),
        (3, Some(2), Some(0), 2, false, false, 0),
        (4, Some(1), None, 3, true, false, 96),
    ] {
        if let Some(e3) = e3 {
            set_property(res.raw.as_mut().unwrap(), TERRAIN_MODE, RawValue::U32(e3));
        }
        res.raw.as_mut().unwrap().0.retain(|p| p.id != 0x4a0b_47e6);
        if let Some(e6) = e6 {
            set_property(res.raw.as_mut().unwrap(), 0x4a0b_47e6, RawValue::U8(e6));
        }
        if view_id == 4 {
            set_property(res.raw.as_mut().unwrap(), TERRAIN_MODE_SECOND, RawValue::U32(2));
        }
        render.settings.clear();
        view.select_with(view_id, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
        assert_eq!(
            &render.settings[..3],
            &[
                RenderSetting::CityLevel(level),
                RenderSetting::PlumbingCoverage(plumbing),
                RenderSetting::SurfaceWater(water),
            ]
        );
        assert_eq!(render.zone_current, alpha);
        if view_id == 4 {
            assert!(render.settings.contains(&RenderSetting::TerrainMode {
                property: TERRAIN_MODE_SECOND,
                mode: 2
            }));
        }
    }
    view.select_with(0, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(view.source(), 0);
    assert!(view.selection_fields().is_none());
    assert!(render.cells.last().unwrap().coordinates.is_empty());
    assert!(render.settings.contains(&RenderSetting::CityLevel(1)));
    assert!(render.water_current);
    assert_eq!(render.zone_current, 96);
    let refreshes = (render.water_refreshes, render.zone_refreshes);
    res.raw
        .as_mut()
        .unwrap()
        .0
        .retain(|p| ![TERRAIN_MODE, TERRAIN_MODE_SECOND, 0x4a0b_47e6].contains(&p.id));
    view.select_with(5, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!((render.water_refreshes, render.zone_refreshes), refreshes);
}

#[test]
fn v01_scale_presence_defaults_shift_and_selection_reset() {
    for scale in [None, Some(0), Some(1), Some(0x1000000)] {
        let f = ScaleFactors::from_scale(scale);
        assert_eq!([0, 255, 256, 65535].map(|v| f.unsigned_index(v)), [0, 0, 1, 255]);
        assert_eq!(
            [-32768, -257, -256, -1, 0, 255, 256, 32767].map(|v| f.signed_index(v)),
            [0, 126, 127, 127, 128, 128, 129, 255]
        );
    }
    let f = ScaleFactors::from_scale(Some(256));
    assert_eq!(
        [0, 1, 255, 256, 512].map(|v| f.unsigned_index(v)),
        [0, 0, 254, 255, 255]
    );
    assert_eq!(
        [-512, -1, 0, 1, 256].map(|v| f.signed_index(v)),
        [0, 127, 128, 128, 255]
    );
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(8);
    sims.grid.values = GridValues::I16(vec![-512, 256]);
    set_property(res.raw.as_mut().unwrap(), SCALE, RawValue::U32(256));
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    let p = view.owned_palettes().unwrap().premultiplied.clone();
    assert_eq!(render.bitmaps.last().unwrap(), &[p.lookup(0), p.lookup(255)]);
    res.raw.as_mut().unwrap().0.retain(|p| p.id != SCALE);
    view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(render.bitmaps.last().unwrap(), &[p.lookup(126), p.lookup(129)]);
}

#[test]
fn v34_developer_empty_shared_layers_debug_order_and_temporary_interpolation() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(64);
    for (i, layer) in [None, Some(1), Some(99)].into_iter().enumerate() {
        if let Some(layer) = layer {
            set_property(res.raw.as_mut().unwrap(), LAYER, RawValue::U32(layer));
        }
        view.select_with(i as u32 + 1, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
        assert!(sims.reads.is_empty());
        assert!(render.bitmaps.last().unwrap().iter().all(|p| *p == 0));
    }
    sims.grid = RawMapGrid {
        width: 2,
        height: 2,
        cell_size: [32.0; 2],
        values: GridValues::I16(vec![0, -12, -29536, 0]),
    };
    for layer in [3, 4] {
        set_property(res.raw.as_mut().unwrap(), LAYER, RawValue::U32(layer));
        view.select_with(layer + 10, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
    }
    let shared = render.bitmaps.last().unwrap().clone();
    assert_eq!(&render.bitmaps[render.bitmaps.len() - 2], &shared);
    sims.debug_entries = Some(vec![[1, 0, 2], [0, 1, -1], [1, 0, 3000]]);
    set_property(res.raw.as_mut().unwrap(), LAYER, RawValue::U32(6));
    set_property(res.raw.as_mut().unwrap(), INTERPOLATE, RawValue::Bool(true));
    sims.cells = [4, 4];
    view.select_with(20, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(render.bitmaps.last().unwrap(), &shared);
    assert!(view.selection_fields().unwrap().interpolate);
    set_property(res.raw.as_mut().unwrap(), LAYER, RawValue::U32(2));
    view.set_developer_debug_flag(true);
    view.select_with(21, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(render.bitmaps.last().unwrap(), &shared);
    sims.debug_entries = Some(vec![]);
    view.draw_with(&mut render, &mut sims).unwrap();
    let p = view.owned_palettes().unwrap().premultiplied.lookup(128);
    assert_eq!(render.bitmaps.last().unwrap(), &vec![p; 4]);
    sims.debug_entries = None;
    assert_eq!(
        view.draw_with(&mut render, &mut sims).unwrap(),
        OwnerUpdate::DrawNothing
    );
}
