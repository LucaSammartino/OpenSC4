//! Discriminating vectors for NO21–23/25/26, not original owner outputs.
use super::model::*;
use super::record::Vertex;
use std::sync::{Arc, Mutex};

const CONSTANTS: ProcessorConstants = ProcessorConstants {
    bind_colour: 0xFF80_8080,
    standard_source_format: 2,
    processor_format: 10,
}; // Explicit fixture values, not asserted native constants (NO27).

fn vertex(x: f32, y: f32, z: f32) -> Vertex {
    Vertex {
        pos: [x, y, z],
        u: x / 4.0,
        v: z / 4.0,
        color: 0x1234_5678,
    }
}

fn mesh(vertices: Vec<Vertex>) -> Mesh {
    let count = vertices.len();
    Mesh {
        format: 2,
        vertices: Arc::new(vertices),
        indices: None,
        ranges: vec![PrimitiveRange {
            kind: 7,
            start: 0,
            count,
        }],
    }
}

struct Resource {
    mesh: Mesh,
    bounds: Bounds,
    fail: bool,
    id: u32,
    drops: Arc<Mutex<Vec<u32>>>,
}
impl ModelResource for Resource {
    fn meshes(&self, lod: u16) -> Option<Vec<Mesh>> {
        assert_eq!(lod, 0);
        if self.fail {
            None
        } else {
            Some(vec![self.mesh.clone()])
        }
    }
    fn bounds(&self) -> Bounds {
        self.bounds
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.drops.lock().unwrap().push(self.id);
    }
}

struct Provider {
    mesh: Mesh,
    bounds: Bounds,
    id: u32,
    fail_load: bool,
    fail_mesh: bool,
    fail_load_id: Option<u32>,
    fail_mesh_id: Option<u32>,
    drops: Arc<Mutex<Vec<u32>>>,
    loads: Mutex<Vec<ResourceKey>>,
    id_args: Mutex<Vec<(u8, bool)>>,
}
impl ModelProvider for Provider {
    fn model_instance_id(&self, rotation: u8, alternate: bool) -> u32 {
        self.id_args.lock().unwrap().push((rotation, alternate));
        self.id
    }
    fn load(&self, key: ResourceKey) -> Option<Arc<dyn ModelResource>> {
        self.loads.lock().unwrap().push(key);
        if self.fail_load || self.fail_load_id == Some(key.instance_id()) {
            return None;
        }
        Some(Arc::new(Resource {
            mesh: self.mesh.clone(),
            bounds: self.bounds,
            fail: self.fail_mesh || self.fail_mesh_id == Some(key.instance_id()),
            id: key.instance_id(),
            drops: Arc::clone(&self.drops),
        }))
    }
}

fn provider(vertices: Vec<Vertex>) -> Provider {
    let mut bounds = Bounds {
        min: [f32::MAX; 3],
        max: [-f32::MAX; 3],
    };
    for v in &vertices {
        for i in 0..3 {
            bounds.min[i] = bounds.min[i].min(v.pos[i]);
            bounds.max[i] = bounds.max[i].max(v.pos[i]);
        }
    }
    Provider {
        mesh: mesh(vertices),
        bounds,
        id: 7,
        fail_load: false,
        fail_mesh: false,
        fail_load_id: None,
        fail_mesh_id: None,
        drops: Arc::new(Mutex::new(Vec::new())),
        loads: Mutex::new(Vec::new()),
        id_args: Mutex::new(Vec::new()),
    }
}

fn instance(p: &Provider) -> NetworkModelInstance {
    let key = ResourceKey::MainInstance(p.id);
    NetworkModelInstance::create(key, p.load(key).unwrap()).unwrap()
}

fn slab(lo: f32, hi: f32) -> Bounds {
    Bounds {
        min: [-f32::MAX, lo, -f32::MAX],
        max: [f32::MAX, hi, f32::MAX],
    }
}

