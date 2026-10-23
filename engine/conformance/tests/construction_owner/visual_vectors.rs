// Included by construction_owner.rs to share its recording host.

fn visual_geometry() -> ConstructionGeometry {
    ConstructionGeometry {
        ground: 1.,
        start_y: 2.,
        top_y: 9.,
        f: 2.,
        segments: 2,
        scaffold: ([0., 1., 0.], [10., 9., 8.]),
    }
}

fn visual_owner() -> (ConstructionOccupant, ConstructionStatics, Host) {
    let mut c = ConstructionOccupant::default();
    let mut st = statics();
    st.tuning.post_width = 1.;
    st.tuning.plank_width = 2.;
    st.tuning.plank_texture_scale = Some(0.25);
    st.tuning.scaffold_texture_scale = Some(0.5);
    st.tuning.girder_texture_scale = Some(2.);
    st.tuning.crane_height_above_scaffold = 3.;
    let g = visual_geometry();
    c.geometry = Some(g);
    c.rec.local_bounds = g.scaffold;
    c.rec.world_bounds = ([-1., 1., -1.], [11., 9., 9.]);
    c.meshes.rebuild(g, &st.tuning, 2.);
    c.model_dirty = true;
    (c, st, Host::default())
}

#[test]
fn ld36_phase_endpoints_divisor_zero_and_wrapping() {
    let mut s = ConstructionScalars {
        start_y: 12.,
        segments: 3,
        span: 12.,
        counter: 2,
        a: 1,
        b: 1,
        c: 1,
        e: 2,
        ..ConstructionScalars::default()
    };
    for (state, expected) in [(0, [10., 11., 12.]), (1, [16., 18., 20.]), (3, [24., 17., 10.])] {
        s.state = state;
        for (ms, value) in [0, 250, 500].into_iter().zip(expected) {
            s.ms_acc = ms;
            assert_eq!(phase_value(s, 10., 4.), Some((false, value)));
        }
    }
    s.state = 2;
    for (ms, value) in [(250, 5_f32 / 12.), (500, 0.5), (1000, 2_f32 / 3.)] {
        s.ms_acc = ms;
        assert_eq!(phase_value(s, 10., 4.), Some((true, value)));
    }
    s.state = 0;
    s.a = 0;
    s.ms_acc = 0;
    assert_eq!(phase_value(s, 10., 4.), Some((false, 12.)));
    s.state = 1;
    s.b = 0;
    assert!(phase_value(s, 10., 4.).unwrap().1.is_nan());
    s.ms_acc = 1;
    assert_eq!(phase_value(s, 10., 4.), Some((false, 20.)));
    s.state = 3;
    s.start_y = 0.1;
    s.span = 8.;
    s.ms_acc = 0;
    assert_eq!(phase_value(s, 0., 4.), Some((false, 8.1)));
    let (mut c, mut st, mut h) = visual_owner();
    c.rec.scalars.state = 4;
    c.rec.scalars.ms_acc = u32::MAX;
    c.scaffold_height = 123.;
    c.animation_tick(&mut h, &mut st, 2);
    assert_eq!(c.rec.scalars.ms_acc, 1);
    assert_eq!(c.scaffold_height, 123.);
}

