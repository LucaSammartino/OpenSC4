// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent, hand-calculated vectors from the public contracts.
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn v(p: Vec3) -> Vertex {
    Vertex {
        position: p,
        colour: 0x12345678,
        uv: [0.25, 0.75],
    }
}
fn mesh(points: &[Vec3], indices: Option<Vec<u16>>, groups: Vec<Primitive>) -> Mesh {
    Mesh {
        vertices: Some(Reference::new(VertexBlock::new(
            2,
            points.iter().copied().map(v).collect(),
        ))),
        indices: indices.map(Reference::new),
        primitives: Some(Reference::new(groups)),
    }
}
fn triangles(points: &[Vec3]) -> Mesh {
    mesh(
        points,
        None,
        vec![Primitive {
            kind: 0,
            first: 0,
            count: points.len() as u32,
        }],
    )
}
struct Resource {
    header: AnimationHeader,
    frames: Vec<ModelFrame>,
    bounds: BoundingBox,
    requests: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}
impl ModelProvider for Resource {
    fn animation_header(&self) -> AnimationHeader {
        self.header
    }
    fn bounds(&self) -> BoundingBox {
        self.bounds
    }
    fn model_frame(&self, f: u16) -> Option<ModelFrame> {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.frames.get(usize::from(f)).cloned()
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}
fn model(frames: Vec<ModelFrame>, rate: u16) -> ModelReference {
    Reference::new(Box::new(Resource {
        header: AnimationHeader {
            frame_count: frames.len() as u16,
            frame_rate: rate,
            mode: 99,
            flags: 0xFF,
            displacement: 42.,
        },
        frames,
        bounds: BoundingBox {
            min: [-1., -2., -3.],
            max: [2., 4., 6.],
        },
        requests: Arc::new(AtomicUsize::new(0)),
        drops: Arc::new(AtomicUsize::new(0)),
    }))
}
fn instance(frame: ModelFrame) -> Reference<StaticInstance> {
    let mut instance = StaticInstance::new(DrawAttributes::default());
    assert!(instance.set_model(Some(&model(vec![frame], 30))));
    Reference::new(instance)
}
fn animated(count: usize, rate: u16) -> AnimatedInstance {
    let frames = (0..count)
        .map(|i| ModelFrame {
            meshes: vec![],
            materials: vec![Material {
                textures: vec![100 + i as u32],
                ..Material::default()
            }],
        })
        .collect();
    AnimatedInstance::new(&model(frames, rate), DrawAttributes::default()).unwrap()
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}

#[test]
fn reference_query_owner_and_final_release() {
    struct Dropped(Arc<AtomicUsize>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let owner: OwnerReference = Reference::new(Box::new(Dropped(drops.clone())));
    let basic = Reference::new(BasicInstance::new(
        7,
        0x89abcdef,
        DrawAttributes {
            flags: 9,
            opaque: [0xCC; 16],
        },
    ));
    basic.write().set_owner(Some(&owner));
    assert_eq!(owner.reference_count(), 2);
    basic.write().set_owner(Some(&owner));
    assert_eq!(owner.reference_count(), 2);
    assert!(basic.query_interface(0xFFFFFFFF).is_none());
    assert_eq!(basic.reference_count(), 1);
    let query = basic.query_interface(IID_BASIC).unwrap();
    assert_eq!(query.part, InterfacePart::Basic);
    assert_eq!(basic.reference_count(), 2);
    assert_eq!(query.owner.release(), 1);
    assert_eq!(owner.release(), 1);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert_eq!(basic.release(), 0);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn base_delegation_attributes_and_identity() {
    struct Derived;
    impl BasicDerived for Derived {
        fn world_bounds(&self) -> BoundingBox {
            BoundingBox {
                min: [1., 2., 3.],
                max: [4., 5., 6.],
            }
        }
        fn sort_value(&self) -> u32 {
            81
        }
    }
    let mut basic = BasicInstance::new(
        7,
        0xFFFFFFFF,
        DrawAttributes {
            flags: 9,
            opaque: [0xA5; 16],
        },
    );
    let changed = Transform {
        dirty: true,
        kind: 7,
        rotation: [[2.; 3]; 3],
        translation: [8.; 3],
        scale: 9.,
        change_counter: 40,
    };
    basic.set_transform(&changed);
    let mut out = changed;
    basic.get_transform(&mut out);
    assert_eq!(out.rotation, IDENTITY);
    assert_eq!(out.translation, [0.; 3]);
    assert_eq!(out.scale, 1.);
    assert_eq!(out.change_counter, 41);
    assert!(!out.dirty);
    basic.get_transform(&mut out);
    assert_eq!(out.change_counter, 41);
    assert_eq!(basic.bounds(&Derived), Derived.world_bounds());
    assert_eq!(basic.sort_value(&Derived), 81);
    assert_eq!(basic.properties(), 0xFFFFFFFF);
    assert_eq!(basic.custom_data(), 7);
    basic.set_custom_data(12);
    basic.set_properties(0x12345678);
    assert_eq!(basic.custom_data(), 12);
    assert_eq!(basic.properties(), 0x12345678);
    let attrs = DrawAttributes {
        flags: 0xFEDCBA98,
        opaque: [0x35; 16],
    };
    basic.set_draw_attributes(attrs);
    assert_eq!(basic.draw_attributes(), attrs);
    assert!(basic.resource_model().is_none());
    assert!(!basic.pick(&mut PickContext {
        origin: [0.; 3],
        direction: [0.; 3],
        nearest: 100.
    }));
    basic.draw();
}

#[test]
fn asymmetric_transform_kinds_and_bounds() {
    let mut t = Transform {
        dirty: true,
        kind: 0,
        rotation: [[0., -1., 0.], [1., 0., 0.], [0., 0., -1.]],
        translation: [10., 20., 30.],
        scale: -2.,
        change_counter: 8,
    };
    let expected = [
        [2., 3., 5.],
        [12., 23., 35.],
        [-4., -6., -10.],
        [6., 14., 20.],
        [-3., 2., -5.],
        [7., 22., 25.],
        [16., 16., 40.],
        [16., 16., 40.],
    ];
    for (kind, expected) in expected.into_iter().enumerate() {
        t.kind = kind as u8;
        assert_eq!(t.point([2., 3., 5.]), expected);
    }
    t.kind = 7;
    let b = BoundingBox {
        min: [1., 2., 3.],
        max: [4., 6., 8.],
    };
    assert_eq!(
        b.transformed(&t),
        BoundingBox {
            min: [14., 12., 36.],
            max: [22., 18., 46.]
        }
    );
    assert_eq!(BoundingBox::EMPTY.transformed(&t), BoundingBox::EMPTY);
}

#[test]
fn update_bounds_empty_negative_and_center() {
    let mut block = VertexBlock::new(2, vec![v([-4., 7., 1.]), v([2., -3., 5.])]);
    assert_eq!(
        block.bounds,
        BoundingBox {
            min: [-4., -3., 1.],
            max: [2., 7., 5.]
        }
    );
    assert_eq!(block.bounds.center(), [-1., 2., 3.]);
    block.vertices.clear();
    block.update_bounds();
    assert_eq!(block.bounds, BoundingBox::EMPTY);
}

#[test]
fn static_material_properties_sort_and_mesh_references() {
    let mesh = triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]);
    let vertex = mesh.vertices.as_ref().unwrap().clone();
    let material = Material {
        flags: 0x1B3,
        source_blend: 8,
        destination_blend: 1,
        byte_18: 0xFE,
        textures: vec![0x12345678, 0x87654321],
        opaque: vec![0xAA; 16],
    };
    let mut s = StaticInstance::new(DrawAttributes {
        flags: 0x100,
        opaque: [0xA5; 16],
    });
    s.set_properties(0xFFFF0002);
    let resource = model(
        vec![ModelFrame {
            meshes: vec![mesh],
            materials: vec![material.clone()],
        }],
        0,
    );
    assert!(s.set_model(Some(&resource)));
    assert_eq!(resource.reference_count(), 2);
    assert_eq!(s.mesh_count(), 1);
    assert_eq!(s.sort_value(), 0x12345678);
    assert_eq!(s.properties(), 4);
    assert_eq!(
        s.draw_attributes(),
        DrawAttributes {
            flags: 0x10E,
            opaque: [0xA5; 16]
        }
    );
    let changed = s.material(0).unwrap();
    assert_eq!(changed.flags, 0x133);
    assert_eq!(changed.byte_18, 0);
    assert_eq!(changed.opaque, material.opaque);
    assert_eq!(vertex.reference_count(), 3);
    assert_eq!(
        s.bounds(),
        BoundingBox {
            min: [-1., -2., -3.],
            max: [2., 4., 6.]
        }
    );
    assert!(s.set_model(None));
    assert_eq!(s.mesh_count(), 0);
    assert_eq!(s.sort_value(), 0);
    assert_eq!(resource.reference_count(), 1);
}

#[test]
fn ordinary_blend_only_changes_properties() {
    let mut s = StaticInstance::new(DrawAttributes {
        flags: 0x80,
        opaque: [3; 16],
    });
    s.set_properties(0xAB02);
    let m = Material {
        flags: 0x90,
        source_blend: 4,
        destination_blend: 5,
        byte_18: 9,
        ..Material::default()
    };
    s.set_model(Some(&model(
        vec![ModelFrame {
            meshes: vec![],
            materials: vec![m.clone()],
        }],
        0,
    )));
    assert_eq!(s.properties(), 0xAB04);
    assert_eq!(s.draw_attributes().flags, 0x80);
    assert_eq!(s.material(0), Some(&m));
}

#[test]
fn static_transform_counter_bounds_query_and_draw() {
    let s = instance(ModelFrame::default());
    let query = s.query_interface(IID_UNKNOWN).unwrap();
    assert_eq!(query.part, InterfacePart::MeshContainer);
    drop(query);
    assert_eq!(s.reference_count(), 1);
    assert!(s.query_interface(IID_ANIMATED).is_none());
    let t = Transform {
        kind: 1,
        translation: [5., 6., 7.],
        change_counter: 900,
        ..Transform::identity()
    };
    s.write().set_transform(&t);
    assert_eq!(s.read().transform().change_counter, 1);
    s.write().set_transform(&t);
    assert_eq!(s.read().transform().change_counter, 2);
    assert_eq!(
        s.write().world_bounds(),
        BoundingBox {
            min: [4., 4., 4.],
            max: [7., 10., 13.]
        }
    );
    struct Draw(bool);
    impl DrawProvider for Draw {
        fn render_model_instance(
            &mut self,
            s: &StaticInstance,
            t: &Transform,
            a: DrawAttributes,
            option: bool,
        ) -> bool {
            assert_eq!(*t, s.transform());
            assert_eq!(a, s.draw_attributes());
            assert!(option);
            self.0 = true;
            true
        }
    }
    let mut draw = Draw(false);
    assert!(s.read().draw(&mut draw, true));
    assert!(draw.0);
}

#[test]
fn all_triangle_walk_types_first_and_indices() {
    let expectations = [
        (0, 6, vec![[5, 6, 7], [8, 9, 10]]),
        (1, 4, vec![[5, 6, 7], [6, 7, 8]]),
        (2, 4, vec![[5, 6, 7], [5, 7, 8]]),
        (3, 2, vec![[5, 5, 5], [6, 6, 6]]),
        (4, 4, vec![[5, 6, 5], [7, 8, 7]]),
        (5, 3, vec![[5, 6, 5], [6, 7, 6]]),
        (6, 4, vec![[5, 6, 7], [7, 9, 10]]),
        (7, 4, vec![[5, 6, 7], [7, 8, 9]]),
    ];
    for (kind, count, expected) in expectations {
        let p = Primitive { kind, first: 5, count };
        assert_eq!(p.triangle_count() as usize, expected.len());
        for (i, expected) in expected.into_iter().enumerate() {
            assert_eq!(p.triangle(i as u32), Some(expected));
        }
        assert_eq!(p.triangle(p.triangle_count()), None);
    }
    assert_eq!(
        Primitive {
            kind: 99,
            first: 0,
            count: 99
        }
        .triangle_count(),
        0
    );
    let m = mesh(
        &[[9.; 3], [2.; 3], [3.; 3]],
        Some(vec![2, 0, 1]),
        vec![Primitive {
            kind: 0,
            first: 0,
            count: 3,
        }],
    );
    let mut seen = false;
    super::geometry::walk_triangles(&m, |points, ids| {
        assert_eq!(ids, [2, 0, 1]);
        assert_eq!(points, [[3.; 3], [9.; 3], [2.; 3]]);
        seen = true;
        false
    });
    assert!(seen);
}

#[test]
fn picking_stops_within_mesh_but_walks_every_mesh_and_negative_t() {
    let far = [[0., 0., 5.], [1., 0., 5.], [0., 1., 5.]];
    let near_tri = [[0., 0., 2.], [1., 0., 2.], [0., 1., 2.]];
    let mut points = far.to_vec();
    points.extend(near_tri);
    let m = triangles(&points);
    let mut pick = PickContext {
        origin: [0.2, 0.2, 0.],
        direction: [0., 0., 1.],
        nearest: 100.,
    };
    assert!(pick.mesh_pick(&m, &Transform::identity()));
    assert_eq!(pick.nearest, 5.);
    pick.nearest = 100.;
    assert!(pick.model_pick(&[m, triangles(&near_tri)], &Transform::identity()));
    assert_eq!(pick.nearest, 2.);
    pick.origin[2] = 10.;
    assert!(pick.mesh_pick(&triangles(&far), &Transform::identity()));
    assert_eq!(pick.nearest, -5.);
}

#[test]
fn animation_frame_build_and_failure_are_transactional() {
    let requests = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let resource: ModelReference = Reference::new(Box::new(Resource {
        header: AnimationHeader {
            frame_count: 2,
            ..AnimationHeader::default()
        },
        frames: vec![
            ModelFrame::default(),
            ModelFrame {
                meshes: vec![],
                materials: vec![Material {
                    textures: vec![55],
                    ..Material::default()
                }],
            },
        ],
        bounds: BoundingBox::EMPTY,
        requests: requests.clone(),
        drops: drops.clone(),
    }));
    let mut a = AnimatedInstance::new(&resource, DrawAttributes::default()).unwrap();
    assert_eq!(requests.load(Ordering::Relaxed), 1);
    assert_eq!(a.set_frame(0), 2);
    assert_eq!(requests.load(Ordering::Relaxed), 1);
    assert_eq!(a.set_frame(2), 0);
    assert_eq!(a.frame(), 0);
    assert_eq!(a.set_frame(1), 2);
    assert_eq!(a.instance.sort_value(), 55);
    assert_eq!(requests.load(Ordering::Relaxed), 2);
    assert_eq!(a.set_animation(100), 0);
    assert_eq!(a.frame(), 1);
    drop(resource);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    drop(a);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn animation_speed_rate_zero_and_one_step_catchup() {
    let mut a = animated(5, 30);
    assert_eq!(a.mode(), 3);
    assert_eq!(a.direction(), 1);
    assert_eq!(a.speed(), -100.);
    assert_eq!(a.advance(0.), 0);
    // 30 times the *stored* float32 millisecond factor, not literal 0.03.
    assert_eq!(a.speed().to_bits(), 0x3CF5C290);
    assert_eq!(a.advance(1000.), 2);
    assert_eq!(a.frame(), 1);
    assert_eq!(a.accumulator(), 1.);
    assert_eq!(a.advance(0.), 2);
    assert_eq!(a.frame(), 2);
    assert_eq!(a.accumulator(), 0.);
    let mut zero = animated(3, 0);
    assert_eq!(zero.advance(10000.), 0);
    assert_eq!(zero.frame(), 0);
    zero.set_frame(2);
    assert_eq!(zero.frame(), 2);
    zero.set_speed(0.25);
    assert_eq!(zero.advance(2.), 0);
    assert_eq!(zero.accumulator(), 0.5);
    assert_eq!(zero.advance(2.), 2);
    assert_eq!(zero.frame(), 0);
    assert_eq!(zero.accumulator(), 0.);
}

#[test]
fn animation_pingpong_subtracts_new_direction_and_hold_zeroes_accumulator() {
    let mut a = animated(3, 1);
    a.set_anim_mode(2);
    a.set_speed(1.);
    a.set_frame(2);
    assert_eq!(a.advance(1.), 2);
    assert_eq!(a.frame(), 1);
    assert_eq!(a.direction(), -1);
    assert_eq!(a.accumulator(), 2.);
    a.set_frame(0);
    a.step();
    assert_eq!(a.frame(), 1);
    assert_eq!(a.direction(), 1);
    let mut once = animated(2, 1);
    once.set_anim_mode(1);
    once.set_speed(1.);
    once.set_frame(1);
    assert_eq!(once.advance(1.), 2);
    assert_eq!(once.direction(), 0);
    assert_eq!(once.accumulator(), 0.);
    assert_eq!(once.advance(50.), 0);
    once.set_anim_mode(3);
    once.step();
    assert_eq!(once.frame(), 1);
}

#[test]
fn animation_backwards_loop_and_single_frame() {
    let mut a = animated(3, 1);
    a.set_anim_mode(2);
    a.set_frame(2);
    a.step();
    assert_eq!(a.direction(), -1);
    a.set_frame(0);
    a.set_anim_mode(3);
    a.step();
    assert_eq!(a.frame(), 2);
    a.set_frame(0);
    a.set_anim_mode(0);
    a.step();
    assert_eq!(a.direction(), 0);
    let mut one = animated(1, 1);
    one.step();
    assert_eq!(one.frame(), 0);
    assert_eq!(one.direction(), 1);
}

#[test]
fn processor_refusal_conversion_restore_and_release_counts() {
    let a = Reference::new(animated(2, 30));
    let animated = InstanceReference::Animated(a.clone());
    let mut p = ModelProcessor::new();
    assert!(!p.set_model(Some(&animated)));
    assert!(!p.has_instance());
    assert_eq!(a.reference_count(), 2);
    let mesh = triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]);
    let original = mesh.vertices.as_ref().unwrap().clone();
    let s = instance(ModelFrame {
        meshes: vec![mesh],
        materials: vec![],
    });
    let h = InstanceReference::Static(s.clone());
    assert!(p.set_model(Some(&h)));
    assert_eq!(s.reference_count(), 4);
    assert_eq!(p.target_format(), 10);
    {
        let read = s.read();
        let block = read.mesh(0).unwrap().vertices.as_ref().unwrap().read();
        assert_eq!(block.format, 10);
        assert_eq!(
            block.vertices[1],
            Vertex {
                position: [1., 0., 0.],
                colour: 0xFFFFFFFF,
                uv: [0.25, 0.75]
            }
        );
        assert_eq!(block.bounds, original.read().bounds);
    }
    assert!(p.set_model(None));
    assert_eq!(s.reference_count(), 2);
    assert!(s.read().mesh(0).unwrap().vertices.as_ref().unwrap().ptr_eq(&original));
    p.release_model();
    p.release_model();
    assert_eq!(s.reference_count(), 2);
}

