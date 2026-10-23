// SPDX-License-Identifier: GPL-3.0-or-later
//! map-view Ãƒâ€šÃ‚Â§10 and owner interface vectors. No game/renderer implementation.
use fc4_core::map_view::*;

fn raw(source: u32) -> RawExemplar {
    RawExemplar(vec![
        Property::new(SOURCE, RawValue::U32(source)),
        Property::new(RAMP, RawValue::U32List(vec![0, 0xff00_0000, 255, 0xffff_ffff])),
        Property::new(INTERPOLATE, RawValue::Bool(false)),
        Property::new(HIGHLIGHT, RawValue::U32(9)),
        Property::new(0x4a0b_47e2, RawValue::Bool(false)),
        Property::new(TERRAIN_MODE, RawValue::U32(0)),
        Property::new(TERRAIN_MODE_SECOND, RawValue::U32(2)),
        Property::new(CHILD_FLAG, RawValue::Bool(true)),
        Property::new(0x4a0b_47f2, RawValue::U32List(vec![0xff00_0000, 0xffff_ffff])),
        Property::new(0x4a0b_47f3, RawValue::U32List(vec![11, 22])),
        Property::new(0x4a0b_47f4, RawValue::U32List(vec![33])),
    ])
}

struct Resource {
    raw: Option<RawExemplar>,
    keys: Vec<ExemplarKey>,
}
impl Resources for Resource {
    fn exemplar(&mut self, key: ExemplarKey) -> Result<Option<RawExemplar>, OwnerError> {
        self.keys.push(key);
        Ok(self.raw.clone())
    }
}

#[derive(Default)]
struct Windows {
    effects: Vec<WindowEffect>,
    visible: bool,
    deny: bool,
    created: Vec<(u32, u32)>,
    fail: bool,
    traffic_toggle: bool,
}
impl WindowSystem for Windows {
    fn effect(&mut self, effect: WindowEffect) -> Result<(), OwnerError> {
        if self.fail {
            return Err(OwnerError::Foreign(123));
        }
        self.effects.push(effect);
        Ok(())
    }
    fn visible(&self, panel: u32) -> bool {
        assert_eq!(panel, 0x8a28_71c3);
        self.visible
    }
    fn button_down(&self, button: u32) -> bool {
        assert_eq!(button, 0x8001);
        self.traffic_toggle
    }
    fn tutorial_allows(&mut self, command: u32, _button: u32) -> Result<bool, OwnerError> {
        assert_eq!(command, 0x2872_59f6);
        Ok(!self.deny)
    }
    fn create_custom_view(&mut self, class: u32, iid: u32) -> Result<u64, OwnerError> {
        self.created.push((class, iid));
        Ok(42)
    }
}

#[derive(Default)]
struct Render {
    settings: Vec<RenderSetting>,
    bitmaps: Vec<Vec<u32>>,
    clears: usize,
    fail: bool,
    own_cells: bool,
}
impl Renderer for Render {
    fn render(&mut self, command: RenderCommand<'_>) -> Result<(), OwnerError> {
        if self.fail {
            return Err(OwnerError::Foreign(456));
        }
        match command {
            RenderCommand::Setting(setting) => self.settings.push(setting),
            RenderCommand::ClearMap => self.clears += 1,
            RenderCommand::Bitmap {
                class,
                iid,
                bits,
                width,
                height,
                cell_size,
                own_cells,
                pixels,
            } => {
                assert_eq!((class, iid, bits), (0x2073_2180, 0xc470_d325, 32));
                assert_eq!((width, height, cell_size), (2, 1, [4.0, 4.0]));
                self.own_cells = own_cells;
                self.bitmaps.push(pixels.to_vec());
            }
        }
        Ok(())
    }
}