#[test]
fn ld35_scaffold_topology_height_and_roof_propagation() {
    let (mut c, st, mut h) = visual_owner();
    let sides = c.meshes.sides.map(Option::unwrap);
    assert_eq!(
        sides[0].map(|v| v.position),
        [[0., 1., 0.], [0., 9., 0.], [10., 9., 0.], [10., 1., 0.]]
    );
    assert_eq!(sides[1][0].position, [0., 1., 8.]);
    assert_eq!(sides[2][0].position, [10., 1., 8.]);
    assert_eq!(sides[3][0].position, [0., 1., 0.]);
    assert!(sides.iter().flatten().all(|v| v.uv == [0., 0.]));
    let planks = c.meshes.planks.unwrap();
    assert_eq!(
        planks[..4].iter().map(|v| v.position).collect::<Vec<_>>(),
        vec![[0., 9., 2.], [0., 9., 0.], [10., 9., 0.], [10., 9., 2.]]
    );
    assert_eq!(planks[4].position, [8., 9., 6.]);
    assert_eq!(planks[8].position, [0., 9., 8.]);
    assert_eq!(planks[12].position, [0., 9., 6.]);
    assert_eq!(planks[2].uv, [2.5, 0.]);
    assert_eq!(
        c.meshes.girder.unwrap().map(|v| v.position),
        [[2., 9., 6.], [2., 9., 2.], [8., 9., 2.], [8., 9., 6.]]
    );
    let posts = c.meshes.posts.unwrap();
    assert_eq!(posts.len(), 80);
    assert_eq!(posts[0].position, [-0.5, 1., -0.5]);
    assert_eq!(posts[16].position, [-0.5, 10., 0.5]);
    c.meshes.crane_post(4., 5., 1., 0.5);
    c.rec.has_crane = true;
    c.rec.crane_position = [4., 0., 5.];
    c.rec.roof_props = vec![reference(11)];
    c.rec.crew = vec![
        CrewEntry {
            first: true,
            second: false,
            automaton: reference(12),
        },
        CrewEntry {
            first: false,
            second: true,
            automaton: reference(13),
        },
    ];
    h.positions = vec![(11, [2., 0., 3.]), (12, [4., 1., 5.]), (13, [6., 1., 7.])];
    let bounds = c.bounding_box();
    c.set_scaffold_height(&mut h, &st.tuning, 20.);
    assert_eq!(c.bounding_box(), bounds);
    assert_eq!(c.rec.local_bounds, visual_geometry().scaffold);
    assert_eq!(c.rec.crane_position, [4., 23., 5.]);
    assert_eq!(h.moved_props, vec![(11, [2., 20., 3.])]);
    assert_eq!(h.moved_workers, vec![(13, [6., 20., 7.])]);
    for (old, new) in posts.iter().zip(c.meshes.posts.unwrap()) {
        assert_eq!(new.position[0], old.position[0]);
        assert_eq!(new.position[2], old.position[2]);
        assert_eq!(new.position[1], if old.position[1] == 1. { 1. } else { 21. });
    }
    assert!(c.meshes.planks.unwrap().iter().all(|v| v.position[1] == 20.));
    assert!(c.meshes.girder.unwrap().iter().all(|v| v.position[1] == 20.));
    let post = c.meshes.crane_post.unwrap();
    assert_eq!((post[0].position[1], post[1].position[1]), (1., 23.));
    // No clamp: even a below-ground height propagates, without changing bases.
    c.set_scaffold_height(&mut h, &st.tuning, -5.);
    assert_eq!(c.meshes.sides[0].unwrap()[1].position[1], -5.);
}

#[test]
fn ld35_inner_crane_positions_swing_fit_and_angle() {
    let size = CraneSize {
        width: 1.,
        back: 4.,
        reach: 12.,
    };
    let b = bounds(0., 0., 40., 20.);
    for (side, x, centre) in [(0, 16., std::f32::consts::TAU), (2, 24., std::f32::consts::PI)] {
        let (p, lo, hi) = inner_crane_pose(b, size, side);
        assert_eq!(p, [x, 0., 10.]);
        let a = (10_f64 / 12.).asin();
        assert_eq!(lo, (f64::from(centre) - a) as f32);
        assert_eq!(hi, (f64::from(centre) + a) as f32);
    }
    let b = bounds(0., 0., 30., 40.);
    for (side, z, lo, hi) in [
        (1, 24., 0., std::f32::consts::PI),
        (3, 16., std::f32::consts::PI, std::f32::consts::TAU),
    ] {
        assert_eq!(inner_crane_pose(b, size, side), ([15., 0., z], lo, hi));
    }
    let mut st = statics();
    let mut expected = st.rng.clone();
    expected.next_raw();
    assert!(inner_crane_choice(bounds(0., 0., 16., 10.), &[size; 3], &mut st.rng)
        .1
        .is_none());
    assert_eq!(st.rng.next_raw(), expected.next_raw());
    assert_eq!(crane_model_angle(std::f32::consts::PI), std::f32::consts::TAU);
    assert!(crane_model_angle(-4.) < 0.);
    assert!(crane_model_angle(25.) <= std::f32::consts::TAU);
}