#[test]
fn processor_unsupported_format_keeps_only_instance() {
    let mesh = triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]);
    mesh.vertices.as_ref().unwrap().write().format = 10;
    let s = instance(ModelFrame {
        meshes: vec![mesh],
        materials: vec![],
    });
    let h = InstanceReference::Static(s.clone());
    let mut p = ModelProcessor::new();
    assert!(p.set_model(Some(&h)));
    assert!(p.has_instance());
    assert!(!p.has_container());
    assert_eq!(s.reference_count(), 3);
    p.create_vertex_normals();
    assert!(p.normals().is_none());
    drop(p);
    assert_eq!(s.reference_count(), 2);
}

#[test]
fn processor_equal_and_unequal_area_weighted_normals() {
    // First triangle normal (0,0,1); second (0,2,0), sharing vertex 0.
    let points = [[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 2.], [1., 0., 0.], [8.; 3]];
    let m = mesh(
        &points,
        Some(vec![0, 1, 2, 0, 3, 4]),
        vec![Primitive {
            kind: 0,
            first: 0,
            count: 6,
        }],
    );
    let s = instance(ModelFrame {
        meshes: vec![m],
        materials: vec![],
    });
    let mut p = ModelProcessor::new();
    p.set_model(Some(&InstanceReference::Static(s)));
    p.create_vertex_normals();
    let n = &p.normals().unwrap()[0];
    near(n[0][1], 2. / 3.);
    near(n[0][2], 1. / 3.);
    assert_eq!(n[5], [0.; 3]);
    assert_eq!(n[1], [0., 0., 1.]);
    let m = mesh(
        &[[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        Some(vec![0, 1, 2, 0, 3, 1]),
        vec![Primitive {
            kind: 0,
            first: 0,
            count: 6,
        }],
    );
    p.set_model(Some(&InstanceReference::Static(instance(ModelFrame {
        meshes: vec![m],
        materials: vec![],
    }))));
    p.create_vertex_normals();
    let n = p.normals().unwrap()[0][0];
    assert_eq!(n, [0., 0.5, 0.5]);
    near((n[1] * n[1] + n[2] * n[2]).sqrt(), std::f32::consts::FRAC_1_SQRT_2);
}

#[test]
fn processor_stable_mesh_order_has_largest_first_and_ties() {
    let meshes = [1., 3., 2., 3.]
        .map(|z| triangles(&[[0., 0., z], [1., 0., z], [0., 1., z]]))
        .to_vec();
    let s = instance(ModelFrame {
        meshes,
        materials: vec![],
    });
    let mut p = ModelProcessor::new();
    p.set_model(Some(&InstanceReference::Static(s)));
    p.sort_meshes([0., 0., 1.]);
    assert_eq!(p.mesh_order(), [1, 3, 2, 0]);
    p.sort_meshes([0., 0., -1.]);
    assert_eq!(p.mesh_order(), [0, 2, 1, 3]);
}

#[test]
fn processor_lighting_supplies_own_normals_and_literal_negated_rotation() {
    let s = instance(ModelFrame {
        meshes: vec![triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]])],
        materials: vec![Material::default()],
    });
    let matrix = [[0., 1., 2.], [3., 4., 5.], [6., 7., 8.]];
    s.write().set_transform(&Transform {
        rotation: matrix,
        ..Transform::identity()
    });
    struct Lighting {
        expected: Matrix3,
        calls: usize,
    }
    impl VertexLighting for Lighting {
        fn get_vertex_colors(&mut self, n: &[Vec3], r: Matrix3, c: &mut [u32]) {
            assert_eq!(r, self.expected);
            for (actual, expected) in r.into_iter().flatten().zip(self.expected.into_iter().flatten()) {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
            assert_eq!(n, [[0., 0., 1.]; 3]);
            c.copy_from_slice(&[0xFF010203, 0xFF040506, 0xFF070809]);
            self.calls += 1;
        }
    }
    let mut p = ModelProcessor::new();
    p.set_model(Some(&InstanceReference::Static(s.clone())));
    let mut light = Lighting {
        expected: matrix.map(|row| row.map(|v| -v)),
        calls: 0,
    };
    p.light_model(&mut light, true);
    assert_eq!(light.calls, 1);
    assert_eq!(s.read().material(0).unwrap().flags, 8);
    assert_eq!(
        s.read().mesh(0).unwrap().vertices.as_ref().unwrap().read().vertices[1].colour,
        0xFF040506
    );
    light.expected = matrix;
    p.light_model(&mut light, false);
    assert_eq!(light.calls, 2);
}

