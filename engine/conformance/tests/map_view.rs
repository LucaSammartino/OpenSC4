// SPDX-License-Identifier: GPL-3.0-or-later
//! map-view Ãƒâ€šÃ‚Â§10 and owner interface vectors. No game/renderer implementation.
use fc4_core::map_view::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

type Trace = Option<Arc<Mutex<Vec<&'static str>>>>;
fn trace(log: &Trace, entry: &'static str) {
    if let Some(log) = log {
        log.lock().unwrap().push(entry);
    }
}

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
        Property::new(0x4a0b_47f4, RawValue::U32(33)),
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
    factory_false: bool,
    primitive_false: bool,
    missing_server: bool,
    denied_controls: Vec<u32>,
    queries: Vec<(u32, u32)>,
    trace: Trace,
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
        match panel {
            0x8a28_71c3 => self.visible,
            0xaa32_bcd5 => true,
            _ => panic!("unexpected panel {panel:x}"),
        }
    }
    fn button_down(&self, button: u32) -> bool {
        assert_eq!(button, 0x8001);
        self.traffic_toggle
    }
    fn tutorial_allows(&mut self, command: u32, _button: u32) -> Result<bool, OwnerError> {
        assert_eq!(command, 0x2872_59f6);
        Ok(!self.deny)
    }
    fn create_custom_view(&mut self, class: u32, iid: u32) -> Result<Option<u64>, OwnerError> {
        self.created.push((class, iid));
        Ok((!self.factory_false).then_some(42))
    }
    fn primitive(&mut self, effect: WindowEffect) -> Result<bool, OwnerError> {
        match &effect {
            WindowEffect::InitCustomView(_) => trace(&self.trace, "init-custom"),
            WindowEffect::ChildFlag { .. } => trace(&self.trace, "child"),
            WindowEffect::Lifecycle(Effect::RegisterAnimation(_)) => trace(&self.trace, "hook"),
            _ => (),
        }
        self.effect(effect)?;
        Ok(!self.primitive_false)
    }
    fn message_server_present(&self) -> bool {
        !self.missing_server
    }
    fn query_control(&mut self, control: u32, iid: u32) -> Result<bool, OwnerError> {
        self.queries.push((control, iid));
        Ok(!self.denied_controls.contains(&control))
    }
}

#[derive(Default)]
struct Render {
    settings: Vec<RenderSetting>,
    bitmaps: Vec<Vec<u32>>,
    clears: usize,
    fail: bool,
    own_cells: bool,
    primitive_false: bool,
    custom_calls: Vec<u64>,
    creations: VecDeque<bool>,
    next_effect: u64,
    visuals: Vec<VisualLog>,
    occupants: Vec<(u64, bool, u32)>,
    released_occupants: Vec<u64>,
    acquisitions: VecDeque<bool>,
    clip: Option<ClipRect>,
    buffer_calls: Vec<bool>,
    trace: Trace,
}