struct Sims {
    grid: RawMapGrid,
    cells: [usize; 2],
    reads: Vec<SourceSelection>,
    polls: usize,
}
impl Simulators for Sims {
    fn grid(&mut self, request: SourceSelection) -> Result<RawMapGrid, OwnerError> {
        self.reads.push(request);
        Ok(self.grid.clone())
    }
    fn city_cells(&mut self) -> Result<[usize; 2], OwnerError> {
        Ok(self.cells)
    }
    fn buildings(&mut self, request: HighlightRequest) -> Result<Vec<RawBuilding>, OwnerError> {
        assert_eq!(request, highlight_request(9).unwrap());
        self.polls += 1;
        Ok(Vec::new())
    }
}

fn inputs(source: u32) -> (MapView, Resource, Windows, Render, Sims) {
    let mut view = MapView::new(0, 7);
    view.activate();
    (
        view,
        Resource {
            raw: Some(raw(source)),
            keys: Vec::new(),
        },
        Windows::default(),
        Render::default(),
        Sims {
            grid: RawMapGrid {
                width: 2,
                height: 1,
                cell_size: [4.0, 4.0],
                values: GridValues::I8(vec![-128, 127]),
            },
            cells: [2, 1],
            reads: Vec::new(),
            polls: 0,
        },
    )
}

#[test]
fn mayor_rating_owner_builds_and_draws_both_extremes() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::Selected {
            highlight_boundary: Some(Unsupported::HighlightComparison)
        })
    );
    assert_eq!(
        res.keys,
        vec![ExemplarKey {
            resource_type: 0x6534_284a,
            group: 0x690f_693f,
            instance: 0x4a0b_6801
        }]
    );
    assert_eq!(view.current_view(), Some(1));
    assert_eq!(view.update_subscription(), Some(subscription(1)));
    assert_eq!(render.bitmaps, vec![vec![0xff00_0000, 0xffff_ffff]]);
    assert_eq!(view.owned_palettes().unwrap().straight.lookup(128), 0xff80_8080);
    assert!(matches!(
        win.effects.last(),
        Some(WindowEffect::Lifecycle(Effect::Subscribe(0x2a09_b21a)))
    ));
    assert!(win.effects.contains(&WindowEffect::ChildFlag {
        child: 0x4203,
        iid: 0xca31_8385,
        value: true
    }));
}

#[test]
fn conformance_index_conflict_is_explicit_qv5() {
    // Ãƒâ€šÃ‚Â§10.1 cannot be asserted as a conversion: the signed formula gives 64,
    // while its vector requires 65. Neither conflicting result is fabricated.
    assert_eq!((512u64 * (0xffff00 / 1024)) >> 16, 127);
    assert_eq!(128 + ((-512i64 * (0x7fff00 / 1024)) >> 16), 64);
    assert_eq!(u16_index(512, 1024), Err(Unsupported::SixteenBitConflict));
    assert_eq!(i16_index(-512, 1024), Err(Unsupported::SixteenBitConflict));
}

#[test]
fn pollution_throttle_vector_reaches_due_tick_without_faking_conversion() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(8);
    sims.grid.values = GridValues::I16(vec![0, 512]);
    // Selection has installed source/config before the unresolved conversion.
    assert_eq!(
        view.select_with(8, &mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::SixteenBitConflict))
    );
    view.hook_update_messages().unwrap();
    view.notify(0xfa6d_d0bb, None).unwrap();
    assert!(!view.tick_with(100, &mut render, &mut sims).unwrap().due);
    view.notify(0xfa6d_d0bb, None).unwrap();
    assert!(!view.tick_with(300, &mut render, &mut sims).unwrap().due);
    let tick = view.tick_with(1000, &mut render, &mut sims).unwrap();
    assert!(tick.due);
    assert_eq!(
        tick.update,
        Some(Err(OwnerError::Unsupported(Unsupported::SixteenBitConflict)))
    );
    assert_eq!(sims.polls, 4); // selection plus every tick, including throttled ticks
    assert!(view.dirty());
    assert_eq!(view.last_redraw_ms(), 0);
    assert!(render.bitmaps.is_empty());
}