fn anim_bytes(keys: [u32; 5]) -> Vec<u8> {
    [
        SPRITE_ANIMATION_TYPE,
        1,
        2,
        3,
        4,
        0xFFABCDEF,
        keys[0],
        keys[1],
        keys[2],
        keys[3],
        keys[4],
        0x76543210,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect()
}
#[test]
fn sprite_shared_downward_sheet_and_opaque_header() {
    let a = SpriteAnimation::read(100, &anim_bytes([999, 998, 200, 200, 200])).unwrap();
    assert_eq!(a.header, [1, 2, 3, 4, 0xFFABCDEF]);
    assert_eq!(a.trailing_value, 0x76543210);
    assert_eq!(a.keys, [100, 100, 200, 200, 200]);
    assert_eq!(a.source_zooms, [1, 1, 4, 4, 4]);
    assert_eq!(a.scales, [0.5, 1., 32. / 146., 0.5, 1.]);
}
#[test]
fn sprite_relative_keys_and_upward_sources() {
    let a = SpriteAnimation::read(100, &anim_bytes([999, 998, 25, 25, 0])).unwrap();
    assert_eq!(a.keys, [100, 100, 125, 125, 125]);
    assert_eq!(a.source_zooms, [1, 1, 3, 3, 3]);
    assert_eq!(a.scales, [0.5, 1., 32. / 73., 1., 2.]);
    let own = SpriteAnimation::read(100, &anim_bytes([999, 998, 0, 0, 0])).unwrap();
    assert_eq!(own.keys, [100; 5]);
    assert_eq!(own.scales, [8. / 146., 16. / 146., 32. / 146., 0.5, 1.]);
}
#[test]
fn sprite_factory_short_bad_tags_and_table_exact_size() {
    let factory = SpriteAnimationFactory;
    let bytes = anim_bytes([0; 5]);
    for end in 0..bytes.len() {
        assert!(factory.serial_read(SPRITE_ANIMATION_TYPE, 100, &bytes[..end]).is_none());
    }
    assert!(factory.serial_read(0x1234, 100, &bytes).is_none());
    let a = factory.serial_read(SPRITE_ANIMATION_TYPE, 100, &bytes).unwrap();
    assert!(a.read().animation().is_some());
    let q = a.query_interface(SPRITE_ANIMATION_TYPE).unwrap();
    assert_eq!(a.reference_count(), 2);
    drop(q);
    assert_eq!(a.reference_count(), 1);
    let mut table = SPRITE_TABLE_TYPE.to_le_bytes().to_vec();
    table.extend([0xA5; 28]);
    table.extend(2_u32.to_le_bytes());
    table.extend([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let t = factory.serial_read(SPRITE_TABLE_TYPE, 0, &table).unwrap();
    assert_eq!(t.read().table().unwrap().entry_count(), 2);
    assert_eq!(
        t.read().table().unwrap().entry(1),
        Some([9, 10, 11, 12, 13, 14, 15, 16].as_slice())
    );
    assert_eq!(t.read().table().unwrap().entry(2), None);
    assert!(t.query_interface(IID_SPRITE_TABLE).is_some());
    assert!(t.query_interface(SPRITE_TABLE_TYPE).is_none());
    for end in 0..table.len() {
        assert!(SpriteAnimationTable::read(&table[..end]).is_none());
    }
    table.push(0);
    assert!(SpriteAnimationTable::read(&table).is_none());
    table[32..36].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(SpriteAnimationTable::read(&table).is_none());
}

#[test]
fn shared_getters_can_run_on_a_second_thread() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Reference<StaticInstance>>();
    let instance = instance(ModelFrame::default());
    let shared = instance.clone();
    let read = std::thread::spawn(move || {
        for _ in 0..100 {
            assert_eq!(shared.read().sort_value(), 0);
        }
    });
    for _ in 0..100 {
        instance.write().set_properties(3);
    }
    read.join().unwrap();
    assert_eq!(instance.read().properties(), 3);
}

#[test]
fn transform_numeric_rounding_is_separate_multiply_then_add() {
    // 4097^2 rounds to 16785408 before subtracting 4097*4096.
    // Fusing these two operations would instead produce 4097.
    assert_eq!(
        matrix_vector([[4097., -4096., 0.], [0.; 3], [0.; 3]], [4097., 4097., 0.]),
        [4096., 0., 0.]
    );
    let mut transform = Transform {
        dirty: true,
        change_counter: u32::MAX,
        ..Transform::identity()
    };
    transform.make_identity();
    assert_eq!(transform.change_counter, 0);
}

#[test]
fn nearest_pick_metric_and_transform_are_explicit() {
    let m = triangles(&[[0., 0., 2.], [1., 0., 2.], [0., 1., 2.]]);
    let t = Transform {
        kind: 1,
        translation: [0., 0., 4.],
        ..Transform::identity()
    };
    let mut pick = PickContext {
        origin: [0.2, 0.2, 0.],
        direction: [0., 0., 2.],
        nearest: 4.,
    };
    assert!(pick.mesh_pick(&m, &t));
    assert_eq!(pick.nearest, 3.);
    // Equal distance does not replace the nearest hit.
    assert!(!pick.mesh_pick(&m, &t));
}

#[test]
fn failed_resource_frame_does_not_replace_or_retain_models() {
    let requests = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let bad: ModelReference = Reference::new(Box::new(Resource {
        header: AnimationHeader {
            frame_count: 2,
            ..AnimationHeader::default()
        },
        frames: vec![],
        bounds: BoundingBox::EMPTY,
        requests: requests.clone(),
        drops: drops.clone(),
    }));
    assert!(AnimatedInstance::new(&bad, DrawAttributes::default()).is_none());
    assert_eq!(bad.reference_count(), 1);
    let s = instance(ModelFrame {
        meshes: vec![],
        materials: vec![Material {
            textures: vec![0xABCDEF],
            ..Material::default()
        }],
    });
    let old = s.read().resource_model().unwrap().clone();
    assert!(!s.write().set_model(Some(&bad)));
    assert_eq!(s.read().sort_value(), 0xABCDEF);
    assert!(s.read().resource_model().unwrap().ptr_eq(&old));
    assert_eq!(bad.reference_count(), 1);
    assert_eq!(requests.load(Ordering::Relaxed), 2);
    drop(bad);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn switching_processor_to_refused_model_restores_old_vertices() {
    let original = triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]);
    let vertices = original.vertices.as_ref().unwrap().clone();
    let s = instance(ModelFrame {
        meshes: vec![original],
        materials: vec![],
    });
    let handle = InstanceReference::Static(s.clone());
    let mut p = ModelProcessor::new();
    assert!(p.set_model(Some(&handle)));
    let a = Reference::new(animated(2, 30));
    let other = InstanceReference::Animated(a.clone());
    assert!(!p.set_model(Some(&other)));
    assert!(!p.has_container());
    assert!(!p.has_instance());
    assert!(p.normals().is_none());
    assert!(p.mesh_order().is_empty());
    assert!(s.read().mesh(0).unwrap().vertices.as_ref().unwrap().ptr_eq(&vertices));
    assert_eq!(s.reference_count(), 2);
    assert_eq!(a.reference_count(), 2);
}

#[test]
fn processor_drop_releases_two_instance_parts_and_does_not_restore() {
    let m = triangles(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]);
    let original = m.vertices.as_ref().unwrap().clone();
    let s = instance(ModelFrame {
        meshes: vec![m],
        materials: vec![],
    });
    let h = InstanceReference::Static(s.clone());
    let mut p = ModelProcessor::new();
    p.set_model(Some(&h));
    assert_eq!(s.reference_count(), 4);
    drop(p);
    assert_eq!(s.reference_count(), 2);
    let read = s.read();
    let processed = read.mesh(0).unwrap().vertices.as_ref().unwrap();
    assert_eq!(processed.read().format, 10);
    assert!(!processed.ptr_eq(&original));
}

