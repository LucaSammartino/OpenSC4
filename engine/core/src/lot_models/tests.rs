// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct RawWall {
    heights: std::collections::VecDeque<f32>,
    flags: Option<u32>,
    lot: Option<f32>,
    calls: Vec<String>,
}
impl WallTerrain for RawWall {
    fn inner_lot_height(&mut self, c: [i32; 2]) -> Option<f32> {
        self.calls.push(format!("inner {c:?}"));
        self.lot
    }
    fn terrain_height(&mut self, c: [i32; 2]) -> f32 {
        self.calls.push(format!("height {c:?}"));
        self.heights.pop_front().unwrap()
    }
    fn extent_x(&mut self) -> i32 {
        self.calls.push("width".into());
        100
    }
    fn extent_z(&mut self) -> i32 {
        self.calls.push("depth".into());
        100
    }
    fn lot_flags(&mut self, c: [i32; 2], arg: u32) -> Option<u32> {
        assert_eq!(arg, 0);
        self.calls.push(format!("across {c:?}"));
        self.flags
    }
}
fn raw_wall(heights: &[f32]) -> RawWall {
    RawWall {
        heights: heights.iter().copied().collect(),
        flags: None,
        lot: Some(10.),
        calls: Vec::new(),
    }
}
fn wall_box() -> Bounds {
    Bounds {
        min: [32., 0., 80.],
        max: [48., 0., 96.],
    }
}
#[derive(Default)]
struct RawLight {
    calls: Vec<(Vec3, Vec3)>,
}
impl WallLighting for RawLight {
    fn components(&mut self, p: Vec3, n: Vec3) -> [i32; 3] {
        self.calls.push((p, n));
        [257, 514, 771]
    }
}
impl FoundationLighting for RawLight {
    fn components_for_normal(&mut self, n: Vec3) -> [i32; 3] {
        self.calls.push(([0.; 3], n));
        [257, 514, 771]
    }
}

#[test]
fn wall_range_raw_order_bounds_and_tail_literals() {
    let expected = [
        (
            [2, 5],
            3,
            [2, 5],
            "across [2, 4]",
            [0x42000106, 0x423ffefa, 0x42a00083, 0x42a00106],
        ),
        (
            [3, 5],
            6,
            [2, 5],
            "across [3, 5]",
            [0x42400106, 0x4240020c, 0x42a00083, 0x42bfff7d],
        ),
        (
            [2, 6],
            3,
            [2, 5],
            "across [2, 6]",
            [0x42000106, 0x423ffefa, 0x42c00083, 0x42c00106],
        ),
        (
            [2, 5],
            6,
            [2, 5],
            "across [1, 5]",
            [0x42000106, 0x4200020c, 0x42a00083, 0x42bfff7d],
        ),
    ];
    for (side, (start, end, inner, across, bits)) in Edge::ALL.into_iter().zip(expected) {
        assert_eq!(
            WallRange::new(side, wall_box()).unwrap(),
            WallRange { start, end, inner }
        );
        let mut shape = WallShape::empty(side);
        let mut host = raw_wall(&[12., 14.]);
        assert!(shape.rebuild(wall_box(), 0.0625, &mut host).unwrap());
        assert_eq!(host.calls[0], "inner [2, 5]");
        assert_eq!(host.calls[1], format!("height {start:?}"));
        assert_eq!(&host.calls[3..], ["width", "depth", across]);
        assert_eq!(
            [
                shape.bounds.min[0].to_bits(),
                shape.bounds.max[0].to_bits(),
                shape.bounds.min[2].to_bits(),
                shape.bounds.max[2].to_bits()
            ],
            bits
        );
        assert_eq!(shape.emitting_segments, 1);
        let tail = shape.records[3..].to_vec();
        shape.rebuild(wall_box(), 0.0625, &mut raw_wall(&[12., 8.])).unwrap();
        assert_eq!(shape.records.len(), 6);
        assert_eq!(&shape.records[3..], tail);
        assert_eq!(shape.emitting_segments, 1);
        let mut light = RawLight::default();
        shape.recolour(&mut light);
        assert_eq!(light.calls.len(), 6);
        assert!(shape.records.iter().all(|v| v.colour == [3, 2, 1, 255]));
        assert!(light.calls.iter().all(|(_, n)| *n == side.normal().map(|v| -v)));
        let retained = shape.records.clone();
        let mut host = raw_wall(&[8., 10., 8.]);
        assert!(!shape.rebuild(wall_box(), 0.0625, &mut host).unwrap());
        assert_eq!(shape.records, retained);
        assert_eq!(shape.emitting_segments, 0);
        assert!(!shape.geometry_present);
        assert_eq!(host.calls.len(), 4); // inner, first, next, fallback; no neighbour.
    }
}

