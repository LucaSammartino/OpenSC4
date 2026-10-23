// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

// LM4 §8.1 analytical ledgers, not captured native executions. Expected record
// words are public literals; side coordinates and request cells are independent.
#[test]
fn settled_wall_literal_four_side_ledger() {
    let rows: &[(f32, f32, &[(bool, u32)], [f32; 2])] = &[
        (12., 14., &[(false,0x411ccccd),(true,0x411ccccd),(false,0x41433333),(false,0x41433333),(true,0x411ccccd),(true,0x41633333)], [10.,14.]),
        (10., 12., &[(false,0x411ccccd),(true,0x411ccccd),(false,0x41233333),(false,0x41233333),(true,0x411ccccd),(true,0x41433333)], [10.,12.]),
        (12., 10., &[(false,0x411ccccd),(true,0x411ccccd),(false,0x41433333),(false,0x41433333),(true,0x411ccccd),(true,0x41233333)], [10.,12.]),
        (12., 8., &[(false,0x411ccccd),(true,0x40f9999a),(false,0x41433333)], [8.,12.]),
        (8., 12., &[(false,0x40f9999a),(true,0x411ccccd),(true,0x41433333)], [8.,12.]),
        (10.,10., &[], [10.,10.]),
        (8.,10., &[], [8.,9.]),
        (10.,8., &[], [10.,11.]),
        (12.,f32::from_bits(0x411ccccd), &[(false,0x411ccccd),(true,0x4119999a),(false,0x41433333)], [f32::from_bits(0x411ccccd),12.]),
        (f32::from_bits(0x411ccccd),12., &[(false,0x4119999a),(true,0x411ccccd),(true,0x41433333)], [f32::from_bits(0x411ccccd),12.]),
        (8.,9., &[], [8.,9.]),
    ];
    let endpoints = [([32.,80.],[48.,80.]),([48.,80.],[48.,96.]),([32.,96.],[48.,96.]),([32.,80.],[32.,96.])];
    for (side, (a,b)) in Edge::ALL.into_iter().zip(endpoints) {
        for &(h0,h1,expected,ys) in rows {
            let samples = [WallSample{position:a,terrain_height:h0},WallSample{position:b,terrain_height:h1}];
            let shape = WallShape::from_samples(side,10.,&samples,0.0625).unwrap();
            assert_eq!(shape.records.len(),expected.len(),"{side:?} {h0},{h1}");
            for (v,&(next,y)) in shape.records.iter().zip(expected) {
                let p = if next { b } else { a };
                assert_eq!(v.position,[p[0],f32::from_bits(y),p[1]]);
                assert_eq!(v.uv[1].to_bits(),y-0x02000000);
                assert_eq!(v.colour,[0;4]);
            }
            if !expected.is_empty() {
                assert_eq!([shape.bounds.min[1],shape.bounds.max[1]],ys);
            }
        }
    }
}
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