fn state() -> DrawState {
    DrawState {
        position: [0.0; 3],
        model_offset: [0.0; 3],
        rotation: 0,
        orientation_byte: 0,
        network_flags: 0,
        corners: [0.0, 4.0, 8.0, 0.0],
        terrain_heights: [0.0; 4],
        path_id: 0,
        path_origin: [0.0; 2],
        object_yaw: 0.0,
        use_object_yaw: true,
        view_yaw: 0.0,
        option_27812821: false,
        option_6a845768: false,
        alpha: 255,
        shadow_byte: 1,
        second_id: 0,
        second_yaw: 0.0,
        second_offset: [0.0; 3],
    }
}

fn piece() -> PrebuiltPiece {
    PrebuiltPiece::new([Vertex::default(); 4], Vec::new(), 0)
}

fn model(q: &DrawQuery, i: usize) -> &Arc<NetworkModelInstance> {
    match &q.instances[i] {
        PieceRender::Model(m) => m,
        _ => panic!("expected heap model"),
    }
}

#[test]
fn deform_two_planes_centred_corners_midpoint_and_extrapolation() {
    // Noncoplanar quad discriminates against the former bilinear draft.
    let points = vec![
        vertex(-8.0, 0.0, -8.0),
        vertex(-8.0, 0.0, 8.0),
        vertex(8.0, 0.0, 8.0),
        vertex(8.0, 0.0, -8.0),
        vertex(0.0, 0.0, 0.0),
        vertex(12.0, 0.0, 0.0),
        vertex(0.0, 0.0, 16.0),
    ];
    for (diagonal, expected) in [
        (false, vec![0.0, 4.0, 8.0, 0.0, 4.0, 4.0, 8.0]),
        (true, vec![0.0, 4.0, 8.0, 0.0, 2.0, 5.0, 10.0]),
    ] {
        let p = provider(points.clone());
        let mut m = instance(&p);
        assert!(m.bind(CONSTANTS));
        m.deform(Deformation::from_corners([0.0, 4.0, 8.0, 0.0], 0.0, diagonal));
        assert_eq!(
            m.meshes[0].vertices.iter().map(|v| v.pos[1]).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(m.bounds.min, [-8.0, 0.0, -8.0]);
        assert_eq!(m.bounds.max, [12.0, if diagonal { 10.0 } else { 8.0 }, 16.0]);
        assert_eq!(&*p.mesh.vertices, &points);
    }
}

#[test]
fn deform_rotation_flip_origin_relative_y_relief_and_height_point_two() {
    let p = provider(vec![vertex(8.0, 0.5, -8.0)]);
    for (rotation, flip, expected) in [
        (0, false, 0.5),
        (1, false, 8.5),
        (2, false, 4.5),
        (3, false, 0.5),
        (4, false, 0.5),
        (0, true, 0.5),
    ] {
        let mut m = instance(&p);
        let mut d = Deformation::from_corners([0.0, 4.0, 8.0, 0.0], 0.0, false);
        d.rotation = rotation;
        d.flip = flip;
        m.deform(d);
        assert_eq!(m.meshes[0].vertices[0].pos, [8.0, expected, -8.0]);
    }
    let mut m = instance(&p);
    let mut d = Deformation::from_corners([10.0; 4], 10.0, false);
    d.origin = [-3.0, -5.0];
    m.deform(d);
    assert_eq!(m.meshes[0].vertices[0].pos[1], 0.5);
    let mut m = instance(&p);
    m.deform(Deformation::from_terrain([0.0; 4], 0.0, false));
    assert_eq!(m.meshes[0].vertices[0].pos[1], 0.7);
    // Rotation -> flip -> origin; not a rotated origin or clamped sample.
    let mut m = instance(&p);
    let mut d = Deformation::from_corners([0.0, 4.0, 8.0, 0.0], 0.0, false);
    d.rotation = 1;
    d.flip = true;
    d.origin = [-4.0, 0.0];
    m.deform(d);
    assert_eq!(m.meshes[0].vertices[0].pos[1], 5.5);
}

#[test]
fn bind_private_copy_colour_bounds_source_lifetime_and_rejection() {
    let p = provider(vec![vertex(1.0, 2.0, 3.0)]);
    let mut m = instance(&p);
    let bounds = m.bounds;
    assert!(Arc::ptr_eq(&m.meshes[0].vertices, &p.mesh.vertices));
    assert!(m.bind(CONSTANTS));
    assert!(!Arc::ptr_eq(&m.meshes[0].vertices, &p.mesh.vertices));
    assert_eq!(m.meshes[0].vertices[0].color, CONSTANTS.bind_colour);
    assert_eq!(p.mesh.vertices[0].color, 0x1234_5678);
    assert_eq!(m.meshes[0].vertices[0].u, p.mesh.vertices[0].u);
    assert_eq!(m.bounds, bounds);
    assert_eq!(m.retained_source_arrays(), 1);
    m.flip_and_light(false);
    assert_eq!(m.retained_source_arrays(), 0);
    let mut m = instance(&p);
    m.meshes[0].format = 99;
    assert!(!m.bind(CONSTANTS));
    assert_eq!(m.retained_source_arrays(), 0);
    assert!(Arc::ptr_eq(&m.meshes[0].vertices, &p.mesh.vertices));
}

#[test]
fn flip_changes_transform_row_and_winding_not_positions_or_uvs() {
    let mut p = provider((0..9).map(|i| vertex(i as f32, 0.0, 0.0)).collect());
    p.mesh.indices = Some(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    p.mesh.ranges = vec![
        PrimitiveRange {
            kind: 4,
            start: 0,
            count: 3,
        },
        PrimitiveRange {
            kind: 9,
            start: 3,
            count: 6,
        },
    ];
    let mut m = instance(&p);
    m.transform = ModelTransform::new(0.7, [1.0, 2.0, 3.0], true);
    let normal = ModelTransform::new(0.7, [1.0, 2.0, 3.0], false);
    for i in 0..9 {
        assert_eq!(
            m.transform.rotation[i],
            if i < 3 { -normal.rotation[i] } else { normal.rotation[i] }
        );
    }
    m.flip_and_light(true);
    assert_eq!(m.meshes[0].indices.as_deref(), Some(&[8, 7, 6, 5, 4, 3, 2, 1, 0][..]));
    assert_eq!(m.meshes[0].ranges[0].start, 6);
    assert_eq!(m.meshes[0].ranges[1].start, 0);
    assert_eq!(m.meshes[0].vertices, p.mesh.vertices);
    assert_eq!(m.lighting.unwrap().transform.rotation, m.transform.rotation.map(|v| -v));
    assert_eq!(m.lighting.unwrap().transform.translation, [1.0, 2.0, 3.0]);
    p.mesh.indices = None;
    p.mesh.ranges = vec![
        PrimitiveRange {
            kind: 7,
            start: 2,
            count: 3,
        },
        PrimitiveRange {
            kind: 8,
            start: 5,
            count: 3,
        },
    ];
    let mut m = instance(&p);
    m.flip_and_light(true);
    assert_eq!(m.meshes[0].indices.as_deref(), Some(&[7, 6, 5, 4, 3, 2][..]));
    assert_eq!(m.meshes[0].ranges.iter().map(|r| r.start).collect::<Vec<_>>(), [3, 0]);
}

#[test]
fn no25_a_fan_and_uvs_b_single_triangle_c_drop_and_inclusive_boundary() {
    let p = provider(vec![
        vertex(0.0, -2.0, 0.0),
        vertex(4.0, 2.0, 0.0),
        vertex(0.0, 2.0, 4.0),
    ]);
    let mut m = instance(&p);
    assert_eq!(m.clip(Some(slab(0.0, f32::MAX)), 1), Ok(true));
    assert_eq!(
        m.meshes[0]
            .vertices
            .iter()
            .map(|v| (v.pos, v.u, v.v))
            .collect::<Vec<_>>(),
        vec![
            ([2.0, 0.0, 0.0], 0.5, 0.0),
            ([4.0, 2.0, 0.0], 1.0, 0.0),
            ([0.0, 2.0, 4.0], 0.0, 1.0),
            ([0.0, 0.0, 2.0], 0.0, 0.5)
        ]
    );
    assert_eq!(m.meshes[0].indices.as_deref(), Some(&[0, 1, 2, 0, 2, 3][..]));
    assert_eq!(m.bounds.min[1], 0.0);
    let p = provider(vec![
        vertex(0.0, -2.0, 0.0),
        vertex(4.0, -2.0, 0.0),
        vertex(0.0, 2.0, 0.0),
    ]);
    let mut m = instance(&p);
    m.clip(Some(slab(0.0, f32::MAX)), 1).unwrap();
    assert_eq!(
        m.meshes[0].vertices.iter().map(|v| v.pos).collect::<Vec<_>>(),
        [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 0.0]]
    );
    let p = provider(vec![vertex(0.0, -2.0, 0.0); 3]);
    let mut m = instance(&p);
    m.clip(Some(slab(0.0, f32::MAX)), 1).unwrap();
    assert!(m.meshes[0].vertices.is_empty() && m.meshes[0].ranges.is_empty());
    let p = provider(vec![
        vertex(0.0, 0.0, 0.0),
        vertex(4.0, 1.0, 0.0),
        vertex(0.0, 1.0, 4.0),
    ]);
    let mut m = instance(&p);
    assert_eq!(m.clip(Some(slab(0.0, 1.0)), 1), Ok(false));
    assert!(Arc::ptr_eq(&m.meshes[0].vertices, &p.mesh.vertices));
    assert_eq!(m.clip(None, 0), Ok(false));
    assert_eq!(m.clip(Some(slab(0.0, 1.0)), 0), Ok(false));
    let narrow = Bounds {
        min: [1.0, -10.0, -f32::MAX],
        max: [f32::MAX; 3],
    };
    let bounds = m.bounds;
    assert_eq!(m.clip(Some(narrow), 0), Err(UnspecifiedClipMode));
    assert_eq!(m.bounds, bounds);
}

#[test]
fn no25_d_outside_anchor_f32_t_and_extended_colour_truncation() {
    let mut out = vertex(0.0, -1.0, 0.0);
    out.color = 0x0000_00FF;
    let mut inside = vertex(3.0, 2.0, 0.0);
    inside.color = 0x0306_0900;
    let mut p = provider(vec![out, inside, vertex(0.0, 2.0, 3.0)]);
    p.mesh.format = 10;
    let mut m = instance(&p);
    m.clip(Some(slab(0.0, f32::MAX)), 1).unwrap();
    let x = m.meshes[0].vertices[0];
    assert_eq!(x.pos[0].to_bits(), 0x3F80_0000);
    assert_ne!(x.pos[0].to_bits(), 0x3F7F_FFFF);
    assert_eq!(x.color, 0x0102_03A9); // f32 intermediate/nearest rounding gives AA.
}

#[test]
fn clip_both_planes_shared_source_nonshared_cuts_and_range_rewrite() {
    let mut p = provider(vec![
        vertex(0.0, -1.0, 0.0),
        vertex(3.0, 2.0, 0.0),
        vertex(0.0, 0.5, 3.0),
    ]);
    p.mesh.indices = Some(vec![0, 1, 2, 0, 1, 2]);
    p.mesh.ranges = vec![
        PrimitiveRange {
            kind: 8,
            start: 0,
            count: 3,
        },
        PrimitiveRange {
            kind: 9,
            start: 3,
            count: 3,
        },
    ];
    let mut m = instance(&p);
    m.clip(Some(slab(0.0, 1.0)), 1).unwrap();
    let mesh = &m.meshes[0];
    assert_eq!(mesh.vertices.len(), 9); // 4 cuts per input triangle, one shared source.
    assert_eq!(mesh.indices.as_ref().unwrap().len(), 18);
    assert_eq!(
        mesh.ranges,
        vec![
            PrimitiveRange {
                kind: 8,
                start: 0,
                count: 9
            },
            PrimitiveRange {
                kind: 9,
                start: 9,
                count: 9
            }
        ]
    );
    assert_eq!(mesh.vertices[0], mesh.vertices[5]);
    assert_eq!(mesh.vertices[1], mesh.vertices[6]);
    assert_eq!(mesh.vertices.iter().filter(|v| v.pos == [0.0, 0.5, 3.0]).count(), 1);
}

#[test]
fn clip_bounds_are_clamped_not_rebuilt_empty_unsupported_and_no_ranges() {
    for format in [2, 10, 0x8000_4001, 99] {
        let mut p = provider(vec![
            vertex(0.0, -2.0, 0.0),
            vertex(1.0, 1.0, 0.0),
            vertex(0.0, 1.0, 1.0),
        ]);
        p.mesh.format = format;
        p.bounds = Bounds {
            min: [-100.0, -3.0, -200.0],
            max: [100.0, 4.0, 200.0],
        };
        let mut m = instance(&p);
        let b = Bounds {
            min: [-10.0, 0.0, -20.0],
            max: [10.0, 2.0, 20.0],
        };
        m.clip(Some(b), 1).unwrap();
        assert_eq!(m.bounds, b);
        assert_eq!(m.meshes[0].vertices.is_empty(), format == 99);
        assert_eq!(m.meshes[0].format, format);
    }
    let mut p = provider(vec![vertex(0.0, -2.0, 0.0); 3]);
    p.mesh.ranges.clear();
    let mut m = instance(&p);
    m.clip(Some(slab(0.0, 1.0)), 1).unwrap();
    assert!(m.meshes[0].vertices.is_empty());
}

#[test]
fn plain_always_embedded_zero_id_skips_load_second_and_shadow_update() {
    let mut p = provider(vec![vertex(0.0, 0.0, 0.0); 3]);
    p.id = 0;
    let mut owner = piece();
    let mut s = state();
    s.second_id = 8;
    s.position = [10.0, 20.0, 30.0];
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(GET_MODEL_INSTANCES_SLOT, 33);
    assert_eq!(q.instances.len(), 1);
    match &q.instances[0] {
        PieceRender::Embedded(e) => {
            assert!(Arc::ptr_eq(e, &owner.embedded));
            let t = e.transform.read().unwrap();
            assert_eq!(t.translation, s.position);
            assert_eq!(t.mask & 1, 1);
            assert_eq!(t.counter, 1);
        }
        _ => panic!("no model must still return embedded instance"),
    }
    assert!(p.loads.lock().unwrap().is_empty());
    assert!(!q.update_shadow);
    assert_eq!(owner.shadow_byte, 0);
}

#[test]
fn draw_pass_orders_option_exception_second_transform_and_alpha() {
    let p = provider(vec![
        vertex(0.0, -2.0, 0.0),
        vertex(4.0, 2.0, 0.0),
        vertex(0.0, 2.0, 4.0),
    ]);
    for (clip, skip, expected) in [
        (
            false,
            false,
            vec![ModelPass::Bind, ModelPass::Deform, ModelPass::FlipAndLight],
        ),
        (false, true, vec![]),
        (
            true,
            false,
            vec![ModelPass::Clip, ModelPass::Bind, ModelPass::FlipAndLight],
        ),
        (true, true, vec![ModelPass::Clip]),
    ] {
        let mut owner = piece();
        let mut s = state();
        s.option_6a845768 = skip;
        if clip {
            s.network_flags |= 0x40000;
        }
        s.second_id = 8;
        s.second_yaw = 0.5;
        s.second_offset = [1.0, 2.0, 3.0];
        s.model_offset = [4.0, 5.0, 6.0];
        s.alpha = 128;
        s.orientation_byte = 0x80;
        let q = owner.get_model_instances(&p, &s, CONSTANTS);
        assert_eq!(model(&q, 1).passes, expected);
        assert_eq!(
            model(&q, 2).passes,
            if skip {
                vec![]
            } else {
                vec![ModelPass::Bind, ModelPass::FlipAndLight]
            }
        );
        assert_eq!(model(&q, 2).transform, ModelTransform::new(0.5, [5.0, 7.0, 9.0], true));
        assert_eq!(model(&q, 1).alpha, Some(128.0 * 0.003_921_569));
        assert_eq!(model(&q, 1).draw_flags & 2, 2);
        assert_eq!(q.appendix, [1, 2]);
        assert!(q.update_shadow);
    }
}

#[test]
fn fresh_instances_caller_plus_cache_refs_and_no_accumulated_deformation() {
    let p = provider(vec![vertex(0.0, 0.0, 0.0); 3]);
    let mut owner = piece();
    let s = state();
    let q1 = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(Arc::strong_count(model(&q1, 1)), 2);
    let q2 = owner.get_model_instances(&p, &s, CONSTANTS);
    assert!(!Arc::ptr_eq(model(&q1, 1), model(&q2, 1)));
    assert_eq!(Arc::strong_count(model(&q1, 1)), 1);
    assert_eq!(Arc::strong_count(model(&q2, 1)), 2);
    assert!(Arc::ptr_eq(owner.cached_main().unwrap(), model(&q2, 1)));
    assert_eq!(model(&q1, 1).meshes[0].vertices[0].pos[1], 4.0);
    assert_eq!(model(&q2, 1).meshes[0].vertices[0].pos[1], 4.0);
    assert_eq!(p.mesh.vertices[0].pos[1], 0.0);
    assert!(!q2.update_shadow);
    drop(q1);
    assert_eq!(*p.drops.lock().unwrap(), [7]);
    drop(q2);
    assert_eq!(Arc::strong_count(owner.cached_main().unwrap()), 1);
    owner.shutdown(true);
    assert_eq!(*p.drops.lock().unwrap(), [7, 7]);
}

#[test]
fn cache_preserved_on_load_failure_cleared_on_set_model_failure() {
    let mut p = provider(vec![vertex(0.0, 0.0, 0.0); 3]);
    let mut owner = piece();
    let mut s = state();
    s.second_id = 8;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    let old_main = Arc::downgrade(owner.cached_main().unwrap());
    let old_second = Arc::downgrade(owner.cached_second().unwrap());
    drop(q);
    p.fail_load = true;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(q.instances.len(), 1);
    assert!(old_main.upgrade().is_some() && old_second.upgrade().is_some());
    p.fail_load = false;
    p.fail_mesh = true;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(q.instances.len(), 1);
    assert!(owner.cached_main().is_none() && owner.cached_second().is_none());
    assert!(old_main.upgrade().is_none() && old_second.upgrade().is_none());
}

#[test]
fn no26_shutdown_guard_release_order_and_reverse_destructor_no_double_release() {
    for shutdown in [false, true] {
        let p = provider(vec![vertex(0.0, 0.0, 0.0); 3]);
        let mut owner = piece();
        let mut s = state();
        s.second_id = 8;
        drop(owner.get_model_instances(&p, &s, CONSTANTS));
        assert!(owner.shutdown(false));
        assert!(p.drops.lock().unwrap().is_empty());
        assert!(owner.cached_main().is_some() && owner.cached_second().is_some());
        if shutdown {
            assert!(owner.shutdown(true));
            assert_eq!(*p.drops.lock().unwrap(), [7, 8]);
        }
        assert_eq!(owner.shadow_byte, 1);
        drop(owner);
        assert_eq!(*p.drops.lock().unwrap(), if shutdown { vec![7, 8] } else { vec![8, 7] });
    }
}

#[test]
fn model_state_is_safe_for_cross_thread_getters() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<NetworkModelInstance>();
    send_sync::<EmbeddedInstance>();
    send_sync::<PrebuiltPiece>();
}