#[test]
fn successful_redraw_acknowledges_once_and_failed_present_retains_retry() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    view.notify(subscription(1).message, None).unwrap();
    assert!(!view.tick_with(999, &mut render, &mut sims).unwrap().due);
    render.fail = true;
    assert_eq!(
        view.tick_with(1000, &mut render, &mut sims).unwrap().update,
        Some(Err(OwnerError::Foreign(456)))
    );
    assert_eq!(view.last_redraw_ms(), 0);
    render.fail = false;
    assert_eq!(
        view.tick_with(1001, &mut render, &mut sims).unwrap().update,
        Some(Ok(OwnerUpdate::Drawn))
    );
    assert!(!view.dirty());
    assert_eq!(view.last_redraw_ms(), 1001);
    assert!(!view.tick_with(2001, &mut render, &mut sims).unwrap().due);
    assert_eq!(render.bitmaps.len(), 2);
}

#[test]
fn residential_desirability_selection_commits_but_draws_nothing() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(20);
    view.select_with(0x14, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(view.current_view(), Some(0x14));
    assert!(sims.reads.is_empty());
    assert!(render.bitmaps.is_empty());
    assert_eq!(render.clears, 0); // preservation/clearing of an OLD bitmap remains QV9
}

#[test]
fn custom_owned_handle_shutdown_precedes_subscription_drop_and_missing_lookup() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    res.raw
        .as_mut()
        .unwrap()
        .0
        .push(Property::new(0x4a0b_47eb, RawValue::U32(99)));
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(win.created, vec![(99, 0x8a17_5d6b)]);
    win.effects.clear();
    res.raw = None;
    assert_eq!(
        view.select_with(2, &mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::MissingExemplar)
    );
    assert_eq!(win.effects[0], WindowEffect::Lifecycle(Effect::ShutdownCustomView(42)));
    assert_eq!(win.effects[1], WindowEffect::Lifecycle(Effect::ReleaseCustomView(42)));
    assert!(matches!(
        win.effects[2],
        WindowEffect::Lifecycle(Effect::DropUpdateSubscriptions { .. })
    ));
    assert_eq!(view.current_view(), Some(1));
    assert_eq!(view.update_subscription(), None);
    win.effects.clear();
    view.select_with(3, &mut res, &mut win, &mut render, &mut sims)
        .unwrap_err();
    assert_eq!(win.effects.len(), 1); // no duplicate Release on next failure
}