#[test]
fn ld35_crane_lifetime_resource_failure_and_cached_offsets() {
    let (mut c, mut st, mut h) = visual_owner();
    st.tuning.crane_post_width = 2.;
    c.rec.scalars.swing_min = 2.;
    c.rec.scalars.swing_max = 4.;
    c.create_crane(&mut h, &mut st, 6., 7., 1);
    assert_eq!(c.rec.crane_position, [6., 0., 7.]);
    assert_eq!(
        h.crane_requests[0],
        CraneRequest {
            resource: [0x5AD0E817, 0xBADB57F1, 0x1C220000],
            instance: 0,
            position: [6., 0., 7.],
            relative_y_rotation: 5.
        }
    );
    c.set_scaffold_height(&mut h, &st.tuning, 20.);
    assert_eq!(c.rec.crane_position, [6., 23., 7.]);
    h.crane_fail = true;
    st.tuning.crane_post_width = 20.;
    c.create_crane(&mut h, &mut st, 10., 12., 0);
    assert!(!c.rec.has_crane);
    assert_eq!(c.rec.crane_position, [6., 23., 7.]);
    let post = c.meshes.crane_post.unwrap();
    assert_eq!(post[0].position, [9., 1., 11.]);
    assert_eq!(post[1].position, [9., 1.1, 11.]);
    assert!(post.iter().all(|v| v.bgra == [128, 128, 128, 255]));
    // Exercise the actual SetLot inner branch; fit selection remains in the owner.
    h.crane_fail = false;
    h.policy = 2;
    st.cranes = [CraneSize {
        width: 1.,
        back: 2.,
        reach: 10.,
    }; 3];
    c.set_lot(&mut h, &mut st, reference(1));
    assert!(c.rec.has_crane);
    assert_eq!(h.crane_requests.last().unwrap().resource[2], CRANE_MODELS[0]);
    assert_eq!(st.girder_plank, Some(2.));
    st.tuning.plank_width = 7.;
    c.set_lot(&mut h, &mut st, reference(1));
    assert_eq!(c.meshes.girder.unwrap()[1].position[0], c.rec.local_bounds.0[0] + 2.);
}

#[derive(Default)]
struct Renderer {
    rotation: usize,
    varied_lighting: bool,
    commands: Vec<RenderCommand>,
    textured: Vec<Vec<TextureVertex>>,
    coloured: Vec<Vec<ColourVertex>>,
    normals: Vec<[f32; 3]>,
}
impl ConstructionRenderer for Renderer {
    fn rotation(&self) -> usize {
        self.rotation
    }
    fn zoom_column(&self) -> usize {
        2
    }
    fn project_to_view(&mut self, p: [f32; 3]) -> [f32; 3] {
        [p[0] + 1., p[1] + 2., p[2] + 3.]
    }
    fn lighting_rgb(&mut self, n: [f32; 3]) -> [u8; 3] {
        self.normals.push(n);
        if self.varied_lighting {
            [20 * self.normals.len() as u8, 100, 50]
        } else {
            [201, 100, 51]
        }
    }
    fn command(&mut self, c: RenderCommand) {
        self.commands.push(c);
    }
    fn textured(&mut self, v: &[TextureVertex], primitive: u8, format: u8) {
        assert_eq!((primitive, format), (6, 2));
        self.textured.push(v.to_vec());
    }
    fn coloured(&mut self, v: &[ColourVertex], primitive: u8, format: u8) {
        assert_eq!((primitive, format), (6, 1));
        self.coloured.push(v.to_vec());
    }
}