#[test]
fn caches_fail_independently_and_zero_id_keeps_both_previous_instances() {
    let mut p = provider(vec![vertex(0.0, 0.0, 0.0); 3]);
    let mut owner = piece();
    let mut s = state();
    s.second_id = 8;
    drop(owner.get_model_instances(&p, &s, CONSTANTS));
    let old_main = Arc::downgrade(owner.cached_main().unwrap());
    let old_second = Arc::downgrade(owner.cached_second().unwrap());
    p.fail_load_id = Some(7);
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(q.instances.len(), 2);
    assert_eq!(model(&q, 1).key, ResourceKey::network(8));
    assert!(old_main.upgrade().is_some());
    assert!(old_second.upgrade().is_none());
    drop(q);
    p.fail_load_id = None;
    p.fail_mesh_id = Some(8);
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(q.instances.len(), 2);
    assert_eq!(model(&q, 1).key, ResourceKey::MainInstance(7));
    assert!(owner.cached_second().is_none());
    drop(q);
    p.fail_mesh_id = None;
    drop(owner.get_model_instances(&p, &s, CONSTANTS));
    let a = Arc::downgrade(owner.cached_main().unwrap());
    let b = Arc::downgrade(owner.cached_second().unwrap());
    let load_count = p.loads.lock().unwrap().len();
    p.id = 0;
    s.shadow_byte = 2;
    s.rotation = 3;
    s.option_27812821 = true;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(q.instances.len(), 1);
    assert!(!q.update_shadow);
    assert_eq!(owner.shadow_byte, 1);
    assert!(a.upgrade().is_some() && b.upgrade().is_some());
    assert_eq!(p.loads.lock().unwrap().len(), load_count);
    assert_eq!(p.id_args.lock().unwrap().last(), Some(&(3, false)));
}