#[test]
fn sorting_ties_restart_with_container_order_even_after_prior_sort() {
    let meshes = [1., 3., 2.]
        .map(|z| triangles(&[[0., 0., z], [1., 0., z], [0., 1., z]]))
        .to_vec();
    let mut p = ModelProcessor::new();
    p.set_model(Some(&InstanceReference::Static(instance(ModelFrame {
        meshes,
        materials: vec![],
    }))));
    p.sort_meshes([0., 0., 1.]);
    assert_eq!(p.mesh_order(), [1, 2, 0]);
    p.sort_meshes([0.; 3]);
    assert_eq!(p.mesh_order(), [0, 1, 2]);
    p.sort_meshes([f32::NAN; 3]);
    assert_eq!(p.mesh_order(), [0, 1, 2]);
}

#[test]
fn negative_speed_uses_direction_for_step_and_accumulator_subtraction() {
    let mut a = animated(3, 0);
    a.set_speed(-1.);
    assert_eq!(a.advance(1.), 2);
    assert_eq!(a.frame(), 1);
    assert_eq!(a.accumulator(), -2.);
    assert_eq!(a.advance(0.), 2);
    assert_eq!(a.frame(), 2);
    assert_eq!(a.accumulator(), 1.);
    let single = Reference::new(animated(1, 30));
    let query = single.query_interface(IID_ANIMATED).unwrap();
    assert_eq!(query.part, InterfacePart::Animated);
    drop(query);
    let mut processor = ModelProcessor::new();
    assert!(processor.set_model(Some(&InstanceReference::Animated(single))));
    assert!(processor.has_container());
}

