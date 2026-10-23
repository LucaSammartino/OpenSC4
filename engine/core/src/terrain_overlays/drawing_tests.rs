// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

const IDENTITY: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

struct Drawing {
    commands: Vec<DrawCommand>,
    rectangles: Vec<(RectangleRoute, CellRect)>,
    binds: Vec<(u8, bool, bool)>,
    rows: VisibleRows,
    matrix: [f32; 16],
    resets: usize,
    acquires: usize,
    releases: usize,
    extension: bool,
    flag7: bool,
    gate: bool,
    water: i32,
    water_calls: usize,
}

impl Default for Drawing {
    fn default() -> Self {
        Self {
            commands: vec![],
            rectangles: vec![],
            binds: vec![],
            rows: VisibleRows {
                begin: 0,
                end: 5,
                starts: vec![0; 5],
                ends: vec![9; 5],
            },
            matrix: IDENTITY,
            resets: 0,
            acquires: 0,
            releases: 0,
            extension: true,
            flag7: false,
            gate: true,
            water: 0,
            water_calls: 0,
        }
    }
}

impl DrawingProducer for Drawing {
    fn reset_model(&mut self) {
        self.resets += 1;
    }
    fn model_matrix(&self) -> [f32; 16] {
        self.matrix
    }
    fn depth_offset(&self) -> u32 {
        91
    }
    fn acquire_vertex_extension(&mut self) -> bool {
        self.acquires += 1;
        self.extension
    }
    fn release_vertex_extension(&mut self) {
        self.releases += 1;
    }
    fn draw_rows(&mut self) -> VisibleRows {
        self.rows.clone()
    }
    fn command(&mut self, c: DrawCommand) {
        self.commands.push(c);
    }
    fn bind_texture(&mut self, stage: u8, b: Option<&Arc<dyn TextureBinding>>, s: Option<&Arc<dyn TextureSource>>) {
        self.binds.push((stage, b.is_some(), s.is_some()));
    }
    fn draw_rectangle(&mut self, route: RectangleRoute, vertices: CellRect) {
        self.rectangles.push((route, vertices));
    }
    fn water_slot53(&mut self, cells: &mut CellRect) -> i32 {
        self.water_calls += 1;
        *cells = rect([-100; 2], [100; 2]);
        self.water
    }
    fn device_flag7(&self) -> bool {
        self.flag7
    }
    fn location_strength_enabled(&self) -> bool {
        self.gate
    }
}

struct Lighting {
    strength: f32,
    s: f32,
    calls: Vec<[f32; 2]>,
    parameter_calls: usize,
}
impl Default for Lighting {
    fn default() -> Self {
        Self {
            strength: 1.0,
            s: 0.25,
            calls: vec![],
            parameter_calls: 0,
        }
    }
}
impl ShadowLighting for Lighting {
    fn parameters_slot14(&mut self) -> ShadowParameters {
        self.parameter_calls += 1;
        ShadowParameters {
            q: 0.5,
            s: self.s,
            rgb: [0.2, 0.3, 0.4],
        }
    }
    fn strength_slot15(&mut self, xz: [f32; 2]) -> f32 {
        self.calls.push(xz);
        self.strength
    }
}

#[test]
fn manager_stats_count_submissions_and_preserve_skipped_passes() {
    let mut m = manager(false);
    for _ in 0..3 {
        m.add_city_decal(4).unwrap();
    }
    for _ in 0..2 {
        m.add_shadow_group(5, [32., 32.], 16., 0., 0).unwrap();
    }
    let (mut d, mut l) = (Drawing::default(), Lighting::default());
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.stats_string(), "3 decals, 2 shadows");
    assert_eq!(d.rectangles.len(), 5);
    assert_eq!((d.resets, d.acquires, d.releases), (1, 1, 1));
    assert_eq!(
        &d.commands[d.commands.len() - 2..],
        &[DrawCommand::ClearTransform(0), DrawCommand::DepthOffset(91)]
    );
    d.rows.end = 0;
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.stats_string(), "3 decals, 2 shadows");
    d.rows.end = 5;
    m.draw_frame(
        FrameOptions {
            decals: false,
            shadows: false,
            ..FRAME
        },
        &mut d,
        &mut l,
    )
    .unwrap();
    assert_eq!(m.stats_string(), "3 decals, 2 shadows");
    for id in 0..5 {
        m.set_shown(Handle(id), false).unwrap();
    }
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.stats_string(), "0 decals, 0 shadows");
}