#[test]
fn direct_buttons_aliases_tutorial_reset_and_legend() {
    for (button, expected) in [
        (0x5001, 0),
        (0x5002, 5),
        (0x5003, 3),
        (0x5004, 27),
        (0x5006, 10),
        (0x5007, 7),
        (0x5009, 32),
        (0x500a, 2),
        (0x500b, 1),
        (0x500c, 30),
        (0x500d, 28),
        (0x500e, 8),
        (0x500f, 24),
        (0x5010, 25),
        (0x5011, 26),
    ] {
        assert_eq!(button_action(button), ButtonAction::View(expected));
        assert_eq!(button_action(button + 0x100), ButtonAction::View(expected));
    }
    for (i, expected) in [40, 41, 42, 50, 51, 52, 53, 54, 60, 61, 62, 63].into_iter().enumerate() {
        assert_eq!(button_action(0x6001 + i as u32), ButtonAction::View(expected));
        assert_eq!(button_action(0x6101 + i as u32), ButtonAction::View(expected));
    }
    for button in 0x8003..=0x800c {
        assert_eq!(button_action(button), ButtonAction::View(0x4c + button - 0x8003));
    }
    for button in [0x5005, 0x5008, 0x8002] {
        assert_eq!(button_action(button), ButtonAction::Group(button));
    }
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    win.deny = true;
    assert_eq!(
        view.button_with(0x510b, &mut res, &mut win, &mut render, &mut sims),
        Ok(None)
    );
    assert!(res.keys.is_empty());
    win.deny = false;
    let boundary = view
        .button_with(0x510b, &mut res, &mut win, &mut render, &mut sims)
        .unwrap()
        .unwrap();
    assert_eq!(boundary.boundary, Unsupported::PanelsQV3);
    assert_eq!(boundary.panels, MAP_PANELS);
    assert!(win.effects.contains(&WindowEffect::ExcludeOtherButtons(0x510b)));
    let legend = Legend::from_exemplar(res.raw.as_ref()).unwrap();
    assert_eq!(
        (legend.panel, legend.string_group, legend.title),
        (0x8a90_9e00, 0x6a23_1eaa, 33)
    );
    assert_eq!(legend.rows, vec![(0xff00_0000, 11), (0xffff_ffff, 22)]);
    assert!(win.effects.contains(&WindowEffect::Legend(legend)));
    win.visible = true;
    for (button, value) in [(0x8101, true), (0x8100, false)] {
        view.button_with(button, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
        assert_eq!(view.two_way_toggle(), Some(value));
    }
    assert_eq!(res.keys.len(), 1); // literal Reset calls same-view guarded SetDataView
}

#[test]
fn window_lifecycle_commands_and_open_panel_matrix_are_explicit() {
    let (mut view, _, mut win, _, _) = inputs(1);
    view.init_with(&mut win).unwrap();
    assert_eq!(win.effects.len(), 8);
    view.window_message(DEACTIVATE, None, &mut win).unwrap();
    assert!(!view.active());
    view.window_message(ACTIVATE, None, &mut win).unwrap();
    assert!(view.active());
    view.window_message(0x8a28_71d5, None, &mut win).unwrap();
    assert_eq!(win.effects.last(), Some(&WindowEffect::PostCommand(0x6a93_5cf1)));
    for message in [0xca28_71f0, 0xca28_71f1, 0xa32c_acd4, 0x0a32_cac3] {
        assert_eq!(
            view.window_message(message, None, &mut win).unwrap().unwrap().boundary,
            Unsupported::PanelsQV3
        );
    }
    assert_eq!(Legend::from_exemplar(None).unwrap().title, 0x2a5e_854f);
    assert_eq!(view.panel_request(PanelRequest::Hide).request, PanelRequest::Hide);
}

#[test]
fn interpolation_is_only_required_for_a_coarser_grid_and_overlay_stays_qv10() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    res.raw
        .as_mut()
        .unwrap()
        .0
        .iter_mut()
        .find(|p| p.id == INTERPOLATE)
        .unwrap()
        .value = RawValue::Bool(true);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    sims.cells = [4, 2];
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::GridInterpolation))
    );
    sims.cells = [2, 1];
    view.set_overlay_enabled(true);
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::OverlayGeometryQV10))
    );
    assert_eq!(render.bitmaps.len(), 1);
}

#[test]
fn exact_ramps_premultiply_and_refuse_unknown_rounding_or_endpoint_rules() {
    let palette = build_palettes(&[0, 0x8000_0000, 255, 0x80ff_ffff]).unwrap();
    assert_eq!(palette.straight.lookup(127), 0x807f_7f7f);
    assert_eq!(palette.premultiplied.lookup(127), 0x8040_4040);
    for pairs in [&[0, 0, 255, 1][..], &[1, 0, 255, 0][..], &[0, 0, 0, 0, 255, 0][..]] {
        assert_eq!(build_palettes(pairs), Err(Unsupported::RampGeneration));
    }
}