struct Binding {
    id: u32,
    alpha: bool,
    drops: Arc<Mutex<Vec<u32>>>,
}
impl Texture for Binding {
    fn id(&self) -> u32 {
        self.id
    }
    fn has_alpha(&self) -> bool {
        self.alpha
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        self.drops.lock().unwrap().push(self.id);
    }
}
#[derive(Default)]
struct Factory {
    calls: Vec<TextureKey>,
    replies: VecDeque<Option<(u32, bool)>>,
    drops: Arc<Mutex<Vec<u32>>>,
}
impl Factory {
    fn replying(replies: &[Option<(u32, bool)>]) -> Self {
        Self {
            replies: replies.iter().copied().collect(),
            ..Self::default()
        }
    }
}
impl TextureFactory for Factory {
    type Binding = Binding;
    fn load(&mut self, key: TextureKey) -> Option<Binding> {
        self.calls.push(key);
        self.replies.pop_front().flatten().map(|(id, alpha)| Binding {
            id,
            alpha,
            drops: self.drops.clone(),
        })
    }
}
#[derive(Default)]
struct Light {
    normals: Vec<Vec3>,
    corners: Vec<(Vec3, Vec3)>,
}
impl Lighting for Light {
    fn for_normal(&mut self, normal: Vec3) -> Vec3 {
        self.normals.push(normal);
        normal
    }
    fn at_corner(&mut self, position: Vec3, normal: Vec3) -> Vec3 {
        self.corners.push((position, normal));
        position
    }
}
fn zoom(value: u8) -> Zoom {
    Zoom::new(value).unwrap()
}
fn foundation(base: f32) -> FoundationModel<Binding> {
    FoundationModel::new(
        Footprint {
            min: [0.0, 0.0],
            max: [32.0, 48.0],
        },
        base,
        40.0,
        8.0,
        FoundationTextureKeys {
            sides: [None; 5],
            tops: [None; 5],
        },
    )
    .unwrap()
}
fn border(ground: f32, water: Option<f32>) -> [BorderHeight; 1] {
    [BorderHeight { ground, water }]
}
fn samples(a: f32, b: f32) -> [WallSample; 2] {
    [
        WallSample {
            position: [0.0, 0.0],
            terrain_height: a,
        },
        WallSample {
            position: [16.0, 0.0],
            terrain_height: b,
        },
    ]
}
fn wall(side: Edge) -> RetainingWallModel<Binding> {
    let points = if matches!(side, Edge::East | Edge::West) {
        [
            WallSample {
                position: [0.0, 0.0],
                terrain_height: 0.0,
            },
            WallSample {
                position: [0.0, 16.0],
                terrain_height: 0.0,
            },
        ]
    } else {
        samples(0.0, 0.0)
    };
    RetainingWallModel::init(side, 10.0, &points, 0.5, [None; 5], true).unwrap()
}
fn layout(step: CornerStep) -> CellCornerLayout {
    // An explicitly chosen test cycle, never a claim about native strip order.
    let corners = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    CellCornerLayout::new(corners, corners, step).unwrap()
}
fn record() -> CellRecord {
    CellRecord {
        texture_key: 0x100,
        cell: [2, -1],
        orientation: Orientation::new(0).unwrap(),
        mirror: false,
        rgb: [255, 128, 64],
        alpha: 255,
        mode: 0,
    }
}
struct Source {
    lot: Option<LotSample>,
    heights: [f32; 4],
    calls: Vec<&'static str>,
}
impl CellPlacementSource for Source {
    fn lot_at(&mut self, cell: [i32; 2]) -> Option<LotSample> {
        assert_eq!(cell, [2, -1]);
        self.calls.push("lot");
        self.lot
    }
    fn terrain_corners(&mut self, cell: [i32; 2]) -> [f32; 4] {
        assert_eq!(cell, [2, -1]);
        self.calls.push("terrain");
        self.heights
    }
}
fn source(flat: bool) -> Source {
    Source {
        lot: Some(LotSample {
            flags: if flat { 0x101 } else { 0x100 },
            altitude: 20.0,
        }),
        heights: [1.0, 3.0, -2.0, 4.0],
        calls: Vec::new(),
    }
}
fn context() -> CellDrawContext {
    CellDrawContext {
        highlighted: false,
        zoom: zoom(2),
        translucent_pass: false,
        terrain_mode: 0,
        global_light: [128.0, 255.0, 0.0],
        no_add_signed: false,
    }
}
fn cell(rec: CellRecord, flat: bool) -> CellTextureModel<Binding> {
    let mut model = CellTextureModel::new(rec, layout(CornerStep::Forward));
    model.place(&mut source(flat), &mut Light::default()).unwrap();
    model
}