#[test]
fn full_shadow_normal_debug_gate_and_midpoint() {
    let mut m = manager(false);
    m.shadow_debug = 5;
    let h = m.add_shadow_group(5, [32., 48.], 32., 0., 0).unwrap();
    let id = m.handle_indices(h).unwrap()[0];
    m.overlays.get_mut(id).unwrap().cells = rect([1, 2], [3, 4]);
    m.selection_byte = 0;
    let (mut d, mut l) = (
        Drawing::default(),
        Lighting {
            strength: 1.0 / 16.0,
            ..Lighting::default()
        },
    );
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(l.calls, [[32., 48.]]);
    assert!(d.rectangles.is_empty());
    assert_eq!(m.shadows_drawn, 0);
    assert!(d
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Transform { stage: 1, .. })));
    l.strength = 0.25;
    d.commands.clear();
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.shadows_drawn, 1);
    assert!(d.commands.contains(&DrawCommand::AlphaTest(Some(1.0 / 32.0))));
    assert_eq!(
        d.rectangles.last(),
        Some(&(RectangleRoute::LandStaticExtension, rect([1, 2], [4, 5])))
    );
    m.shadow_debug = 4;
    l.calls.clear();
    d.binds.clear();
    d.commands.clear();
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert!(l.calls.is_empty());
    assert_eq!(d.binds, [(0, true, false)]); // shared binding only, no per-record primary
    assert!(!d
        .commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Transform { stage: 1, .. })));
    assert!(d.commands.contains(&DrawCommand::ShadowCombiner {
        stage: 0,
        descriptor: 0x50
    }));
    m.shadow_debug = 0;
    d.gate = false;
    l.strength = 0.;
    l.calls.clear();
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.shadows_drawn, 1);
    assert!(l.calls.is_empty());
}

#[test]
fn water_full_routes_use_original_rectangle_and_signed_comparison() {
    for dynamic in [false, true] {
        let mut m = manager(dynamic);
        m.water = true;
        m.shadow_debug = 0;
        m.selection_byte = 0;
        let h = m.add_shadow_group(5, [32., 48.], 32., 0., 0).unwrap();
        let id = m.handle_indices(h).unwrap()[0];
        m.overlays.get_mut(id).unwrap().cells = rect([1, 2], [3, 4]);
        let (mut d, mut l) = (Drawing::default(), Lighting::default());
        for r in [8, 9] {
            d.water = r;
            m.draw_frame(FRAME, &mut d, &mut l).unwrap();
            let route = match (r, dynamic) {
                (8, false) => RectangleRoute::ShadowLandStatic,
                (8, true) => RectangleRoute::ShadowLandDynamic,
                (_, false) => RectangleRoute::WaterStatic,
                (_, true) => RectangleRoute::WaterDynamic,
            };
            assert_eq!(d.rectangles.last(), Some(&(route, rect([1, 2], [4, 5]))));
            assert_eq!(m.snapshot(h).unwrap().unwrap().cells, rect([1, 2], [3, 4]));
            assert!(l.calls.is_empty());
        }
    }
}

#[test]
fn rough_pass_alpha_minimum_flag7_and_snapshot_shadow_mask() {
    let mut m = manager(false);
    m.shadow_quality = 2;
    m.add_shadow_group(5, [32., 32.], 16., 0., 0).unwrap();
    let (mut d, mut l) = (Drawing::default(), Lighting::default());
    for s in [0., f32::NAN, 0.25] {
        l.s = s;
        d.commands.clear();
        m.draw_frame(
            FrameOptions {
                snapshot: true,
                ..FRAME
            },
            &mut d,
            &mut l,
        )
        .unwrap();
        assert_eq!(m.shadows_drawn, 1); // no snapshot flag required for shadows
        assert!(d
            .commands
            .contains(&DrawCommand::AlphaTest(if s > 0. { Some(0.125) } else { None })));
        assert!(d.commands.contains(&DrawCommand::DeviceFlag7(true)));
        assert!(d.commands.contains(&DrawCommand::DeviceFlag7(false)));
    }
    d.flag7 = true;
    d.commands.clear();
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert!(!d.commands.contains(&DrawCommand::DeviceFlag7(false)));
    assert_eq!(d.water_calls, 0);
    assert!(l.calls.is_empty());
    m.dynamic = true;
    d.commands.clear();
    let previous = (d.resets, d.acquires, d.releases, m.stats_string());
    m.draw_frame(
        FrameOptions {
            snapshot: true,
            ..FRAME
        },
        &mut d,
        &mut l,
    )
    .unwrap();
    assert_eq!(previous, (d.resets, d.acquires, d.releases, m.stats_string()));
    assert!(d.commands.is_empty());
}