#[test]
fn filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry() {
    let mut resource = raw(29);
    resource.0.extend([
        Property::new(OCCUPANT_TYPES, RawValue::U32List(vec![0, 7, 9])),
        Property::new(0x4a0b_47e1, RawValue::Bool(true)),
        Property::new(0x4a0b_47f0, RawValue::Bool(true)),
        Property::new(0x4a0b_47e6, RawValue::U8(3)),
    ]);
    let settings = resource.render_settings().unwrap();
    let RenderSetting::OccupantFilter(filter) = &settings[0] else {
        panic!("filter first")
    };
    assert_eq!(filter.types, vec![7, 9]);
    assert!(!filter.includes_type(7));
    assert!(filter.includes_type(8));
    assert!(settings.contains(&RenderSetting::TerrainMode {
        property: TERRAIN_MODE,
        mode: 1
    }));
    assert!(settings.contains(&RenderSetting::TerrainMode {
        property: TERRAIN_MODE_SECOND,
        mode: 2
    }));
    assert!(settings.contains(&RenderSetting::ForceEmptyLotBaseTextures(true)));
    assert!(settings.contains(&RenderSetting::ZoneOption(3)));
    let include = OccupantFilter {
        types: vec![7],
        list_present: true,
        exclude_listed: false,
        hide_empty_bases_and_excluded_decals: false,
    };
    assert!(include.includes_type(7));
    assert!(!include.includes_type(8));
    let all = OccupantFilter {
        types: vec![],
        list_present: false,
        ..include
    };
    assert!(all.includes_type(99));
    let empty = OccupantFilter {
        list_present: true,
        ..all
    };
    assert!(!empty.includes_type(99));
    let mut zeros = raw(1);
    zeros
        .0
        .push(Property::new(OCCUPANT_TYPES, RawValue::U32List(vec![0, 0])));
    let RenderSetting::OccupantFilter(empty) = &zeros.render_settings().unwrap()[0] else {
        panic!("filter first")
    };
    assert!(!empty.includes_type(99));
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(29);
    res.raw = Some(resource);
    sims.grid.values = GridValues::U8(vec![0, 255]);
    view.select_with(29, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert!(render.own_cells);
    assert_eq!(sims.reads, vec![select_source(29, 1, false).unwrap()]);
}

#[test]
fn inactive_same_view_and_foreign_failures_do_not_report_a_completed_selection() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.deactivate();
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::Inactive)
    );
    assert!(res.keys.is_empty());
    view.activate();
    render.fail = true;
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::Foreign(456))
    );
    assert_eq!(view.current_view(), None);
    assert_eq!(view.update_subscription(), None);
    assert!(win.created.is_empty());
    render.fail = false;
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    let effects = win.effects.len();
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::CurrentView)
    );
    assert_eq!(win.effects.len(), effects);
    assert_eq!(res.keys.len(), 2);
    let mut fresh = MapView::new(0, 0);
    win.fail = true;
    assert_eq!(fresh.init_with(&mut win), Err(OwnerError::Foreign(123)));
}

#[test]
fn malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries() {
    let mut malformed = raw(1);
    malformed.0.iter_mut().find(|p| p.id == 0x4a0b_47f3).unwrap().value = RawValue::U32List(vec![1]);
    assert_eq!(Legend::from_exemplar(Some(&malformed)), Err(Unsupported::LegendShape));
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    sims.grid.values = GridValues::I8(vec![0]);
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::BufferLength))
    );
    sims.grid.values = GridValues::U8(vec![0, 0]);
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::GridType))
    );
    sims.grid.values = GridValues::I8(vec![0, 0]);
    res.raw
        .as_mut()
        .unwrap()
        .0
        .push(Property::new(BLOCK_BY_LOT, RawValue::U32(0)));
    assert_eq!(
        view.select_with(2, &mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::LotBlockingQV4))
    );
    assert_eq!(
        view.select_with(0, &mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::FilterResetQV1))
    );
    let mut fresh = MapView::new(0, 0);
    fresh.activate();
    assert_eq!(fresh.draw_with(&mut render, &mut sims), Ok(OwnerUpdate::Cleared));
    assert_eq!(render.clears, 1);
}

#[test]
fn traffic_group_known_toggle_branch_and_show_reset_are_not_delegated() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(9);
    win.traffic_toggle = true;
    sims.grid.values = GridValues::U8(vec![0, 255]);
    view.button_with(0x5008, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(view.current_view(), Some(9));
    view.reset_with(&mut res, &mut win, &mut render, &mut sims).unwrap();
    win.traffic_toggle = false;
    // Beyond the explicitly known toggle branch, group memory remains QV3.
    assert_eq!(
        view.reset_with(&mut res, &mut win, &mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::GroupButtonQV3(0x5008)))
    );
    view.button_with(0x8001, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    win.visible = true;
    let count = win.effects.len();
    let result = view
        .show_with(PanelRequest::Large, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(result.request, PanelRequest::Large);
    assert_eq!(win.effects.len(), count + 2); // showing and visible reset each redraw legend
    view.set_overlay_enabled(true);
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::TrafficMarkersQV4))
    );
}