#[test]
fn foundation_public_step_vector_and_water_precedence() {
    let mut model = foundation(40.0);
    let result = model
        .adjust_height_to_terrain(&border(21.0, None), false, &mut Light::default())
        .unwrap();
    assert_eq!(
        result,
        HeightAdjustment {
            base: 8.0,
            base_changed: true,
            rebuilt: true
        }
    );
    let mut wet = foundation(40.0);
    wet.adjust_height_to_terrain(&border(21.0, Some(25.0)), false, &mut Light::default())
        .unwrap();
    assert_eq!(wet.base(), 24.0);
    // Low water cannot replace higher ground; the largest drop wins, not first/last.
    wet.adjust_height_to_terrain(
        &[
            BorderHeight {
                ground: 30.0,
                water: Some(0.0),
            },
            BorderHeight {
                ground: 0.0,
                water: Some(10.0),
            },
        ],
        false,
        &mut Light::default(),
    )
    .unwrap();
    assert_eq!(wet.base(), 8.0);
}

#[test]
fn foundation_exact_step_and_adjacent_f32_boundary() {
    for (ground, expected) in [
        (24.0, 24.0),
        (f32::from_bits(24.0_f32.to_bits() - 1), 8.0),
        (f32::from_bits(24.0_f32.to_bits() + 1), 24.0),
    ] {
        let mut model = foundation(40.0);
        model
            .adjust_height_to_terrain(&border(ground, None), false, &mut Light::default())
            .unwrap();
        assert_eq!(model.base(), expected, "ground={ground:?}");
    }
}

#[test]
fn foundation_never_raises_base_and_force_rebuild_does_not_relight() {
    let mut model = foundation(8.0);
    let mut light = Light::default();
    model.setup_vertices(false, true, &mut light);
    let shape = model.shape().clone();
    let first = model
        .adjust_height_to_terrain(&border(100.0, None), false, &mut light)
        .unwrap();
    assert_eq!(
        first,
        HeightAdjustment {
            base: 8.0,
            base_changed: false,
            rebuilt: false
        }
    );
    let forced = model
        .adjust_height_to_terrain(&border(100.0, None), true, &mut light)
        .unwrap();
    assert_eq!(
        forced,
        HeightAdjustment {
            base: 8.0,
            base_changed: false,
            rebuilt: true
        }
    );
    assert_eq!(model.shape(), &shape);
    assert_eq!(light.normals.len(), 5);
}

#[test]
fn foundation_fractional_existing_depth_extends_only_downward() {
    let mut model = foundation(25.0);
    model
        .adjust_height_to_terrain(&border(24.0, None), false, &mut Light::default())
        .unwrap();
    assert_eq!(model.base(), 24.0);
    let mut deeper = foundation(20.0);
    deeper
        .adjust_height_to_terrain(&border(24.0, None), false, &mut Light::default())
        .unwrap();
    assert_eq!(deeper.base(), 20.0);
}

#[test]
fn foundation_faces_spans_and_lighting_are_labelled_not_native_ordered() {
    let mut model = foundation(8.0);
    let mut light = Light::default();
    model.setup_vertices(true, true, &mut light);
    let shape = model.shape();
    assert_eq!(
        shape.sides[0].bounds,
        Bounds {
            min: [0.0, 8.0, 0.0],
            max: [32.0, 40.0, 0.0]
        }
    );
    assert_eq!(
        shape.sides.map(|v| v.repeats),
        [[2.0, 2.0], [3.0, 2.0], [2.0, 2.0], [3.0, 2.0]]
    );
    assert_eq!(shape.top.repeats, [4.0, 6.0]);
    assert_eq!(
        light.normals,
        [
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0]
        ]
    );
    model.setup_vertices(false, false, &mut light);
    assert_eq!(light.normals.len(), 5);
}