#[test]
fn camera_order_reset_and_singular_propagation() {
    let mut m = manager(true);
    m.add_decal_binding(Some(Arc::new(Texture)), [0.; 2], 16., 0.).unwrap();
    let (mut d, mut l) = (Drawing::default(), Lighting::default());
    d.matrix[12] = -16.; // inverse is translation +16
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    let transform = d
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::Transform { stage: 0, matrix } => Some(matrix),
            _ => None,
        })
        .unwrap();
    assert_eq!(transform[12], 1.5);
    assert_eq!(
        m.draw_frame_exact(FRAME),
        Err(Error::Gap(ContractGap::WorldTextureTransformOv7))
    );
    d.matrix = [0.; 16];
    d.commands.clear();
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert!(d.commands.iter().any(|c| match c {
        DrawCommand::Transform { matrix, .. } => matrix.iter().any(|x| x.is_nan()),
        _ => false,
    }));
    assert_eq!(m.decals_drawn, 1);
}

#[test]
fn keyed_flip_byte_and_placement_unchanged() {
    let mut m = manager(false);
    let mut placements = Vec::new();
    for flip in [0x100, 0x101] {
        let h = m.add_shadow_group(77, [0.; 2], 16., 0., flip).unwrap();
        let mut p = Placement([0.; 9]);
        m.get_placement(h, &mut p).unwrap();
        placements.push(p);
        let o = m.snapshot(h).unwrap().unwrap();
        // Texture is 64x32, so its upper edge is world z=4.
        let primary_v = 4. * o.primary[9] + o.primary[13];
        let s = o.secondary.unwrap();
        let secondary_v = 4. * s[9] + s[13];
        assert_eq!(primary_v, if flip == 0x100 { 1. } else { 0. });
        assert_eq!(secondary_v, if flip == 0x100 { 0.75 } else { 0.25 });
        assert_eq!(o.flags, 9);
    }
    assert_eq!(placements[0], placements[1]);
}

#[test]
fn decal_source_shade_wrap_dispatch_and_dual_masks() {
    for dynamic in [false, true] {
        for extension in [false, true] {
            let mut m = manager(dynamic);
            let h = m.add_decal_source(Arc::new(Texture), [32., 32.], 16., 0.).unwrap();
            m.enable_self_lighting(h, 1).unwrap();
            m.set_texture_scale(h, 16., 0.).unwrap();
            let (mut d, mut l) = (
                Drawing {
                    extension,
                    ..Drawing::default()
                },
                Lighting::default(),
            );
            m.draw_frame(FRAME, &mut d, &mut l).unwrap();
            assert!(d.binds.contains(&(0, false, true)));
            assert!(d.commands.contains(&DrawCommand::Wrap {
                stage: 0,
                modes: [3, 2]
            }));
            assert!(d.commands.contains(&DrawCommand::Shade {
                rgb: [1.; 3],
                self_lit: true
            }));
            assert!(d.commands.contains(&DrawCommand::VertexColors(false, false)));
            assert!(d.commands.contains(&DrawCommand::ResetTransparency));
            assert!(d.commands.contains(&DrawCommand::ResetShade));
            let expected = match (dynamic, extension) {
                (false, false) => RectangleRoute::LandStatic,
                (false, true) => RectangleRoute::LandStaticExtension,
                (true, false) => RectangleRoute::LandDynamic,
                (true, true) => RectangleRoute::LandDynamicExtension,
            };
            assert_eq!(d.rectangles[0].0, expected);
            assert_eq!(d.releases, usize::from(extension));
        }
    }
    let mut m = manager(true);
    let h = m
        .add_overlay_for_rect(Some(Arc::new(Texture)), rect([2, 3], [2, 3]), IDENTITY, 0x18)
        .unwrap();
    m.overlays.get_mut(h.index()).unwrap().secondary = Some(IDENTITY);
    m.shadow_binding = None;
    let (mut d, mut l) = (Drawing::default(), Lighting::default());
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.stats_string(), "1 decals, 1 shadows");
    assert_eq!(d.rectangles.len(), 2); // no area/triangle test at submission/count
    assert_eq!(d.binds.len(), 1); // debug has no per-record bind and no dummy shared binding
}