#[derive(Debug, PartialEq)]
enum VisualLog {
    Create(String),
    Call(VisualEffect),
}
impl Renderer for Render {
    fn primitive(&mut self, command: RenderCommand<'_>) -> Result<bool, OwnerError> {
        self.render(command)?;
        Ok(!self.primitive_false)
    }
    fn update_custom_view(&mut self, handle: u64) -> Result<bool, OwnerError> {
        trace(&self.trace, "update-custom");
        self.custom_calls.push(handle);
        Ok(false)
    }
    fn create_visual_effect(&mut self, name: &str) -> Result<Option<u64>, OwnerError> {
        self.visuals.push(VisualLog::Create(name.into()));
        self.next_effect += 1;
        Ok(self.creations.pop_front().unwrap_or(true).then_some(self.next_effect))
    }
    fn visual_effect(&mut self, effect: VisualEffect) -> Result<bool, OwnerError> {
        self.visuals.push(VisualLog::Call(effect));
        Ok(false) // Normally returning transform/start/stop results are ignored.
    }
    fn highlight_occupant(&mut self, occupant: u64, highlight: bool, argument: u32) -> Result<bool, OwnerError> {
        self.occupants.push((occupant, highlight, argument));
        Ok(false)
    }
    fn release_occupant(&mut self, occupant: u64) -> Result<(), OwnerError> {
        self.released_occupants.push(occupant);
        Ok(())
    }
    fn acquire_bitmap(&mut self, flags: u32, width: usize, height: usize) -> Result<Option<ClipRect>, OwnerError> {
        assert_eq!(flags, 0x8040);
        self.buffer_calls.push(true);
        Ok(self
            .acquisitions
            .pop_front()
            .unwrap_or(true)
            .then_some(self.clip.unwrap_or(ClipRect {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            })))
    }
    fn release_bitmap(&mut self) -> Result<(), OwnerError> {
        self.buffer_calls.push(false);
        Ok(())
    }
    fn render(&mut self, command: RenderCommand<'_>) -> Result<(), OwnerError> {
        if matches!(&command, RenderCommand::Bitmap { .. }) {
            trace(&self.trace, "bitmap");
        }
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
                assert_eq!(width * height, pixels.len());
                assert_eq!(cell_size, [4.0, 4.0]);
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
    hotspots: VecDeque<Option<CrimeHotspot>>,
    altitude_calls: Vec<[f32; 2]>,
    footprints: Vec<u64>,
    trace: Trace,
}
impl Simulators for Sims {
    fn crime_hotspot(&mut self) -> Result<Option<CrimeHotspot>, OwnerError> {
        self.polls += 1;
        Ok(self.hotspots.pop_front().unwrap_or(None))
    }
    fn terrain_altitude(&mut self, x: f32, z: f32) -> Result<f32, OwnerError> {
        self.altitude_calls.push([x, z]);
        Ok(12.0)
    }
    fn occupant_cells(&mut self, occupant: u64) -> Result<[i32; 4], OwnerError> {
        self.footprints.push(occupant);
        Ok([2, 3, 3, 4])
    }
    fn city_world_width(&mut self) -> Result<f32, OwnerError> {
        Ok(256.0)
    }
    fn grid(&mut self, request: SourceSelection) -> Result<RawMapGrid, OwnerError> {
        self.reads.push(request);
        Ok(self.grid.clone())
    }
    fn city_cells(&mut self) -> Result<[usize; 2], OwnerError> {
        Ok(self.cells)
    }
    fn buildings(&mut self, request: HighlightRequest) -> Result<Vec<RawBuilding>, OwnerError> {
        trace(&self.trace, "highlights");
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
            hotspots: VecDeque::new(),
            altitude_calls: Vec::new(),
            footprints: Vec::new(),
            trace: None,
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
    assert_eq!(
        legend.rows,
        vec![
            LegendRow {
                index: 0,
                rgb: 0,
                string: Some(11)
            },
            LegendRow {
                index: 1,
                rgb: 0xffffff,
                string: Some(22)
            }
        ]
    );
    assert!(win.effects.contains(&WindowEffect::LegendTitle {
        group: legend.string_group,
        string: legend.title
    }));
    win.visible = true;
    for (button, value) in [(0x8101, true), (0x8100, false)] {
        view.button_with(button, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
        assert_eq!(view.two_way_toggle(), Some(value));
    }
    assert_eq!(res.keys.len(), 4); // one selection, one legend, two visible reset legends
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
    assert_eq!(
        Legend::from_exemplar(Some(&malformed)),
        Err(Unsupported::LegendStringRead(1))
    );
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
    assert_eq!(win.effects.len(), count + 48); // showing and visible reset each redraw legend
    view.set_overlay_enabled(true);
    assert_eq!(
        view.draw_with(&mut render, &mut sims),
        Err(OwnerError::Unsupported(Unsupported::TrafficMarkersQV4))
    );
}

fn set_property(raw: &mut RawExemplar, id: u32, value: RawValue) {
    raw.0.retain(|p| p.id != id);
    raw.0.push(Property::new(id, value));
}

fn legend_hide_prefix() -> Vec<WindowEffect> {
    let mut effects = Vec::new();
    for base in [0x8a90_9e00, 0x8a90_9e10] {
        for index in 0..9 {
            effects.push(WindowEffect::HideControl(base + index));
        }
    }
    effects.push(WindowEffect::InvalidateControl(0x8a90_9e00));
    effects
}

// §10.5: a visible reset refreshes a fresh legend, not the selection resource.
#[test]
fn qv12_selection_guards_and_visible_reset() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    view.deactivate();
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::Inactive)
    );
    assert!(res.keys.is_empty() && win.effects.is_empty() && render.bitmaps.is_empty());
    view.activate();
    view.button_with(0x500b, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    let polls = sims.polls;
    let hooks = view.update_subscription();
    res.keys.clear();
    win.effects.clear();
    win.created.clear();
    render.bitmaps.clear();
    win.visible = true;
    set_property(res.raw.as_mut().unwrap(), 0x4a0b_47f4, RawValue::U32(77));
    view.reset_with(&mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(res.keys, vec![exemplar_key(1).unwrap()]);
    assert_eq!(sims.polls, polls);
    assert_eq!(view.update_subscription(), hooks);
    assert!(win.created.is_empty() && render.bitmaps.is_empty());
    assert!(win.effects.contains(&WindowEffect::LegendTitle {
        group: 0x6a23_1eaa,
        string: 77
    }));
    assert!(!win.effects.iter().any(|e| matches!(e, WindowEffect::Lifecycle(_))));
}

// §10.6: the failed request keeps A's identity/source, after tearing down A.
#[test]
fn qv12_failed_button_replacement_remembers_requested_legend() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
    set_property(res.raw.as_mut().unwrap(), 0x4a0b_47eb, RawValue::U32(99));
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    res.raw = None;
    win.effects.clear();
    res.keys.clear();
    view.button_with(0x500a, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(view.current_view(), Some(1));
    assert_eq!(view.source(), 1);
    assert_eq!(view.update_subscription(), None);
    assert_eq!(res.keys, vec![exemplar_key(2).unwrap(), exemplar_key(2).unwrap()]);
    assert_eq!(
        &win.effects[1..3],
        &[
            WindowEffect::Lifecycle(Effect::ShutdownCustomView(42)),
            WindowEffect::Lifecycle(Effect::ReleaseCustomView(42))
        ]
    );
    assert!(win.effects.contains(&WindowEffect::HideControl(0x8a90_9e00)));
    let count = win.effects.len();
    assert_eq!(
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims),
        Ok(OwnerSelection::CurrentView)
    );
    assert_eq!(win.effects.len(), count);
    assert_eq!(view.update_subscription(), None);
    // Reset proves that the failed button remembered B, and still refreshes B's legend.
    win.visible = true;
    res.keys.clear();
    view.reset_with(&mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(res.keys, vec![exemplar_key(2).unwrap(), exemplar_key(2).unwrap()]);
}

// §10.7 and §7's common tail: false custom calls do not prevent completion.
#[test]
fn qv12_custom_initialization_factory_and_child_false_are_local() {
    for factory_false in [false, true] {
        let (mut view, mut res, mut win, mut render, mut sims) = inputs(1);
        set_property(res.raw.as_mut().unwrap(), 0x4a0b_47eb, RawValue::U32(99));
        let log = Arc::new(Mutex::new(Vec::new()));
        win.trace = Some(log.clone());
        render.trace = Some(log.clone());
        sims.trace = Some(log.clone());
        win.factory_false = factory_false;
        win.primitive_false = true;
        render.primitive_false = true;
        win.denied_controls.push(0x4203);
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(view.current_view(), Some(1));
        assert_eq!(view.update_subscription(), Some(subscription(1)));
        assert!(win.queries.contains(&(0x4203, 0xca31_8385)));
        assert!(!win.effects.iter().any(|e| matches!(e, WindowEffect::ChildFlag { .. })));
        let expected = if factory_false {
            vec!["highlights", "bitmap", "hook"]
        } else {
            vec!["init-custom", "highlights", "bitmap", "update-custom", "hook"]
        };
        assert_eq!(*log.lock().unwrap(), expected);
        assert_eq!(render.custom_calls, if factory_false { vec![] } else { vec![42] });
        view.notify(subscription(1).message, None).unwrap();
        assert_eq!(
            view.tick_with(1000, &mut render, &mut sims).unwrap().update,
            Some(Ok(OwnerUpdate::Drawn))
        );
        assert_eq!(view.last_redraw_ms(), 1000);
        assert!(!view.dirty());
        res.raw = None;
        win.effects.clear();
        view.select_with(2, &mut res, &mut win, &mut render, &mut sims)
            .unwrap_err();
        assert_eq!(
            win.effects.contains(&WindowEffect::Lifecycle(Effect::ReleaseCustomView(42))),
            !factory_false
        );
    }
}

// §10.8: the Init latch is set before the missing-message-server false path.
#[test]
fn qv12_init_false_primitives_and_missing_server_latch() {
    let mut view = MapView::new(0, 0);
    let mut win = Windows {
        primitive_false: true,
        ..Windows::default()
    };
    assert_eq!(view.init_with(&mut win), Ok(()));
    assert_eq!(win.effects.len(), 8);
    let mut view = MapView::new(0, 0);
    win.effects.clear();
    win.missing_server = true;
    assert_eq!(view.init_with(&mut win), Err(OwnerError::MissingMessageServer));
    assert_eq!(win.effects, vec![WindowEffect::Lifecycle(Effect::WindowGeneratorInit)]);
    win.missing_server = false;
    assert_eq!(
        view.init_with(&mut win),
        Err(OwnerError::Unsupported(Unsupported::RepeatedInit))
    );
}

// §10.9: scalar/default title, fresh lookup, hide-before-failure, local QI skip.
#[test]
fn qv12_legend_scalar_default_lookup_failure_and_title_query_skip() {
    let (view, mut res, mut win, _, _) = inputs(1);
    set_property(res.raw.as_mut().unwrap(), 0x4a0b_47f4, RawValue::U32(0x1234_5678));
    view.display_legend(7, &mut res, &mut win).unwrap();
    assert_eq!(res.keys, vec![exemplar_key(7).unwrap()]);
    assert_eq!(&win.effects[..19], legend_hide_prefix().as_slice());
    assert_eq!(
        win.effects[19],
        WindowEffect::LegendTitle {
            group: 0x6a23_1eaa,
            string: 0x1234_5678
        }
    );
    res.raw.as_mut().unwrap().0.retain(|p| p.id != 0x4a0b_47f4);
    win.effects.clear();
    view.display_legend(7, &mut res, &mut win).unwrap();
    assert!(win.effects.contains(&WindowEffect::LegendTitle {
        group: 0x6a23_1eaa,
        string: 0x2a5e_854f
    }));
    win.denied_controls.push(0x8a90_9d6a);
    win.effects.clear();
    view.display_legend(7, &mut res, &mut win).unwrap();
    assert!(!win
        .effects
        .iter()
        .any(|e| matches!(e, WindowEffect::LegendTitle { .. })));
    assert!(win
        .effects
        .iter()
        .any(|e| matches!(e, WindowEffect::LegendLabel { .. })));
    res.raw = None;
    win.effects.clear();
    view.display_legend(7, &mut res, &mut win).unwrap();
    assert_eq!(win.effects, legend_hide_prefix());
    win.denied_controls.clear();
    win.effects.clear();
    view.display_legend(0, &mut res, &mut win).unwrap();
    assert_eq!(win.effects.last(), Some(&WindowEffect::HidePanel(0x8a28_71c4)));
}

// §10.10: preserve indices, do not zip, cap to nine or invent unsafe-read text.
#[test]
fn qv12_legend_unequal_rows_zero_colors_and_control_skips() {
    let (view, mut res, mut win, _, _) = inputs(1);
    for (colors, strings, unsafe_index, rows) in [
        (vec![0xff11_2233, 0], vec![10], None, vec![0]),
        (vec![0xff11_2233], vec![10, 20], None, vec![0]),
        (vec![0, 0xff11_2233], vec![10], Some(1), vec![1]),
        (vec![0, 0, 0xff11_2233], vec![10], None, vec![2]),
        (vec![0xff11_2233; 10], vec![10; 10], None, (0..10).collect()),
    ] {
        set_property(res.raw.as_mut().unwrap(), 0x4a0b_47f2, RawValue::U32List(colors));
        set_property(res.raw.as_mut().unwrap(), 0x4a0b_47f3, RawValue::U32List(strings));
        win.effects.clear();
        let result = view.display_legend(1, &mut res, &mut win);
        assert_eq!(
            result,
            unsafe_index.map_or(Ok(()), |i| Err(OwnerError::Unsupported(Unsupported::LegendStringRead(
                i
            ))))
        );
        let controls: Vec<_> = win
            .effects
            .iter()
            .filter_map(|e| match e {
                WindowEffect::LegendSwatch { control, rgb } => {
                    assert_eq!(*rgb, 0x112233);
                    Some(*control - 0x8a90_9e10)
                }
                _ => None,
            })
            .collect();
        assert_eq!(controls, rows);
        if unsafe_index.is_some() || rows == [2] {
            assert!(!win
                .effects
                .iter()
                .any(|e| matches!(e, WindowEffect::LegendLabel { .. })));
        }
    }
    win.denied_controls = vec![0x8a90_9e10, 0x8a90_9e01];
    win.effects.clear();
    view.display_legend(1, &mut res, &mut win).unwrap();
    assert!(!win.effects.iter().any(|e| matches!(
        e,
        WindowEffect::LegendSwatch {
            control: 0x8a90_9e10,
            ..
        } | WindowEffect::LegendLabel {
            control: 0x8a90_9e01,
            ..
        }
    )));
    assert!(win.effects.contains(&WindowEffect::LegendLabel {
        control: 0x8a90_9e00,
        group: 0x6a23_1eaa,
        string: 10
    }));
}

fn hotspot_inputs(name: &str) -> (MapView, Resource, Windows, Render, Sims) {
    let (view, mut res, win, render, mut sims) = inputs(1);
    set_property(res.raw.as_mut().unwrap(), HIGHLIGHT, RawValue::U32(5));
    set_property(res.raw.as_mut().unwrap(), 0x4a0b_47ea, RawValue::String(name.into()));
    sims.hotspots.push_back(Some(CrimeHotspot {
        strength: 7,
        x: 2,
        z: 3,
    }));
    (view, res, win, render, sims)
}

// §10.11: direct signed cells, no second crime-grid factor, strength ignored.
#[test]
fn qv13_hotspot_position_and_strength_independence() {
    let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs("");
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    let record = HighlightRecord {
        occupant: None,
        position: [40.0, 12.0, 56.0],
        radius: 100.0,
        effect: None,
    };
    assert_eq!(view.retained_highlights(), std::slice::from_ref(&record));
    sims.hotspots.push_back(Some(CrimeHotspot {
        strength: 255,
        x: 2,
        z: 3,
    }));
    view.poll_highlights(&mut render, &mut sims).unwrap();
    assert_eq!(view.retained_highlights(), &[record]);
    sims.hotspots.push_back(Some(CrimeHotspot {
        strength: 1,
        x: -2,
        z: -3,
    }));
    view.poll_highlights(&mut render, &mut sims).unwrap();
    assert_eq!(view.retained_highlights()[0].position, [-24.0, 12.0, -40.0]);
    assert_eq!(sims.altitude_calls, vec![[40.0, 56.0], [40.0, 56.0], [-24.0, -40.0]]);
    assert!(sims.footprints.is_empty() && render.visuals.is_empty());
}

fn effect_sequence(temporary: u64, retained: u64) -> Vec<VisualLog> {
    let mut events = Vec::new();
    for (handle, scale) in [(temporary, 100.0), (retained, 200.0)] {
        events.push(VisualLog::Create("N".into()));
        events.push(VisualLog::Call(VisualEffect::Transform {
            handle,
            position: [40.0, 12.0, 56.0],
            scale,
            identity_rotation: true,
            flags: [2, 3],
        }));
        events.push(VisualLog::Call(VisualEffect::Start {
            handle,
            argument: false,
        }));
    }
    events.push(VisualLog::Call(VisualEffect::Release(temporary)));
    events
}

// §10.12: two creation stages; temporary released without an owner Stop.
#[test]
fn qv13_hotspot_two_stage_effect_creation() {
    let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs("N");
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(render.visuals, effect_sequence(1, 2));
    assert_eq!(view.retained_highlights()[0].effect, Some(2));
    assert!(view.dirty());
    assert_eq!(render.bitmaps.len(), 1);
}

// §10.13: repeated coordinates still replace; disappear/zero stop and release.
#[test]
fn qv13_hotspot_repeated_removal_empty_name_and_kind_zero() {
    for name in ["N", ""] {
        let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs(name);
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        render.visuals.clear();
        sims.hotspots.push_back(Some(CrimeHotspot {
            strength: 7,
            x: 2,
            z: 3,
        }));
        let tick = view.tick_with(1000, &mut render, &mut sims).unwrap();
        assert!(tick.due);
        assert!(!view.dirty());
        if !name.is_empty() {
            let mut expected = effect_sequence(3, 4);
            expected.insert(
                3,
                VisualLog::Call(VisualEffect::Stop {
                    handle: 2,
                    argument: false,
                }),
            );
            expected.push(VisualLog::Call(VisualEffect::Release(2)));
            assert_eq!(render.visuals, expected);
        } else {
            assert!(render.visuals.is_empty());
        }
        render.visuals.clear();
        view.poll_highlights(&mut render, &mut sims).unwrap();
        assert!(view.dirty() && view.retained_highlights().is_empty());
        if !name.is_empty() {
            assert_eq!(
                render.visuals,
                vec![
                    VisualLog::Call(VisualEffect::Stop {
                        handle: 4,
                        argument: false
                    }),
                    VisualLog::Call(VisualEffect::Release(4))
                ]
            );
        }
        render.visuals.clear();
        view.poll_highlights(&mut render, &mut sims).unwrap();
        assert!(render.visuals.is_empty());
    }
    let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs("N");
    view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
    set_property(res.raw.as_mut().unwrap(), HIGHLIGHT, RawValue::U32(0));
    render.visuals.clear();
    view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
    assert!(view.retained_highlights().is_empty());
    assert_eq!(
        render.visuals,
        vec![
            VisualLog::Call(VisualEffect::Stop {
                handle: 2,
                argument: false
            }),
            VisualLog::Call(VisualEffect::Release(2))
        ]
    );
}

// §10.14: either successful creation can supply the retained effect.
#[test]
fn qv13_hotspot_creation_failures_do_not_abort_selection_or_bitmap() {
    for (first, second, retained) in [(false, true, Some(2)), (true, false, Some(1)), (false, false, None)] {
        let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs("N");
        render.creations = VecDeque::from([first, second]);
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(view.current_view(), Some(1));
        assert_eq!(view.retained_highlights()[0].effect, retained);
        assert_eq!(view.retained_highlights()[0].radius, 100.0);
        assert_eq!(render.bitmaps.len(), 1);
        assert!(view.dirty());
    }
}

// §10.15: independent bitmap colours; no occupant footprint for a hotspot.
#[test]
fn qv13_independent_footprints_rings_and_effect_transforms() {
    let (_, _, _, mut render, mut sims) = inputs(1);
    sims.cells = [16, 16];
    let building = HighlightRecord {
        occupant: Some(17),
        position: [40.0, 12.0, 56.0],
        radius: 100.0,
        effect: None,
    };
    let hotspot = HighlightRecord {
        occupant: None,
        ..building.clone()
    };
    assert_eq!(
        footprint_rect([2, 3, 3, 4], [32, 32], [16, 16]),
        Ok(ClipRect {
            left: 4,
            top: 6,
            right: 9,
            bottom: 11
        })
    );
    let mut pixels = vec![0xff00_0000; 32 * 32];
    let style = HighlightStyle {
        name: "N".into(),
        ring: 0,
        footprint: 0x80ff_0000,
    };
    decorate_highlights(
        &[building.clone(), hotspot.clone()],
        &style,
        32,
        32,
        &mut pixels,
        &mut render,
        &mut sims,
    )
    .unwrap();
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(
                pixels[y * 32 + x],
                if (4..9).contains(&x) && (6..11).contains(&y) {
                    0xff80_0000
                } else {
                    0xff00_0000
                }
            );
        }
    }
    assert_eq!(sims.footprints, vec![17]);
    assert_eq!(render.buffer_calls, vec![true, false]);
    assert_eq!(ring_arguments(32, 256.0, &hotspot), Ok([5.0, 7.0, 12.5]));
    for (ef, ee) in [(0x80ff_0000, 0), (0, 0x00ff_0000)] {
        let (mut view, mut res, mut win, mut render, mut sims) = hotspot_inputs("N");
        set_property(res.raw.as_mut().unwrap(), 0x4a0b_47ef, RawValue::U32(ef));
        set_property(res.raw.as_mut().unwrap(), 0x4a0b_47ee, RawValue::U32(ee));
        sims.grid.width = 32;
        sims.grid.height = 24;
        sims.grid.values = GridValues::I8(vec![-128; 32 * 24]);
        view.select_with(1, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(render.visuals, effect_sequence(1, 2));
        assert!(sims.footprints.is_empty());
        if ee != 0 {
            assert!(render.bitmaps[0].iter().any(|&pixel| pixel != 0xff00_0000));
        }
        // A new selection clears the name and both colors when absent.
        res.raw = Some(raw(1));
        view.select_with(2, &mut res, &mut win, &mut render, &mut sims).unwrap();
        assert_eq!(view.highlight_style(), &HighlightStyle::default());
    }
}

// §10.16 and buffer failure/clip policy; EE alpha never supplies the weight.
#[test]
fn qv13_ring_blend_clip_and_independent_acquisitions() {
    let mut pixels = vec![0xff00_0000; 10 * 10];
    let clip = ClipRect {
        left: 0,
        top: 0,
        right: 10,
        bottom: 10,
    };
    paint_ring(&mut pixels, 10, 10, clip, [4.5, 4.5, 2.0], 0x0000_00ff).unwrap();
    assert_eq!(pixels[5 * 10 + 7], 0xff00_00fe);
    assert_eq!(pixels[5 * 10 + 5], 0xff00_0000);
    let before = pixels.clone();
    paint_ring(
        &mut pixels,
        10,
        10,
        ClipRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        [4.5, 4.5, 2.0],
        0x00ff_0000,
    )
    .unwrap();
    assert_eq!(pixels, before);
    let (_, _, _, mut render, mut sims) = inputs(1);
    let records = [
        HighlightRecord {
            occupant: Some(1),
            position: [115.2, 0.0, 115.2],
            radius: 51.2,
            effect: None,
        },
        HighlightRecord {
            occupant: None,
            position: [115.2, 0.0, 115.2],
            radius: 51.2,
            effect: None,
        },
    ];
    render.acquisitions = VecDeque::from([false, false, true]);
    pixels.fill(0xff00_0000);
    decorate_highlights(
        &records,
        &HighlightStyle {
            name: String::new(),
            footprint: 0x80ff_0000,
            ring: 0x0000_00ff,
        },
        10,
        10,
        &mut pixels,
        &mut render,
        &mut sims,
    )
    .unwrap();
    assert_eq!(render.buffer_calls, vec![true, true, true, false]);
    assert!(sims.footprints.is_empty());
    assert_eq!(pixels[5 * 10 + 7], 0xff00_00fe);
}

#[test]
fn qv13_common_building_effect_and_reference_primitives() {
    let mut render = Render::default();
    let mut building = HighlightRecord {
        occupant: Some(17),
        position: [1.0, 2.0, 3.0],
        radius: 0.5,
        effect: None,
    };
    building.add("B", &mut render).unwrap();
    assert_eq!(render.occupants, vec![(17, true, 0)]);
    assert_eq!(
        render.visuals[1],
        VisualLog::Call(VisualEffect::Transform {
            handle: 1,
            position: [1.0, 2.0, 3.0],
            scale: 1.0,
            identity_rotation: true,
            flags: [2, 1]
        })
    );
    building.remove(&mut render).unwrap();
    assert_eq!(render.occupants, vec![(17, true, 0), (17, false, 0)]);
    assert_eq!(render.released_occupants, vec![17]);
    assert_eq!(
        &render.visuals[3..],
        &[
            VisualLog::Call(VisualEffect::Stop {
                handle: 1,
                argument: false
            }),
            VisualLog::Call(VisualEffect::Release(1))
        ]
    );
}