#[test]
fn factory_creation_types_zero_count_table_and_bad_tags() {
    let factory = SpriteAnimationFactory;
    let anim = factory.create_instance(SPRITE_ANIMATION_TYPE).unwrap();
    assert!(anim.read().animation().is_none());
    assert_eq!(anim.read().resource_type(), SPRITE_ANIMATION_TYPE);
    assert_eq!(anim.reference_count(), 1);
    assert_eq!(anim.release(), 0);
    let table = factory.create_instance(SPRITE_TABLE_TYPE).unwrap();
    assert!(table.read().table().is_none());
    assert!(table.query_interface(IID_SPRITE_TABLE).is_some());
    assert!(table.query_interface(SPRITE_TABLE_TYPE).is_none());
    assert!(factory.create_instance(123).is_none());
    let mut bytes = SPRITE_TABLE_TYPE.to_le_bytes().to_vec();
    bytes.extend([0; 32]);
    let empty = factory.serial_read(SPRITE_TABLE_TYPE, 0, &bytes).unwrap();
    assert_eq!(empty.read().table().unwrap().entry_count(), 0);
    bytes[..4].copy_from_slice(&0_u32.to_le_bytes());
    assert!(factory.serial_read(SPRITE_TABLE_TYPE, 0, &bytes).is_none());
    let mut bytes = anim_bytes([0; 5]);
    bytes[..4].copy_from_slice(&0_u32.to_le_bytes());
    assert!(factory.serial_read(SPRITE_ANIMATION_TYPE, 1, &bytes).is_none());
}

#[test]
fn release_counts_are_atomic_and_derived_owner_drops_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner: OwnerReference = Reference::new(Box::new(Resource {
        header: AnimationHeader::default(),
        frames: vec![],
        bounds: BoundingBox::EMPTY,
        requests: Arc::new(AtomicUsize::new(0)),
        drops: drops.clone(),
    }));
    let s = instance(ModelFrame::default());
    s.write().set_owner(Some(&owner));
    s.write().set_custom_data(0x87654321);
    assert_eq!(s.read().custom_data(), 0x87654321);
    assert!(s.read().owner().unwrap().ptr_eq(&owner));
    drop(owner);
    let second = s.clone();
    let a = std::thread::spawn(move || second.release());
    let b = std::thread::spawn(move || s.release());
    let mut counts = [a.join().unwrap(), b.join().unwrap()];
    counts.sort();
    assert_eq!(counts, [0, 1]);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}