#[test]
fn clip_unordered_bounds_nan_coordinate_and_whole_triangle_x_rejection() {
    let mut p = provider(vec![
        vertex(f32::NAN, 0.0, 0.0),
        vertex(1.0, 0.0, 0.0),
        vertex(0.0, 0.0, 1.0),
    ]);
    p.bounds.min[1] = f32::NAN;
    let mut m = instance(&p);
    assert_eq!(m.clip(Some(slab(0.0, 1.0)), 1), Ok(true));
    assert!(m.bounds.min[1].is_nan());
    assert_eq!(m.meshes[0].vertices.len(), 3);
    assert!(m.meshes[0].vertices[0].pos[0].is_nan());
    let p = provider(vec![
        vertex(-2.0, -1.0, 0.0),
        vertex(-2.0, 1.0, 0.0),
        vertex(-2.0, 1.0, 1.0),
    ]);
    let mut m = instance(&p);
    let b = Bounds {
        min: [0.0, 0.0, -f32::MAX],
        max: [f32::MAX; 3],
    };
    m.clip(Some(b), 1).unwrap();
    assert!(m.meshes[0].vertices.is_empty());
}

#[test]
fn draw_uses_view_yaw_path_origin_and_shared_flat_model_when_skipped() {
    let p = provider(vec![vertex(8.0, 0.5, -8.0); 3]);
    let mut owner = piece();
    let mut s = state();
    s.rotation = 1;
    s.orientation_byte = 0x80;
    s.path_id = 1;
    s.path_origin = [4.0, 0.0];
    s.use_object_yaw = false;
    s.view_yaw = 0.6;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert_eq!(model(&q, 1).transform, ModelTransform::new(-0.6, [0.0; 3], true));
    assert_eq!(model(&q, 1).meshes[0].vertices[0].pos[1], 5.5);
    s.option_6a845768 = true;
    let q = owner.get_model_instances(&p, &s, CONSTANTS);
    assert!(Arc::ptr_eq(&model(&q, 1).meshes[0].vertices, &p.mesh.vertices));
    assert!(model(&q, 1).passes.is_empty());
    assert_eq!(model(&q, 1).alpha, None);
    assert_eq!(model(&q, 1).draw_flags & 2, 0);
}