#[test]
fn ld35_draw_order_transforms_relighting_and_removal() {
    let (mut c, st, _) = visual_owner();
    c.rec.has_crane = true;
    c.meshes.crane_post(2., 3., 1., 1.);
    let before = c.meshes.posts.unwrap();
    let mut r = Renderer {
        rotation: 1,
        ..Renderer::default()
    };
    assert!(c.draw(&mut r, &st.tuning, true));
    assert!(r.commands.is_empty() && r.normals.is_empty());
    assert!(c.draw(&mut r, &st.tuning, false));
    assert!(!c.model_dirty);
    assert_eq!(
        r.normals,
        vec![[0., 0., -1.], [0., 0., 1.], [1., 0., 0.], [-1., 0., 0.], [0., 1., 0.]]
    );
    for (old, new) in before.iter().zip(c.meshes.posts.unwrap()) {
        assert_eq!(old.position, new.position);
        assert_eq!(new.bgra, [25, 50, 100, 255]);
    }
    assert!(c
        .meshes
        .crane_post
        .unwrap()
        .iter()
        .all(|v| v.bgra == [25, 50, 100, 255]));
    let sides = [2, 0, 3, 1].map(|i| c.meshes.sides[i].unwrap().to_vec());
    assert_eq!(&r.textured[..4], &sides);
    let textures: Vec<_> = r
        .commands
        .iter()
        .filter_map(|c| {
            if let RenderCommand::Texture { row, column } = c {
                Some(TEXTURE_BASES[*row] + *column as u32)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        textures,
        vec![0x696F3002, 0x696F2002, 0x696F1002, 0x696F2002, 0x696F5002, 0x696F4002]
    );
    let blends: Vec<_> = r
        .commands
        .iter()
        .filter_map(|c| {
            if let RenderCommand::Blend(b) = c {
                Some(*b)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(blends, vec![false, false, true, false, true, false]);
    assert_eq!(r.coloured.iter().map(Vec::len).collect::<Vec<_>>(), vec![80, 16]);
    assert_eq!(
        &r.commands[r.commands.len() - 4..],
        &[
            RenderCommand::ResetShade,
            RenderCommand::DefaultState,
            RenderCommand::DepthOffset(0),
            RenderCommand::CraneModel(false)
        ]
    );
    let first = c.meshes.sides[2].unwrap()[0].position;
    let expected = [
        [0.5, 0., 0., 0.],
        [0., -0.5, 0., 0.],
        [0., 0., 0.5, 0.],
        [
            -0.5 * (first[0] + 1.),
            0.5 * (first[1] + 2.),
            -0.5 * (first[2] + 3.),
            1.,
        ],
    ];
    assert!(r.commands.contains(&RenderCommand::TextureTransform(Some(expected))));
    c.rec.scalars.state = 3;
    let mut r = Renderer::default();
    c.draw(&mut r, &st.tuning, false);
    assert_eq!(r.textured.len(), 6);
    assert_eq!(r.coloured.iter().map(Vec::len).collect::<Vec<_>>(), vec![80]);
    assert!(!r.commands.contains(&RenderCommand::CraneModel(false)));
    assert_eq!(r.commands.last(), Some(&RenderCommand::DepthOffset(0)));
    c.meshes = ConstructionMeshes::default();
    let mut r = Renderer::default();
    c.draw(&mut r, &st.tuning, false);
    assert!(r.coloured.is_empty() && r.textured.is_empty());
}

#[test]
fn ld37_prop_anchor_crew_flags_normalization_and_render_gate() {
    let b = ([10., -100., 20.], [40., 0., 50.]);
    assert_eq!(
        prop_anchor(b, [4., 2.], [3., 5.], Some([0., 7., 0.]), |_, _| panic!()).0,
        [15., 7., 26.]
    );
    let mut samples = [2., 8., 5., 3.].into_iter();
    assert_eq!(
        prop_anchor(b, [4., 2.], [3., 5.], None, |_, _| samples.next().unwrap()).0,
        [15., 8., 26.]
    );
    let n = normalized_direction(0.3, 0.4);
    assert!((n[0] - 0.6).abs() < 1e-7 && (n[1] - 0.8).abs() < 1e-7);
    assert!(normalized_direction(0., 0.).iter().all(|v| v.is_nan()));
    for detail in [-1, 2, 3] {
        let (mut c, mut st, mut h) = visual_owner();
        h.detail = detail;
        let mut expected_rng = st.rng.clone();
        expected_rng.range_i32(0, 1); // empty roof table still draws its count
        c.set_lot(&mut h, &mut st, reference(1));
        if detail < 3 {
            assert!(c.rec.crew.is_empty());
            assert!(h.directions.is_empty());
            assert_eq!(st.rng.next_raw(), expected_rng.next_raw());
        } else {
            assert!(c.rec.crew.iter().any(|e| !e.first && e.second));
            assert!(c.rec.crew.iter().any(|e| !e.first && !e.second));
            assert!(c.rec.crew.iter().all(|e| !e.first));
            assert!(h
                .positions
                .iter()
                .filter(|(id, _)| (100..200).contains(id))
                .all(|(_, p)| p[1] == 100.));
            assert!(h
                .directions
                .iter()
                .all(|d| (d[0] * d[0] + d[2] * d[2] - 1.).abs() < 2e-7));
            let roof = c.rec.crew.iter().filter(|e| e.second).count();
            assert_eq!(h.moved_workers.len(), roof);
        }
    }
}

#[test]
fn ld38_bindings_effect_parameters_and_unclipped_cuts() {
    for (extent, name) in [
        (9.99, "small_dust"),
        (10., "medium_dust"),
        (19.99, "medium_dust"),
        (20., "large_dust"),
    ] {
        assert_eq!(dust_name(bounds(0., 0., extent, 1.)), name);
    }
    let (mut c, mut st, mut h) = visual_owner();
    c.set_lot(&mut h, &mut st, reference(1));
    let bindings = h.bindings.unwrap();
    for (row, base) in TEXTURE_BASES.into_iter().enumerate() {
        for (j, binding) in bindings[row].iter().enumerate() {
            assert_eq!(
                *binding,
                TextureBinding {
                    class: 0x1AC0E11A,
                    interface: 0xFAC0E219,
                    resource: [0x7AB50E44, 0x1ABE787D, base + j as u32]
                }
            );
        }
    }
    assert_eq!(h.effects.len(), 1 + c.rec.effects.len());
    assert_eq!(
        h.effects[0],
        EffectRequest {
            name: "large_dust",
            position: [0., 100., 0.],
            slot: 7,
            enabled: true
        }
    );
    assert!(h.effects[1..].iter().all(|e| *e
        == EffectRequest {
            name: "welding_sparks",
            position: [0., 100., 0.],
            slot: 8,
            enabled: true
        }));
    let dust = token(&c.rec.current_effect);
    c.rec.scalars.counter = c.rec.scalars.segments;
    c.rec.scalars.state = 2;
    c.rec.scalars.next_day = 0;
    c.do_daily_sim(&mut h, &mut st);
    assert!(h.events.contains(&format!("effect8 {dust}")));
    assert_eq!(h.effects.len(), 1 + c.rec.effects.len());
    let mut b = ([0., -7., 0.], [10., 12., 10.]);
    remove_rectangle(&mut b, bounds(-5., 2., -4., 8.));
    assert_eq!(b, ([-4., -7., 0.], [10., 12., 10.]));
    let mut b = bounds(0., 0., 10., 10.);
    remove_rectangle(&mut b, bounds(4., 4., 6., 6.));
    assert_eq!(b, bounds(0., 0., 10., 4.));
}

#[test]
fn ld35_all_view_orders_face_colours_and_alpha() {
    let (mut c, st, _) = visual_owner();
    c.meshes.crane_post(2., 3., 1., 1.);
    c.meshes.posts.as_mut().unwrap()[0].bgra[3] = 77;
    let mut r = Renderer {
        varied_lighting: true,
        ..Renderer::default()
    };
    c.do_rotation_change(&mut r);
    assert_eq!(c.meshes.posts.unwrap()[0].bgra, [25, 50, 10, 77]);
    for face in 0..5 {
        assert_eq!(c.meshes.posts.unwrap()[face * 4].bgra[2], 10 * (face + 1) as u8);
    }
    for (face, red) in [10, 30, 20, 40].into_iter().enumerate() {
        assert_eq!(c.meshes.crane_post.unwrap()[face * 4].bgra, [25, 50, red, 255]);
    }
    for (rotation, order) in [[0, 3, 1, 2], [2, 0, 3, 1], [1, 2, 0, 3], [3, 1, 2, 0]]
        .into_iter()
        .enumerate()
    {
        let mut r = Renderer {
            rotation,
            ..Renderer::default()
        };
        c.draw(&mut r, &st.tuning, false);
        for (draw, side) in order.into_iter().enumerate() {
            assert_eq!(r.textured[draw], c.meshes.sides[side].unwrap());
        }
        let offsets: Vec<_> = r
            .commands
            .iter()
            .filter_map(|c| match c {
                RenderCommand::DepthOffset(n) => Some(*n),
                _ => None,
            })
            .collect();
        assert_eq!(offsets, vec![1, 1, -1, -1, -1, -1, 0]);
    }
}

#[test]
fn ld36_live_quantized_removal_and_ld37_unrounded_anchor() {
    let (mut c, mut st, mut h) = visual_owner();
    c.set_lot(&mut h, &mut st, reference(1));
    let g = c.geometry.unwrap();
    assert_eq!(
        c.rec.scalars.span,
        (f64::from(g.segments) * f64::from(st.tuning.segment_height)) as f32
    );
    c.rec.scalars.state = 3;
    c.rec.scalars.ms_acc = 0;
    c.animation_tick(&mut h, &mut st, 0);
    assert_eq!(
        c.scaffold_height,
        (f64::from(g.start_y) + f64::from(c.rec.scalars.span)) as f32
    );
    assert!(c.scaffold_height > g.top_y);
    // Rounding the RDRU result before adding the minimum/half-size loses this bit.
    let offset = 2_f64.powi(-24) + 2_f64.powi(-50);
    let p = prop_anchor(
        bounds(1., 0., 10., 10.),
        [1., 1.],
        [offset, 0.],
        Some([0.; 3]),
        |_, _| panic!(),
    )
    .0;
    assert_eq!(p[0].to_bits(), 1.5_f32.to_bits() + 1);
    let t = ConstructionTuning::default();
    assert!(t.plank_texture_scale.is_none());
    let mut meshes = ConstructionMeshes::default();
    meshes.rebuild(visual_geometry(), &t, t.plank_width);
    assert!(meshes.planks.unwrap()[0].uv[0].is_nan()); // LD39, no invented default
}