#[test]
fn disabled_index_scans_records_and_empty_selection_still_cleans_context() {
    let mut m = manager(false);
    m.add_city_decal(3).unwrap();
    let (mut d, mut l) = (
        Drawing {
            rows: VisibleRows::default(),
            ..Drawing::default()
        },
        Lighting::default(),
    );
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert!(d.rectangles.is_empty());
    assert_eq!((d.resets, d.acquires, d.releases), (1, 1, 1));
    assert_eq!(
        d.commands,
        [DrawCommand::ClearTransform(0), DrawCommand::DepthOffset(91)]
    );
    m.handle_settings_message(CHANGE_MESSAGE, [3, 4], 0);
    m.draw_frame(FRAME, &mut d, &mut l).unwrap();
    assert_eq!(m.decals_drawn, 1);
}

#[test]
fn redisplay_callback_observes_order_and_empty_dynamic_suppression() {
    type Events = Mutex<Vec<(i32, bool, CellRect, i32)>>;
    struct Ground(Events);
    impl TerrainProvider for Ground {
        fn cells(&self) -> [u32; 2] {
            [8, 4]
        }
        fn cell_width(&self) -> f32 {
            16.
        }
        fn redisplay(&self, pass: i32, water: bool, cells: CellRect, flags: i32) {
            self.0.lock().unwrap().push((pass, water, cells, flags));
        }
    }
    struct Parent(Arc<Ground>, Arc<View>);
    impl ViewProvider for Parent {
        fn terrain(&self) -> Option<Arc<dyn TerrainProvider>> {
            Some(self.0.clone())
        }
        fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>> {
            self.1.texture_factory()
        }
        fn shadow_quality(&self) -> i32 {
            3
        }
        fn shadow_debug(&self) -> i32 {
            4
        }
        fn subscribe(&self, _: u32) {}
        fn unsubscribe(&self, _: u32) {}
    }
    let ground = Arc::new(Ground(Mutex::new(vec![])));
    let mut m = OverlayManager::new();
    m.init(Arc::new(Parent(ground.clone(), view(3))), false, true).unwrap();
    let h = m
        .add_decal_binding(Some(Arc::new(Texture)), [32., 32.], 16., 0.)
        .unwrap();
    let old = m.snapshot(h).unwrap().unwrap().cells;
    ground.0.lock().unwrap().clear();
    m.set_position(h, [80., 32.]).unwrap();
    let new = m.snapshot(h).unwrap().unwrap().cells;
    assert_eq!(*ground.0.lock().unwrap(), [(0, true, old, 0), (0, true, new, 0)]);
    ground.0.lock().unwrap().clear();
    m.set_shown(h, true).unwrap(); // identical value still marks
    assert_eq!(*ground.0.lock().unwrap(), [(0, true, new, 0)]);
    ground.0.lock().unwrap().clear();
    m.dynamic = true;
    m.set_color(h, [1.; 3]).unwrap();
    assert!(ground.0.lock().unwrap().is_empty());
    m.dynamic = false;
    m.add_overlay_for_rect(Some(Arc::new(Texture)), rect([2, 3], [2, 3]), IDENTITY, 0)
        .unwrap();
    assert!(ground.0.lock().unwrap().is_empty());
}

#[test]
fn rotated_coefficients_expand_footprint_only_without_repeats() {
    let mut m = manager(false);
    let h = m
        .add_decal_binding(Some(Arc::new(Texture)), [64., 32.], 32., 0.125)
        .unwrap();
    let o = m.snapshot(h).unwrap().unwrap();
    assert!(o.primary[2] > 0.7 && o.primary[10] > 0.7);
    assert!(o.cells.min[1] < 1 && o.cells.max[1] > 3);
    let p = Placement([64., 32., 32., 0.125, 1., 2., 1., 0., f32::NAN]);
    m.set_placement(h, p).unwrap();
    let repeated = m.snapshot(h).unwrap().unwrap();
    assert_eq!(repeated.cells, rect([3, 1], [5, 3]));
    assert_ne!(repeated.flags & flag::REPEAT, 0);
    let mut out = Placement([0.; 9]);
    m.get_placement(h, &mut out).unwrap();
    assert_eq!(out.0.map(f32::to_bits), p.0.map(f32::to_bits));
    m.set_texture_scale(h, 0., 0.).unwrap();
    let matrix = m.snapshot(h).unwrap().unwrap().primary;
    assert_eq!(matrix[0], f32::INFINITY);
    assert_eq!(matrix[5], f32::NEG_INFINITY);
    assert!(matrix[13].is_nan());
}