#[test]
fn wall_empty_narrow_ties_flat_neighbour_and_missing_inner() {
    let mut shape = WallShape::empty(Edge::North);
    for max in [32., 34.] {
        let mut bounds = wall_box();
        bounds.max[0] = max;
        let mut host = raw_wall(&[12., 12.]);
        assert!(!shape.rebuild(bounds, 0.0625, &mut host).unwrap());
        assert_eq!(host.calls, ["inner [2, 5]", "height [2, 5]", "height [2, 5]"]);
        assert_eq!(
            shape.bounds,
            Bounds {
                min: [32., 12., 80.],
                max: [33., 13., 81.]
            }
        );
    }
    for (x, cell) in [(8., 0), (24., 2)] {
        let mut bounds = wall_box();
        bounds.min[0] = x;
        assert_eq!(WallRange::new(Edge::North, bounds).unwrap().start[0], cell);
    }
    let mut host = raw_wall(&[12., 14., 12.]);
    host.flags = Some(1);
    assert!(!shape.rebuild(wall_box(), 0.0625, &mut host).unwrap());
    assert_eq!(
        host.calls,
        [
            "inner [2, 5]",
            "height [2, 5]",
            "height [3, 5]",
            "width",
            "depth",
            "across [2, 4]",
            "height [2, 5]"
        ]
    );
    let previous = shape.clone();
    let mut host = raw_wall(&[]);
    host.lot = None;
    assert!(!shape.rebuild(wall_box(), 1., &mut host).unwrap());
    assert_eq!(shape.bounds, previous.bounds);
    assert_eq!(shape.records, previous.records);
}

#[test]
fn settled_owner_zero_same_fallback_and_key_mutation() {
    let mut f = FoundationModel::<Binding>::new(
        Footprint {
            min: [0., 0.],
            max: [16., 16.],
        },
        8.,
        40.,
        16.,
        FoundationTextureKeys {
            sides: [Some(0); 5],
            tops: [None; 5],
        },
    )
    .unwrap();
    let mut factory = Factory::default();
    f.load_textures(zoom(0), &mut factory);
    assert!(factory.calls.is_empty());
    assert!(f.side_texture().is_none() && f.top_texture().is_none());
    let mut w = wall(Edge::North);
    let mut factory = Factory::replying(&[None, None]);
    w.load_texture(zoom(0), &mut factory);
    assert_eq!(
        factory.calls,
        [TextureKey::Wall {
            group: 0x891b0e1a,
            instance: 0x695c00c6
        }; 2]
    );
    assert!(w.texture().is_none());
    assert_eq!(w.keys()[0], Some(0x695c00c6));
    let mut f = foundation(8.);
    let mut factory = Factory::replying(&[None, None, None, None]);
    f.load_textures(zoom(0), &mut factory);
    assert_eq!(
        factory.calls,
        [
            TextureKey::FoundationSide(0x69f20000),
            TextureKey::FoundationSide(0x69f20000),
            TextureKey::FoundationTop(0x0912220f),
            TextureKey::FoundationTop(0x0912220f)
        ]
    );
    f.load_textures(zoom(0), &mut factory);
    assert_eq!(factory.calls.len(), 4); // failed load is still cached.
    let mut rec = record();
    rec.texture_key = 0;
    let mut c = CellTextureModel::<Binding>::from_record(rec);
    c.load_texture(zoom(3), &mut factory).unwrap();
    assert_eq!(factory.calls.len(), 4);
    assert!(c.texture().is_none());
    assert_eq!(c.sort_value(), 0);
}

#[test]
fn foundation_quad_uv_order_and_raw_normal_colours() {
    let mut f = foundation(8.);
    let positions = [
        [[32., 40., 0.], [32., 8., 0.], [0., 8., 0.], [0., 40., 0.]],
        [[32., 40., 48.], [32., 8., 48.], [32., 8., 0.], [32., 40., 0.]],
        [[0., 40., 48.], [0., 8., 48.], [32., 8., 48.], [32., 40., 48.]],
        [[0., 40., 0.], [0., 8., 0.], [0., 8., 48.], [0., 40., 48.]],
    ];
    for (face, expected) in f.shape().sides.iter().zip(positions) {
        assert_eq!(face.vertices.map(|v| v.position), expected);
        assert_eq!(face.vertices.map(|v| v.uv), [[0., 0.], [0., 2.], [2., 2.], [2., 0.]]);
    }
    assert_eq!(
        f.shape().top.vertices.map(|v| v.position),
        [[0., 40., 0.], [0., 40., 48.], [32., 40., 48.], [32., 40., 0.]]
    );
    let mut light = RawLight::default();
    f.recolour(&mut light);
    assert_eq!(light.calls.len(), 5);
    assert!(f.shape().top.vertices.iter().all(|v| v.colour == [3, 2, 1, 255]));
    f.adjust_height_to_terrain(&border(0., None), true, &mut Light::default())
        .unwrap();
    assert!(f
        .shape()
        .sides
        .iter()
        .all(|f| f.vertices.iter().all(|v| v.colour == [3, 2, 1, 255])));
    assert_eq!(light.calls.len(), 5);
}