#[test]
fn foundation_draw_order_fallback_and_independent_zoom_rotation_caches() {
    let mut model = foundation(8.0);
    let mut factory = Factory::replying(&[
        Some((0x1000_0000, false)),
        Some((2, false)),
        Some((3, false)),
        Some((4, false)),
    ]);
    let mut light = Light::default();
    let draws = model.draw(false, zoom(3), Edge::East, &mut factory, &mut light);
    assert_eq!(
        draws.iter().map(|d| d.part).collect::<Vec<_>>(),
        [
            FoundationPart::Side(Edge::South),
            FoundationPart::Side(Edge::West),
            FoundationPart::Top
        ]
    );
    assert!(draws
        .iter()
        .all(|d| d.primitive_type == 6 && d.state == DrawState::OPAQUE));
    assert_eq!(
        factory.calls,
        [
            TextureKey::FoundationSide(0x69F2_0003),
            TextureKey::FoundationTop(0x0912_2212)
        ]
    );
    assert_eq!(model.sort_value(), 0x1000_2AB6);
    model.draw(false, zoom(3), Edge::East, &mut factory, &mut light);
    assert_eq!((factory.calls.len(), light.normals.len()), (2, 5));
    model.draw(false, zoom(3), Edge::South, &mut factory, &mut light);
    assert_eq!((factory.calls.len(), light.normals.len()), (2, 10));
    model.draw(false, zoom(4), Edge::South, &mut factory, &mut light);
    assert_eq!((factory.calls.len(), light.normals.len()), (4, 10));
}

#[test]
fn highlighted_foundation_leaves_all_state_untouched() {
    let mut model = foundation(8.0);
    let mut factory = Factory::default();
    let mut light = Light::default();
    assert!(model
        .draw(true, zoom(3), Edge::West, &mut factory, &mut light)
        .is_empty());
    assert_eq!((model.zoom(), model.rotation()), (None, None));
    assert!(factory.calls.is_empty() && light.normals.is_empty());
    assert_eq!(model.sort_value(), 0xE986_2AB6);
}

#[test]
fn foundation_failures_are_atomic_and_top_only_draw_is_not_assumed() {
    let mut model = foundation(8.0);
    let before = model.shape().clone();
    for data in [
        vec![],
        border(f32::NAN, None).to_vec(),
        border(2.0, Some(f32::INFINITY)).to_vec(),
    ] {
        assert_eq!(
            model.adjust_height_to_terrain(&data, true, &mut Light::default()),
            Err(ContractError::InvalidInput)
        );
        assert_eq!(model.shape(), &before);
    }
    let mut factory = Factory::replying(&[None, Some((9, false))]);
    assert!(model
        .draw(false, zoom(0), Edge::North, &mut factory, &mut Light::default())
        .is_empty());
    assert_eq!(model.sort_value(), 0xE986_2AB6);
    assert_eq!(model.top_texture().unwrap().id(), 9);
    assert_eq!(model.zoom(), Some(zoom(0)));
}

#[test]
fn wall_public_facing_vector_and_all_rotations() {
    let expected = [
        [true, false, false, true],
        [true, true, false, false],
        [false, true, true, false],
        [false, false, true, true],
    ];
    for (r, rotation) in Edge::ALL.into_iter().enumerate() {
        for (s, side) in Edge::ALL.into_iter().enumerate() {
            assert_eq!(side.wall_faces(rotation), expected[r][s]);
        }
    }
}

#[test]
fn wall_settled_triangles_differ_from_old_semantic_bands() {
    let level = WallShape::from_samples(Edge::North, 10.0, &samples(10.0, 10.0), 0.5).unwrap();
    assert!(level.bands.is_empty());
    let shape = WallShape::from_samples(Edge::South, 10.0, &samples(8.0, 12.0), 0.5).unwrap();
    assert_eq!(shape.bands.len(), 1);
    assert_eq!(shape.bands[0].expanded_heights, [[7.8, 10.2], [9.8, 12.2]]);
    assert_eq!(shape.outward_normal, [0.0, 0.0, 1.0]);
    assert_eq!(shape.outward_offset_range, [0.001, 0.002]);
    // DIFFERS: old fixture expected an LM4 gap. Settled §3.2 has a literal
    // ascending triangle, without an interpolated intersection.
    assert_eq!(shape.triangles().unwrap(), [[0.,7.8,0.],[16.,9.8,0.],[16.,12.2,0.]]);
    assert_eq!(
        WallShape::from_samples(Edge::North, 10.0, &samples(10.0, 9.0), 1.0)
            .unwrap()
            .bands
            .len(),
        0
    );
}

#[test]
fn wall_fallback_zoom_four_highlight_and_cull_order() {
    let mut model = wall(Edge::South);
    let mut factory = Factory::replying(&[Some((0x8000_0000, false))]);
    assert!(model.draw(true, zoom(4), Edge::East, &mut factory).is_none());
    assert!(factory.calls.is_empty());
    assert!(model.draw(false, zoom(4), Edge::East, &mut factory).is_none());
    assert_eq!(
        factory.calls,
        [TextureKey::Wall {
            group: 0x891B_0E1A,
            instance: 0x695C_00CA
        }]
    );
    assert_eq!(model.sort_value(), 0x8000_2A86);
    assert!(model.recolour_pending());
    model.set_facing_test(false);
    assert!(
        model
            .draw(false, zoom(4), Edge::East, &mut factory)
            .unwrap()
            .recolour_required
    );
    assert_eq!(factory.calls.len(), 1);
}

#[test]
fn wall_rebuild_failure_does_not_replace_shape() {
    let mut model = wall(Edge::North);
    let original = model.shape().clone();
    assert_eq!(
        model.setup_vertices(10.0, &samples(0.0, f32::NAN), 1.0),
        Err(ContractError::InvalidInput)
    );
    assert_eq!(model.shape(), &original);
    assert_eq!(
        WallShape::from_samples(Edge::East, 10.0, &samples(0.0, 0.0), 1.0),
        Err(ContractError::InvalidInput)
    );
    assert_eq!(model.sort_value(), 0xC986_2A86);
    assert!(!across_edge_lot_is_flat(0xFFFF_FFFE));
    assert!(across_edge_lot_is_flat(0x8000_0001));
}