// LM4 §8.1 analytical ledgers, not captured native executions. Expected record
// words are public literals; side coordinates and request cells are independent.
#[test]
fn settled_wall_literal_four_side_ledger() {
    type HeightRow = (f32, f32, &'static [(bool, u32)], [f32; 2]);
    let rows: &[HeightRow] = &[
        (
            12.,
            14.,
            &[
                (false, 0x411ccccd),
                (true, 0x411ccccd),
                (false, 0x41433333),
                (false, 0x41433333),
                (true, 0x411ccccd),
                (true, 0x41633333),
            ],
            [10., 14.],
        ),
        (
            10.,
            12.,
            &[
                (false, 0x411ccccd),
                (true, 0x411ccccd),
                (false, 0x41233333),
                (false, 0x41233333),
                (true, 0x411ccccd),
                (true, 0x41433333),
            ],
            [10., 12.],
        ),
        (
            12.,
            10.,
            &[
                (false, 0x411ccccd),
                (true, 0x411ccccd),
                (false, 0x41433333),
                (false, 0x41433333),
                (true, 0x411ccccd),
                (true, 0x41233333),
            ],
            [10., 12.],
        ),
        (
            12.,
            8.,
            &[(false, 0x411ccccd), (true, 0x40f9999a), (false, 0x41433333)],
            [8., 12.],
        ),
        (
            8.,
            12.,
            &[(false, 0x40f9999a), (true, 0x411ccccd), (true, 0x41433333)],
            [8., 12.],
        ),
        (10., 10., &[], [10., 10.]),
        (8., 10., &[], [8., 9.]),
        (10., 8., &[], [10., 11.]),
        (
            12.,
            f32::from_bits(0x411ccccd),
            &[(false, 0x411ccccd), (true, 0x4119999a), (false, 0x41433333)],
            [f32::from_bits(0x411ccccd), 12.],
        ),
        (
            f32::from_bits(0x411ccccd),
            12.,
            &[(false, 0x4119999a), (true, 0x411ccccd), (true, 0x41433333)],
            [f32::from_bits(0x411ccccd), 12.],
        ),
        (8., 9., &[], [8., 9.]),
    ];
    let endpoints = [
        ([32., 80.], [48., 80.]),
        ([48., 80.], [48., 96.]),
        ([32., 96.], [48., 96.]),
        ([32., 80.], [32., 96.]),
    ];
    for (side, (a, b)) in Edge::ALL.into_iter().zip(endpoints) {
        for &(h0, h1, expected, ys) in rows {
            let samples = [
                WallSample {
                    position: a,
                    terrain_height: h0,
                },
                WallSample {
                    position: b,
                    terrain_height: h1,
                },
            ];
            let shape = WallShape::from_samples(side, 10., &samples, 0.0625).unwrap();
            assert_eq!(shape.records.len(), expected.len(), "{side:?} {h0},{h1}");
            for (v, &(next, y)) in shape.records.iter().zip(expected) {
                let p = if next { b } else { a };
                assert_eq!(v.position, [p[0], f32::from_bits(y), p[1]]);
                assert_eq!(v.uv[1].to_bits(), y - 0x02000000);
                assert_eq!(v.colour, [0; 4]);
            }
            if !expected.is_empty() {
                assert_eq!([shape.bounds.min[1], shape.bounds.max[1]], ys);
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
    drops_before_lookup: Vec<Vec<u32>>,
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
        self.drops_before_lookup.push(self.drops.lock().unwrap().clone());
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
            // Explicit configured keys; DIFFERS from old None=>fallback policy.
            sides: std::array::from_fn(|i| Some(0x69f2_0000 + i as u32)),
            tops: std::array::from_fn(|i| Some(0x0912_220f + i as u32)),
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
    RetainingWallModel::init(
        side,
        10.0,
        &points,
        0.5,
        std::array::from_fn(|i| Some(0x695c_00c6 + i as u32)),
        true,
    )
    .unwrap()
}
fn layout(step: CornerStep) -> CellCornerLayout {
    // An explicitly chosen test cycle, never a claim about native strip order.
    let corners = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    CellCornerLayout::new(corners, corners, step).unwrap()
}
fn record() -> CellRecord {
    CellRecord {
        texture_key: 0x100,
        cell: [2, 3],
        orientation: Orientation::new(0).unwrap(),
        mirror: false,
        rgb: [255, 128, 64],
        alpha: 255,
        mode: 255,
    }
}
struct Source {
    lot: Option<LotSample>,
    heights: [f32; 4],
    calls: Vec<&'static str>,
}
impl CellPlacementSource for Source {
    fn lot_at(&mut self, cell: [i32; 2]) -> Option<LotSample> {
        assert_eq!(cell, [2, 3]);
        self.calls.push("lot");
        self.lot
    }
    fn terrain_corners(&mut self, cell: [i32; 2]) -> [f32; 4] {
        assert_eq!(cell, [2, 3]);
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
    // DIFFERS: N=C=1 retains base; old ceil policy lowered to 24.
    assert_eq!(wet.base(), 40.0);
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
        (24.0, 8.0),
        (f32::from_bits(24.0_f32.to_bits() - 1), 8.0),
        (f32::from_bits(24.0_f32.to_bits() + 1), 40.0),
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
    assert_eq!(model.base(), 8.0);
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
    assert_eq!(shape.sides.map(|v| v.repeats), [[2.0, 2.0]; 4]);
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
fn foundation_unsupported_inputs_and_settled_independent_top_draw() {
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
    let mut factory = Factory::replying(&[None, None, Some((9, false))]);
    let draws = model.draw(false, zoom(0), Edge::North, &mut factory, &mut Light::default());
    assert_eq!(draws.iter().map(|d| d.part).collect::<Vec<_>>(), [FoundationPart::Top]);
    assert_eq!(model.sort_value(), 9 | 0x2ab6);
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
    assert_eq!(
        shape.triangles().unwrap(),
        [[0., 7.8, 0.], [16., 9.8, 0.], [16., 12.2, 0.]]
    );
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
    // DIFFERS from the old fixture: §3 says zero skips, not bypasses facing.
    assert!(model.draw(false, zoom(4), Edge::East, &mut factory).is_none());
    assert!(model.draw(false, zoom(4), Edge::South, &mut factory).is_none());
    model.set_facing_test(true);
    assert!(model.draw(false, zoom(4), Edge::East, &mut factory).is_none());
    assert!(
        model
            .draw(false, zoom(4), Edge::South, &mut factory)
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
            [32.0, 20.0, 48.0],
            [48.0, 20.0, 48.0],
            [32.0, 20.0, 64.0],
            [48.0, 20.0, 64.0]
        ]
    );
    assert_eq!(
        placement.bounds,
        Bounds {
            min: [32.1, 18.0, 48.1],
            max: [47.9, 22.0, 63.9]
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
    assert_eq!(cell_mode(255, 128, false), CellMode::Alpha);
    assert_eq!(cell_mode(255, 255, false), CellMode::Opaque);
    for alpha in [0, 1, 254] {
        assert_eq!(cell_mode(255, alpha, false), CellMode::Alpha);
    }
    assert_eq!(cell_mode(255, 255, true), CellMode::Alpha);
    for mode in [0, 1, 2, 3] {
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
                assert_eq!(cell, [2, 3]);
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
        assert_eq!(request.environment.rgb, [128.0, 64.250984, 32.125492]);
        assert_eq!(request.environment.alpha, f32::from_bits(0x3b808081) * 128.0 / divisor);
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
    assert_eq!(draw.combiner.unwrap().environment.alpha.to_bits(), 0x3efeff00);
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
    // LM5 DIFFERS from the former skipped-equal-fallback fixture: two null
    // replies are needed for side zoom1, even though both attempts use the
    // same ID. No lease exists for either null; top then acquires lease3.
    let mut factory = Factory::replying(&[Some((1, false)), Some((2, false)), None, None, Some((3, false))]);
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
        assert_eq!(
            factory.calls,
            [
                TextureKey::FoundationSide(0x69f20000),
                TextureKey::FoundationTop(0x0912220f),
                TextureKey::FoundationSide(0x69f20001),
                TextureKey::FoundationSide(0x69f20001),
                TextureKey::FoundationTop(0x09122210),
            ]
        );
        assert_eq!(
            factory.drops_before_lookup,
            [vec![], vec![], vec![1], vec![1], vec![1, 2]]
        );
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
fn settled_cell_wrapped_key_differs_from_old_overflow_error() {
    let mut rec = record();
    rec.texture_key = u32::MAX;
    let mut model = cell(rec, true);
    let mut factory = Factory::replying(&[Some((9, false))]);
    model.load_texture(zoom(0), &mut factory).unwrap();
    model.load_texture(zoom(1), &mut factory).unwrap();
    assert_eq!(model.zoom(), Some(zoom(1)));
    assert!(model.texture().is_none());
    assert_eq!(factory.calls.len(), 3);
    assert_eq!(
        factory.calls[1],
        TextureKey::Cell {
            type_id: 0x7ab50e44,
            group: 0x0986135e,
            instance: 0
        }
    );
}

#[test]
fn portable_domains_do_not_silently_wrap_or_default() {
    assert_eq!(Zoom::new(5), Err(ContractError::InvalidInput));
    assert_eq!(Orientation::new(4), Err(ContractError::InvalidInput));
    assert_eq!(
        CellCornerLayout::new([[0.0, 0.0]; 4], [[0.0, 0.0]; 4], CornerStep::Forward),
        Err(ContractError::InvalidInput)
    );
    assert!(FoundationModel::<Binding>::new(
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
    .unwrap()
    .shape()
    .top
    .repeats[0]
        .is_infinite());
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
    assert!(FoundationModel::<Binding>::new(
        Footprint {
            min: [0.0, 0.0],
            max: [16.0, 16.0]
        },
        0.0,
        1.0,
        f32::from_bits(1),
        keys()
    )
    .unwrap()
    .shape()
    .top
    .repeats[0]
        .is_infinite());
}

struct FoundationSamples {
    calls: Vec<String>,
}

#[test]
fn foundation_unchecked_scale_and_signed_existing_count() {
    let mut model = foundation(80.);
    let mut light = Light::default();
    let change = model
        .adjust_height_to_terrain(&border(40., None), false, &mut light)
        .unwrap();
    // C=trunc(-24/16)=-1, N=1: a negative existing count is still compared.
    assert_eq!(change.base, 24.);
    model.set_top_scale(0., &mut light).unwrap();
    assert!(model.shape().top.repeats.into_iter().all(f32::is_infinite));
    model.set_top_scale(-16., &mut light).unwrap();
    assert_eq!(model.shape().top.repeats, [-2., -3.]);
    assert!(light.normals.is_empty());
}

#[test]
fn literal_signed_lattice_and_reversed_foundation_border() {
    // At -16 and -32 the biased words are 4B3FFFFF and 4B3FFFFE;
    // upper(-16) stores 4B3FFFFE then adds one after subtraction.
    let bounds = Bounds {
        min: [-32., 0., -16.],
        max: [-16., 0., 0.],
    };
    assert_eq!(
        WallRange::new(Edge::North, bounds).unwrap(),
        WallRange {
            start: [-2, -1],
            end: -1,
            inner: [-2, -1]
        }
    );
    // Outside the earlier arbitrary 2^24 coordinate restriction, still an
    // exact 16 m traversal and an ordinary literal bias conversion.
    let bounds = Bounds {
        min: [33_554_432., 0., 80.],
        max: [33_554_448., 0., 96.],
    };
    assert_eq!(
        WallRange::new(Edge::North, bounds).unwrap(),
        WallRange {
            start: [2_097_152, 5],
            end: 2_097_153,
            inner: [2_097_152, 5]
        }
    );
    let mut model = FoundationModel::<Binding>::new(
        Footprint {
            min: [32., 0.],
            max: [0., 16.],
        },
        41.,
        40.,
        16.,
        FoundationTextureKeys {
            sides: [None; 5],
            tops: [None; 5],
        },
    )
    .unwrap();
    let mut host = FoundationSamples { calls: Vec::new() };
    let change = model
        .adjust_from_terrain(false, &mut host, &mut Light::default())
        .unwrap();
    assert_eq!(host.calls, ["width"]); // Reversed index bounds sample nothing.
    assert_eq!(change.base, 24.); // C=0, maximumDrop=0 => N=1.
}

#[test]
fn foundation_raw_draw_recolours_only_on_rotation_and_skips_highlight() {
    let mut model = foundation(8.);
    let mut factory = Factory::replying(&[Some((30, false)), Some((31, false))]);
    let mut light = RawLight::default();
    assert!(model
        .draw_lit(true, zoom(0), Edge::North, &mut factory, &mut light)
        .is_empty());
    assert!(light.calls.is_empty());
    let draws = model.draw_lit(false, zoom(0), Edge::North, &mut factory, &mut light);
    assert_eq!(draws.len(), 3);
    assert_eq!(draws[2].face.vertices[0].colour, [3, 2, 1, 255]);
    assert_eq!(light.calls.len(), 5);
    model.draw_lit(false, zoom(0), Edge::North, &mut factory, &mut light);
    assert_eq!(light.calls.len(), 5);
    model.draw_lit(false, zoom(0), Edge::East, &mut factory, &mut light);
    assert_eq!(light.calls.len(), 10);
}
impl FoundationTerrain for FoundationSamples {
    fn width(&mut self) -> i32 {
        self.calls.push("width".into());
        4
    }
    fn ground(&mut self, index: i32) -> f32 {
        self.calls.push(format!("g{index}"));
        24.
    }
    fn water(&mut self, index: i32) -> f32 {
        self.calls.push(format!("w{index}"));
        20.
    }
}

#[test]
fn foundation_border_order_exact_multiple_and_force_without_relight() {
    let mut model = FoundationModel::<Binding>::new(
        Footprint {
            min: [0., 0.],
            max: [32., 32.],
        },
        40.,
        40.,
        16.,
        FoundationTextureKeys {
            sides: [None; 5],
            tops: [None; 5],
        },
    )
    .unwrap();
    let mut host = FoundationSamples { calls: Vec::new() };
    let mut light = Light::default();
    // x outer/z inner, corners once, centre index6 absent; ground before water.
    model.adjust_from_terrain(false, &mut host, &mut light).unwrap();
    assert_eq!(
        host.calls,
        [
            "width", "g0", "w0", "g5", "w5", "g10", "w10", "g1", "w1", "g11", "w11", "g2", "w2", "g7", "w7", "g12",
            "w12"
        ]
    );
    assert_eq!(model.base(), 8.); // exact depth16 requires N=2, C=1.
    let forced = model
        .adjust_height_to_terrain(&border(40., None), true, &mut light)
        .unwrap();
    assert_eq!(
        forced,
        HeightAdjustment {
            base: 8.,
            base_changed: false,
            rebuilt: true
        }
    );
    assert!(light.normals.is_empty());
}

#[test]
fn foundation_literal_existing_count_zero_allows_required_count_one() {
    // Public F-step-water: C=trunc(15/16)=0, N=trunc(31/16)=1.
    // The former base<=top constructor guard excluded this published input.
    let mut model = FoundationModel::<Binding>::new(
        Footprint {
            min: [0., 0.],
            max: [16., 16.],
        },
        41.,
        40.,
        16.,
        FoundationTextureKeys {
            sides: [None; 5],
            tops: [None; 5],
        },
    )
    .unwrap();
    let mut light = Light::default();
    assert_eq!(
        model
            .adjust_height_to_terrain(&border(21., Some(25.)), false, &mut light)
            .unwrap(),
        HeightAdjustment {
            base: 24.,
            base_changed: true,
            rebuilt: true,
        }
    );
    assert!(light.normals.is_empty());
}

#[derive(Default)]
struct Commands {
    trace: Vec<CellCommand>,
    queries: Vec<&'static str>,
    no_add: bool,
    fail_at: Option<usize>,
}
impl CellCommands for Commands {
    fn request(&mut self, command: CellCommand) -> Result<(), ContractError> {
        if self.fail_at == Some(self.trace.len()) {
            return Err(ContractError::InvalidInput);
        }
        self.trace.push(command);
        Ok(())
    }
    fn global_light(&mut self) -> Vec3 {
        self.queries.push("light");
        [1., 2., 3.]
    }
    fn no_add_signed(&mut self) -> bool {
        self.queries.push("capability");
        self.no_add
    }
}

#[test]
fn cell_literal_orientation_postmirror_and_unsigned_placement() {
    let tables = [
        [[0., 0.], [1., 0.], [0., 1.], [1., 1.]],
        [[0., 1.], [0., 0.], [1., 1.], [1., 0.]],
        [[1., 1.], [0., 1.], [1., 0.], [0., 0.]],
        [[1., 0.], [1., 1.], [0., 0.], [0., 1.]],
    ];
    let mirrored = [
        [[1., 0.], [0., 0.], [1., 1.], [0., 1.]],
        [[1., 1.], [1., 0.], [0., 1.], [0., 0.]],
        [[0., 1.], [1., 1.], [0., 0.], [1., 0.]],
        [[0., 0.], [0., 1.], [1., 0.], [1., 1.]],
    ];
    for i in 0..4 {
        let mut rec = record();
        rec.orientation = Orientation::new(i).unwrap();
        for (mirror, expected) in [(false, tables[i as usize]), (true, mirrored[i as usize])] {
            rec.mirror = mirror;
            let mut c = CellTextureModel::<Binding>::from_record(rec);
            c.update_vertices(true, true, &mut source(false), &mut RawLight::default())
                .unwrap();
            let p = c.placement().unwrap();
            assert_eq!(p.vertices.map(|v| v.uv), expected);
            assert_eq!(
                p.positions,
                [[32., 1., 48.], [48., 3., 48.], [32., -2., 64.], [48., 4., 64.]]
            );
            assert_eq!(p.vertices.map(|v| v.colour), [[0; 4]; 4]); // nonflat never lights
        }
    }
    struct ByteSource;
    impl CellPlacementSource for ByteSource {
        fn lot_at(&mut self, c: [i32; 2]) -> Option<LotSample> {
            assert_eq!(c, [255, 128]);
            Some(LotSample {
                flags: 1,
                altitude: 20.,
            })
        }
        fn terrain_corners(&mut self, _: [i32; 2]) -> [f32; 4] {
            panic!("flat")
        }
    }
    let mut rec = record();
    rec.cell = [255, 128];
    let mut c = CellTextureModel::<Binding>::from_record(rec);
    c.update_vertices(true, true, &mut ByteSource, &mut RawLight::default())
        .unwrap();
    assert_eq!(
        c.placement().unwrap().positions,
        [
            [4080., 20., 2048.],
            [4096., 20., 2048.],
            [4080., 20., 2064.],
            [4096., 20., 2064.]
        ]
    );
    assert_eq!(c.placement().unwrap().vertices.map(|v| v.colour), [[3, 2, 1, 255]; 4]);
}

#[test]
fn cell_all_modes_literal_state_combiner_and_light_order() {
    for (mode, alpha, no_add, state, op, alpha_word) in [
        (255, 255, false, CellPackedState::OPAQUE, None, 0),
        (255, 128, false, CellPackedState::ALPHA, None, 0),
        (
            0,
            255,
            false,
            CellPackedState::TINTED,
            Some(CellRgbOperation::SelectArg1),
            0x3f800000,
        ),
        (
            1,
            255,
            false,
            CellPackedState::TINTED,
            Some(CellRgbOperation::Modulate),
            0x3f800000,
        ),
        (
            2,
            255,
            false,
            CellPackedState::TINTED,
            Some(CellRgbOperation::Add),
            0x3f800000,
        ),
        (
            3,
            255,
            false,
            CellPackedState::TINTED,
            Some(CellRgbOperation::AddSigned),
            0x3f800000,
        ),
        (
            3,
            255,
            true,
            CellPackedState::TINTED,
            Some(CellRgbOperation::Modulate),
            0x3efeff00,
        ),
    ] {
        let mut rec = record();
        rec.mode = mode;
        rec.alpha = alpha;
        rec.rgb = [64, 128, 255];
        let mut c = CellTextureModel::<Binding>::from_record(rec);
        let mut ctx = context();
        ctx.translucent_pass = true;
        let mut commands = Commands {
            no_add,
            ..Commands::default()
        };
        assert_eq!(
            c.draw_owned(
                ctx,
                Edge::North,
                &mut source(true),
                &mut RawLight::default(),
                &mut Factory::replying(&[Some((9, false))]),
                &mut commands
            ),
            Ok(1)
        );
        assert_eq!(
            &commands.trace[..5],
            &[
                CellCommand::ModelTransform,
                CellCommand::ResetShade,
                CellCommand::Texture { word: 9, stage: 0 },
                CellCommand::State(state),
                CellCommand::Wrap(2, 2, 0)
            ]
        );
        if let Some(op) = op {
            assert_eq!(commands.trace.len(), 9);
            assert_eq!(commands.trace[5], CellCommand::Filter(1, 1, 0));
            let CellCommand::TexColour(colour) = commands.trace[6] else {
                panic!("colour order")
            };
            assert_eq!(colour.rgb.map(f32::to_bits), [0x3e808081, 0x3f008081, 0x3f800000]);
            assert_eq!(colour.alpha.to_bits(), alpha_word);
            let CellCommand::Combiner { bytes, decoded, stage } = commands.trace[7] else {
                panic!("combiner order")
            };
            let effective = if no_add { 1 } else { mode };
            assert_eq!(bytes, [effective, 0, 2, 0x21, 1, 0, 3, 1, 0, 0, 0, 0]);
            assert_eq!(stage, 0);
            assert_eq!(
                decoded,
                CellDecodedCombiner {
                    rgb_operation: op,
                    alpha_operation: 1,
                    rgb_sources: [0, 2, 1],
                    rgb_operands: [2, 2, 4],
                    alpha_sources: [0, 3, 1],
                    alpha_operands: [4, 4, 4],
                    scales: [0, 0]
                }
            );
            assert_eq!(
                commands.queries,
                if mode == 3 {
                    vec!["light", "capability"]
                } else {
                    vec!["light"]
                }
            );
        } else {
            assert_eq!(commands.trace.len(), 6);
            assert!(commands.queries.is_empty());
        }
        let CellCommand::Flat {
            primitive,
            format,
            vertices,
        } = commands.trace.last().unwrap()
        else {
            panic!("flat order")
        };
        assert_eq!((*primitive, *format), (1, 10));
        assert_eq!(vertices.map(|v| v.colour), [[3, 2, 1, alpha]; 4]);
    }
    assert_eq!(
        CellPackedState::OPAQUE,
        (CellPackedState {
            value: 0x020001a7,
            mask: 0x1e0003ff
        })
    );
    assert_eq!(
        CellPackedState::ALPHA,
        (CellPackedState {
            value: 0x020a91bf,
            mask: 0x1fffffff
        })
    );
    assert_eq!(
        CellPackedState::TINTED,
        (CellPackedState {
            value: 0x000a80b5,
            mask: 0x101fe3ff
        })
    );
    assert_eq!(CellPackedState::TINTED.apply(0x020a91bf), 0x020a90b5);
    assert_eq!(0x020a91bf ^ CellPackedState::TINTED.apply(0x020a91bf), 0x10a);
}

#[test]
fn cell_first_zoom_later_recolour_settexture_and_pass_prefix() {
    let mut c = CellTextureModel::<Binding>::from_record(record());
    let mut src = source(true);
    let mut light = RawLight::default();
    let mut factory = Factory::replying(&[Some((11, false)), Some((12, false)), Some((13, false))]);
    let mut commands = Commands::default();
    let mut ctx = context();
    assert_eq!(
        c.draw_owned(ctx, Edge::North, &mut src, &mut light, &mut factory, &mut commands),
        Ok(1)
    );
    assert_eq!(src.calls, ["lot"]);
    assert_eq!(light.calls.len(), 8); // first zoom plus first rotation
    let before = c.placement().unwrap().clone();
    src.lot.as_mut().unwrap().altitude = 500.;
    ctx.zoom = zoom(3);
    commands.trace.clear();
    c.draw_owned(ctx, Edge::North, &mut src, &mut light, &mut factory, &mut commands)
        .unwrap();
    assert_eq!(src.calls, ["lot"]);
    assert_eq!(light.calls.len(), 12);
    assert_eq!(c.placement().unwrap().positions, before.positions);
    let mut rec = record();
    rec.mode = 0;
    rec.alpha = 1;
    rec.orientation = Orientation::new(1).unwrap();
    c.set_texture(rec, zoom(3), &mut factory).unwrap();
    let after = c.placement().unwrap();
    assert_eq!(after.positions, before.positions);
    assert_eq!(after.bounds, before.bounds);
    assert_eq!(after.vertices.map(|v| v.colour), before.vertices.map(|v| v.colour));
    assert_eq!(after.vertices.map(|v| v.uv), [[0., 1.], [0., 0.], [1., 1.], [1., 0.]]);
    commands.trace.clear();
    ctx.terrain_mode = 8;
    assert_eq!(
        c.draw_owned(ctx, Edge::North, &mut src, &mut light, &mut factory, &mut commands),
        Ok(2)
    );
    assert!(commands.trace.is_empty()); // force8 cannot bypass pass0
    assert_eq!(factory.calls.len(), 3);
}

#[test]
fn cell_reload_before_null_and_pass_failures_and_foreign_partial_prefix() {
    for (reply, expected) in [(None, 1), (Some((8, true)), 2)] {
        let mut rec = record();
        rec.mode = 0;
        let mut c = CellTextureModel::<Binding>::from_record(rec);
        let mut src = source(true);
        let mut light = RawLight::default();
        let mut commands = Commands::default();
        let mut factory = Factory::replying(&[reply, None]);
        assert_eq!(
            c.draw_owned(
                context(),
                Edge::North,
                &mut src,
                &mut light,
                &mut factory,
                &mut commands
            ),
            Ok(expected)
        );
        assert_eq!(src.calls, ["lot"]);
        assert_eq!(light.calls.len(), 8);
        assert_eq!(c.zoom(), Some(zoom(2)));
        assert!(commands.trace.is_empty());
        let calls = factory.calls.len();
        c.draw_owned(
            context(),
            Edge::North,
            &mut src,
            &mut light,
            &mut factory,
            &mut commands,
        )
        .unwrap();
        assert_eq!(factory.calls.len(), calls);
        assert_eq!(light.calls.len(), 8);
    }
    let mut c = CellTextureModel::<Binding>::from_record(record());
    let mut commands = Commands {
        fail_at: Some(3),
        ..Commands::default()
    };
    assert_eq!(
        c.draw_owned(
            context(),
            Edge::North,
            &mut source(true),
            &mut RawLight::default(),
            &mut Factory::replying(&[Some((22, false))]),
            &mut commands
        ),
        Err(ContractError::InvalidInput)
    );
    assert_eq!(
        commands.trace,
        [
            CellCommand::ModelTransform,
            CellCommand::ResetShade,
            CellCommand::Texture { word: 22, stage: 0 }
        ]
    );
    assert_eq!(c.zoom(), Some(zoom(2)));
    assert!(c.placement().is_some());
}

#[test]
fn cell_nonflat_fallback_keeps_original_alpha_and_force_flag() {
    let mut rec = record();
    rec.mode = 3;
    rec.alpha = 255;
    let mut c = CellTextureModel::<Binding>::from_record(rec);
    let mut ctx = context();
    ctx.translucent_pass = true;
    ctx.terrain_mode = 8;
    let mut commands = Commands {
        no_add: true,
        ..Commands::default()
    };
    let mut light = RawLight::default();
    c.draw_owned(
        ctx,
        Edge::North,
        &mut source(false),
        &mut light,
        &mut Factory::replying(&[Some((9, false))]),
        &mut commands,
    )
    .unwrap();
    assert!(light.calls.is_empty());
    assert_eq!(
        commands.trace.last(),
        Some(&CellCommand::Terrain {
            cell: [2, 3],
            corners: [[0., 0.], [1., 0.], [0., 1.], [1., 1.]],
            alpha: 1.,
            forced: true
        })
    );
}

#[test]
fn cell_zoom_element_selection_and_fallback_zero_are_literal() {
    struct Elements;
    impl Texture for Elements {
        fn id(&self) -> u32 {
            0xabcdef00
        }
        fn has_alpha(&self) -> bool {
            false
        }
        fn element_count(&self) -> usize {
            3
        }
        fn element_word(&self, index: usize) -> Option<u32> {
            [0x101, 0x202, 0x303].get(index).copied()
        }
    }
    struct ElementFactory;
    impl TextureFactory for ElementFactory {
        type Binding = Elements;
        fn load(&mut self, _: TextureKey) -> Option<Elements> {
            Some(Elements)
        }
    }
    let mut c = CellTextureModel::<Elements>::from_record(record());
    let mut ctx = context();
    let mut commands = Commands::default();
    c.draw_owned(
        ctx,
        Edge::North,
        &mut source(true),
        &mut RawLight::default(),
        &mut ElementFactory,
        &mut commands,
    )
    .unwrap();
    assert_eq!(commands.trace[2], CellCommand::Texture { word: 0x303, stage: 0 });
    assert_eq!(c.sort_value(), 0xabcdef00); // first texture word, not key0x100 or selected zoom element
    ctx.zoom = zoom(4);
    commands.trace.clear();
    c.draw_owned(
        ctx,
        Edge::North,
        &mut source(true),
        &mut RawLight::default(),
        &mut ElementFactory,
        &mut commands,
    )
    .unwrap();
    assert_eq!(commands.trace[2], CellCommand::Texture { word: 0x101, stage: 0 });
    let mut c = CellTextureModel::<Binding>::from_record(record());
    let mut factory = Factory::replying(&[None, Some((1, false))]);
    c.load_texture(zoom(0), &mut factory).unwrap();
    assert_eq!(
        factory.calls,
        [TextureKey::Cell {
            type_id: 0x7ab50e44,
            group: 0x0986135e,
            instance: 0x100
        }; 2]
    );
    let mut rec = record();
    rec.texture_key = 0;
    c.set_texture(rec, zoom(0), &mut factory).unwrap();
    assert_eq!(factory.calls.len(), 2);
    assert!(c.texture().is_none());
    assert_eq!(*factory.drops.lock().unwrap(), [1]);
}

#[test]
fn sampled_wall_model_preserves_stored_tail_and_colours() {
    let mut model =
        RetainingWallModel::<Binding>::init(Edge::North, 10., &samples(12., 14.), 0.0625, [None; 5], false).unwrap();
    let mut light = RawLight::default();
    model.draw_lit(false, zoom(0), Edge::North, &mut Factory::default(), &mut light);
    let tail = model.shape().records[3..].to_vec();
    model.setup_vertices(10., &samples(12., 8.), 0.0625).unwrap();
    assert_eq!(model.shape().records.len(), 6);
    assert_eq!(model.shape().emitting_segments, 1);
    assert_eq!(&model.shape().records[3..], tail);
    assert!(model.shape().records.iter().all(|v| v.colour == [3, 2, 1, 255]));
}