#[test]
fn cell_orientation_public_vector_with_explicit_cycle() {
    let forward = layout(CornerStep::Forward);
    let one = Orientation::new(1).unwrap();
    assert_eq!(
        forward.texture_corners(one, false),
        [[1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]
    );
    assert_eq!(
        forward.texture_corners(one, true),
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
    );
    assert_eq!(
        layout(CornerStep::Backward).texture_corners(one, false),
        [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
    );
    for i in 0..4 {
        let orientation = Orientation::new(i).unwrap();
        let a = forward.texture_corners(orientation, false);
        let b = forward.texture_corners(orientation, true);
        for j in 0..4 {
            assert_eq!([1.0 - a[j][0], a[j][1]], b[j]);
        }
    }
}

#[test]
fn cell_flat_placement_queries_lot_then_lighting_without_terrain() {
    let mut model = CellTextureModel::<Binding>::new(record(), layout(CornerStep::Forward));
    let mut source = source(true);
    let mut light = Light::default();
    model.place(&mut source, &mut light).unwrap();
    assert_eq!(source.calls, ["lot"]);
    let placement = model.placement().unwrap();
    assert_eq!(
        placement.positions,
        [
            [32.0, 20.0, -16.0],
            [48.0, 20.0, -16.0],
            [48.0, 20.0, 0.0],
            [32.0, 20.0, 0.0]
        ]
    );
    assert_eq!(
        placement.bounds,
        Bounds {
            min: [32.1, 18.0, -15.9],
            max: [47.9, 22.0, -0.1]
        }
    );
    assert_eq!(placement.colours, Some(placement.positions));
    assert_eq!(light.corners.len(), 4);
    assert!(light.corners.iter().all(|(_, normal)| *normal == [0.0, 1.0, 0.0]));
}

#[test]
fn cell_slope_and_absent_lot_use_terrain_without_lighting() {
    for lot in [source(false).lot, None] {
        let mut source = source(false);
        source.lot = lot;
        let mut light = Light::default();
        let mut model = CellTextureModel::<Binding>::new(record(), layout(CornerStep::Forward));
        model.place(&mut source, &mut light).unwrap();
        assert_eq!(source.calls, ["lot", "terrain"]);
        assert!(light.corners.is_empty());
        let p = model.placement().unwrap();
        assert_eq!(p.positions.map(|v| v[1]), [1.0, 3.0, -2.0, 4.0]);
        assert_eq!(p.bounds.min[1], -4.0);
        assert_eq!(p.bounds.max[1], 6.0);
        assert!(!p.flat && p.colours.is_none());
    }
}

#[test]
fn failed_cell_placement_preserves_previous_geometry_and_colours() {
    let mut model = cell(record(), true);
    let before = model.placement().cloned();
    let mut source = source(false);
    source.heights[2] = f32::NAN;
    let mut light = Light::default();
    assert_eq!(model.place(&mut source, &mut light), Err(ContractError::InvalidInput));
    assert_eq!(model.placement(), before.as_ref());
    assert!(light.corners.is_empty());
}

#[test]
fn cell_mode_public_alpha_vector_and_precedence_boundaries() {
    assert_eq!(cell_mode(0, 128, false), CellMode::Alpha);
    assert_eq!(cell_mode(0, 255, false), CellMode::Opaque);
    for alpha in [0, 1, 254] {
        assert_eq!(cell_mode(0, alpha, false), CellMode::Alpha);
    }
    assert_eq!(cell_mode(0, 255, true), CellMode::Alpha);
    for mode in [1, 2, 3, 255] {
        assert_eq!(cell_mode(mode, 255, false), CellMode::Tinted);
    }
}

#[test]
fn cell_lookup_falls_back_to_unzoomed_key_and_keeps_texture_alpha() {
    let mut model = cell(record(), false);
    let mut factory = Factory::replying(&[None, Some((15, true))]);
    model.load_texture(zoom(2), &mut factory).unwrap();
    assert_eq!(
        factory.calls,
        [
            TextureKey::Cell {
                type_id: 0x7AB5_0E44,
                group: 0x0986_135E,
                instance: 0x102
            },
            TextureKey::Cell {
                type_id: 0x7AB5_0E44,
                group: 0x0986_135E,
                instance: 0x100
            }
        ]
    );
    assert!(model.texture().unwrap().has_alpha());
    assert!(model.draw(context(), &mut factory).unwrap().is_none());
    assert_eq!(factory.calls.len(), 2);
}

#[test]
fn cell_zoom_reload_precedes_translucent_pass_gate_but_highlight_skips_it() {
    let mut rec = record();
    rec.alpha = 128;
    let mut model = cell(rec, true);
    let mut factory = Factory::replying(&[Some((10, false))]);
    let mut ctx = context();
    ctx.highlighted = true;
    assert!(model.draw(ctx, &mut factory).unwrap().is_none());
    assert_eq!(model.zoom(), None);
    assert!(factory.calls.is_empty());
    ctx.highlighted = false;
    ctx.terrain_mode = 8;
    assert!(model.draw(ctx, &mut factory).unwrap().is_none());
    assert_eq!(factory.calls.len(), 1);
    ctx.translucent_pass = true;
    let draw = model.draw(ctx, &mut factory).unwrap().unwrap();
    assert_eq!(draw.mode, CellMode::Alpha);
    assert_eq!(draw.state, DrawState::ALPHA);
    assert!(draw.state.depth_write);
    assert!(matches!(draw.geometry, CellGeometryDraw::FlatStrip { .. }));
    assert!(draw.clamp && draw.combiner.is_none() && draw.filter.is_none());
}

#[test]
fn cell_terrain_force_tests_exactly_eight_and_keeps_record_alpha() {
    let mut model = cell(record(), false);
    let mut factory = Factory::replying(&[Some((10, false))]);
    for mode in [0, 7, 8, 9, 0x18] {
        let mut ctx = context();
        ctx.terrain_mode = mode;
        let draw = model.draw(ctx, &mut factory).unwrap().unwrap();
        match draw.geometry {
            CellGeometryDraw::TerrainQuad {
                cell, alpha, forced, ..
            } => {
                assert_eq!(cell, [2, -1]);
                assert_eq!(alpha, 255);
                assert_eq!(forced, mode == 8);
            }
            _ => panic!("slope used a flat strip"),
        }
    }
}

#[test]
fn cell_tinted_no_add_signed_halves_alpha_only_for_mode_three() {
    for (mode, no_add, expected_mode, divisor) in [
        (3, true, 1, 2.0),
        (3, false, 3, 1.0),
        (1, true, 1, 1.0),
        (2, true, 2, 1.0),
    ] {
        let mut rec = record();
        rec.mode = mode;
        rec.alpha = 128;
        let request = tinted_combiner(&rec, [128.0, 255.0, 0.0], no_add);
        assert_eq!(request.mode, expected_mode);
        assert_eq!(request.environment.rgb, [128.0, 128.0, 0.0]);
        assert_eq!(request.environment.alpha, (128.0 / 255.0) / divisor);
    }
    let mut rec = record();
    rec.mode = 3;
    let mut model = cell(rec, true);
    let mut factory = Factory::replying(&[Some((1, false))]);
    let mut ctx = context();
    ctx.translucent_pass = true;
    ctx.no_add_signed = true;
    let draw = model.draw(ctx, &mut factory).unwrap().unwrap();
    assert_eq!(draw.filter, Some(TextureFilter::Linear));
    assert_eq!(draw.state, DrawState::TINTED);
    assert!(!draw.state.depth_write);
    assert_eq!(draw.combiner.unwrap().environment.alpha, 0.5);
}

#[test]
fn draw_states_keep_depth_and_alpha_contracts_distinct() {
    assert_eq!(DrawState::OPAQUE.depth_test, DepthTest::LessOrEqual);
    assert_eq!(DrawState::OPAQUE.alpha_test, AlphaTest::Disabled);
    assert_eq!(DrawState::ALPHA.alpha_test, AlphaTest::GreaterThanZero);
    assert_eq!(DrawState::ALPHA.blend, Blend::SourceAlphaOneMinusSourceAlpha);
    assert_eq!(DrawState::TINTED.depth_test, DepthTest::Less);
    for state in [DrawState::OPAQUE, DrawState::ALPHA, DrawState::TINTED] {
        assert!(state.colour_write && !state.cull);
        assert_eq!(state.texture_count, 1);
    }
}

#[test]
fn owned_leases_survive_draw_borrows_and_drop_once_on_reload_and_owner_drop() {
    let mut factory = Factory::replying(&[Some((1, false)), Some((2, false)), None, Some((3, false))]);
    let drops = factory.drops.clone();
    {
        let mut model = foundation(8.0);
        {
            let draws = model.draw(false, zoom(0), Edge::North, &mut factory, &mut Light::default());
            assert_eq!(draws[0].texture.id(), 1);
            assert!(drops.lock().unwrap().is_empty());
        }
        model.load_textures(zoom(1), &mut factory);
        assert_eq!(*drops.lock().unwrap(), [1, 2]);
        assert!(model.side_texture().is_none());
        assert_eq!(model.top_texture().unwrap().id(), 3);
    }
    assert_eq!(*drops.lock().unwrap(), [1, 2, 3]);
}

#[test]
fn failed_primary_does_not_create_or_release_a_lease() {
    let mut model = cell(record(), true);
    let mut factory = Factory::replying(&[None, Some((20, false)), None, None]);
    model.load_texture(zoom(2), &mut factory).unwrap();
    assert!(factory.drops.lock().unwrap().is_empty());
    model.load_texture(zoom(3), &mut factory).unwrap();
    assert_eq!(*factory.drops.lock().unwrap(), [20]);
    assert!(model.texture().is_none());
    drop(model);
    assert_eq!(*factory.drops.lock().unwrap(), [20]);
}

#[test]
fn overflow_is_explicit_and_does_not_call_factory_or_change_cached_zoom() {
    let mut rec = record();
    rec.texture_key = u32::MAX;
    let mut model = cell(rec, true);
    let mut factory = Factory::replying(&[Some((9, false))]);
    model.load_texture(zoom(0), &mut factory).unwrap();
    assert_eq!(
        model.load_texture(zoom(1), &mut factory),
        Err(ContractError::TextureKeyOverflow)
    );
    assert_eq!(model.zoom(), Some(zoom(0)));
    assert_eq!(model.texture().unwrap().id(), 9);
    assert_eq!(factory.calls.len(), 1);
}

#[test]
fn portable_domains_do_not_silently_wrap_or_default() {
    assert_eq!(Zoom::new(5), Err(ContractError::InvalidInput));
    assert_eq!(Orientation::new(4), Err(ContractError::InvalidInput));
    assert_eq!(
        CellCornerLayout::new([[0.0, 0.0]; 4], [[0.0, 0.0]; 4], CornerStep::Forward),
        Err(ContractError::InvalidInput)
    );
    assert_eq!(
        FoundationModel::<Binding>::new(
            Footprint {
                min: [0.0; 2],
                max: [1.0; 2]
            },
            0.0,
            1.0,
            0.0,
            FoundationTextureKeys {
                sides: [None; 5],
                tops: [None; 5]
            }
        )
        .err(),
        Some(ContractError::InvalidInput)
    );
}

#[test]
fn owners_and_immutable_getters_support_thread_safe_bindings() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FoundationModel<Binding>>();
    send_sync::<RetainingWallModel<Binding>>();
    send_sync::<CellTextureModel<Binding>>();
}

#[test]
fn configured_foundation_primary_failure_falls_back_before_top_load() {
    let mut side_keys = [None; 5];
    side_keys[3] = Some(0x1234);
    let mut top_keys = [None; 5];
    top_keys[3] = Some(0x5678);
    let mut model = FoundationModel::new(
        Footprint {
            min: [0.0, 0.0],
            max: [16.0, 16.0],
        },
        8.0,
        40.0,
        16.0,
        FoundationTextureKeys {
            sides: side_keys,
            tops: top_keys,
        },
    )
    .unwrap();
    let mut factory = Factory::replying(&[None, Some((1, false)), Some((2, false))]);
    model.load_textures(zoom(3), &mut factory);
    assert_eq!(
        factory.calls,
        [
            TextureKey::FoundationSide(0x1234),
            TextureKey::FoundationSide(0x69F2_0003),
            TextureKey::FoundationTop(0x5678)
        ]
    );
    assert_eq!(model.side_texture().unwrap().id(), 1);
    assert_eq!(model.top_texture().unwrap().id(), 2);
}

#[test]
fn wall_lease_replaced_and_destroyed_exactly_once() {
    let mut factory = Factory::replying(&[Some((30, false)), Some((31, false))]);
    let drops = factory.drops.clone();
    {
        let mut model = wall(Edge::North);
        model.load_texture(zoom(0), &mut factory);
        assert!(drops.lock().unwrap().is_empty());
        model.load_texture(zoom(1), &mut factory);
        assert_eq!(*drops.lock().unwrap(), [30]);
    }
    assert_eq!(*drops.lock().unwrap(), [30, 31]);
}

#[test]
fn nonfinite_derived_foundation_dimensions_are_outside_portable_domain() {
    let keys = || FoundationTextureKeys {
        sides: [None; 5],
        tops: [None; 5],
    };
    assert_eq!(
        FoundationModel::<Binding>::new(
            Footprint {
                min: [-f32::MAX, 0.0],
                max: [f32::MAX, 16.0]
            },
            0.0,
            1.0,
            16.0,
            keys()
        )
        .err(),
        Some(ContractError::InvalidInput)
    );
    assert_eq!(
        FoundationModel::<Binding>::new(
            Footprint {
                min: [0.0, 0.0],
                max: [16.0, 16.0]
            },
            0.0,
            1.0,
            f32::from_bits(1),
            keys()
        )
        .err(),
        Some(ContractError::InvalidInput)
    );
}
