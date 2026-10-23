// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::network_occupants::xform::{Xform, XFORM_MATRIX, XFORM_SCALE, XFORM_TRANSLATION};
use crate::rng::{FixedSeeds, RzRandom};
use crate::stream::{ByteReader, ByteWriter, OutStream};
use std::ops::Range;
use std::sync::{Arc, Mutex};

const INITIAL: FlameInitial = FlameInitial {
    made: 9,
    visible: true,
    wall: false,
    side: 0,
    position: SharedPositionWord(0.0),
};
const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 0,
    process_clock: 0,
};
const BOUNDS: [[f32; 3]; 2] = [[2.0, 0.0, 4.0], [10.0, 20.0, 16.0]];

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Create(String),
    Apply(u32, Xform),
    Start(u32),
    Stop(u32),
    Release(u32),
    BaseInit,
    Group(u32),
}

#[derive(Clone, Default)]
struct Trace(Arc<Mutex<Vec<Event>>>);
impl Trace {
    fn push(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
    fn take(&self) -> Vec<Event> {
        std::mem::take(&mut self.0.lock().unwrap())
    }
}

struct Effect {
    id: u32,
    trace: Trace,
}
impl VisualEffect for Effect {
    fn start(&mut self, argument: u32) {
        assert_eq!(argument, 1);
        self.trace.push(Event::Start(self.id));
    }
    fn stop(&mut self, argument: u32) {
        assert!(argument <= 1);
        self.trace.push(Event::Stop(self.id));
    }
    fn is_active(&mut self) -> bool {
        true
    }
    fn set_transform(&mut self, transform: &Xform) {
        self.trace.push(Event::Apply(self.id, *transform));
    }
}
impl Drop for Effect {
    fn drop(&mut self) {
        self.trace.push(Event::Release(self.id));
    }
}
#[derive(Default)]
struct Factory {
    trace: Trace,
    next: u32,
    fail: bool,
}
impl EffectFactory for Factory {
    type Effect = Effect;
    fn create(&mut self, name: &str) -> Option<Effect> {
        self.trace.push(Event::Create(name.to_owned()));
        if self.fail {
            return None;
        }
        self.next += 1;
        Some(Effect {
            id: self.next,
            trace: self.trace.clone(),
        })
    }
}

struct Base(Trace);
impl OccupantInit for Base {
    fn init(&mut self) {
        self.0.push(Event::BaseInit);
    }
    fn join_group(&mut self, group: u32) {
        self.0.push(Event::Group(group));
    }
}

struct TestMesh {
    vertices: Vec<[f32; 3]>,
    indices: Vec<usize>,
    first: Option<Range<usize>>,
}
impl TestMesh {
    fn triangle() -> Self {
        Self {
            vertices: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 0.0, 4.0]],
            indices: vec![0, 1, 2],
            first: Some(0..3),
        }
    }
}
impl Mesh for TestMesh {
    fn first_primitive(&self) -> Option<Range<usize>> {
        self.first.clone()
    }
    fn index(&self, physical_index: usize) -> Option<usize> {
        self.indices.get(physical_index).copied()
    }
    fn vertex(&self, vertex_index: usize) -> Option<[f32; 3]> {
        self.vertices.get(vertex_index).copied()
    }
}

fn placement<'a>(instances: &'a [ModelInstance<'a>], random: &'a mut RzRandom) -> Placement<'a> {
    Placement {
        instances,
        bounds: BOUNDS,
        target_type: 0,
        view_level: 1,
        seeds: &SEEDS,
        fire_random: random,
    }
}
fn random() -> RzRandom {
    RzRandom::new(1, &SEEDS)
}
fn stage(value: u32) -> Stage {
    Stage::new(value).unwrap()
}

fn temporary(position: [f32; 3]) -> Xform {
    // Independent public FF4 literal, including two negative zero words.
    Xform {
        counter: 2,
        mask: 5,
        matrix: [1., -0., 0., 0., 1., -0., 0., 0., 1.],
        translation: position,
        scale: 1.,
    }
}

#[test]
fn published_effect_name_vectors_and_stage_domain() {
    for (value, top, wall) in [
        (0, "fire_1", "wfire_1"),
        (1, "fire_2", "wfire_2"),
        (2, "fire_3", "wfire_3"),
        (3, "fire_4", "wfire_4"),
        (4, "fire_5", "wfire_5"),
    ] {
        assert_eq!(stage(value).value(), value);
        assert_eq!(stage(value).effect_name(false), top);
        assert_eq!(stage(value).effect_name(true), wall);
    }
    assert_eq!(Stage::new(5), None);
    assert_eq!(Stage::new(u32::MAX), None);
}

#[test]
fn replacement_stops_releases_creates_applies_starts_in_order() {
    let mut factory = Factory::default();
    let mut flame = Flame::from_initial(FlameInitial { wall: true, ..INITIAL });
    flame.update(stage(2), &mut factory).unwrap();
    let transform = temporary([0.; 3]);
    assert_eq!(
        factory.trace.take(),
        vec![
            Event::Create("wfire_3".into()),
            Event::Apply(1, transform),
            Event::Start(1)
        ]
    );
    flame.update(stage(0), &mut factory).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![
            Event::Stop(1),
            Event::Release(1),
            Event::Create("wfire_1".into()),
            Event::Apply(2, transform),
            Event::Start(2)
        ]
    );
    drop(flame);
    // No unpublished destruction-stop algorithm is asserted (FF2).
    assert_eq!(factory.trace.take(), vec![Event::Release(2)]);
}

#[test]
fn creation_failure_releases_old_reference_and_is_retryable() {
    let mut factory = Factory::default();
    let mut flame = Flame::from_initial(INITIAL);
    flame.update(stage(0), &mut factory).unwrap();
    factory.trace.take();
    factory.fail = true;
    flame.update(stage(4), &mut factory).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![Event::Stop(1), Event::Release(1), Event::Create("fire_5".into())]
    );
    assert!(!flame.has_effect());
    assert_eq!((flame.made(), flame.visible()), (9, true));
    factory.fail = false;
    flame.update(stage(4), &mut factory).unwrap();
    assert!(flame.has_effect());
}

#[test]
fn hidden_effect_is_transformed_but_never_started() {
    let mut factory = Factory::default();
    let mut flame = Flame::from_initial(FlameInitial {
        visible: false,
        ..INITIAL
    });
    flame.update(stage(0), &mut factory).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![Event::Create("fire_1".into()), Event::Apply(1, temporary([0.; 3]))]
    );
}

#[test]
fn published_yaw_vectors_are_radians_with_translation() {
    // FF4 DIFFERS: former libm approximate side1/2/3 tests did not qualify
    // native FSIN/FCOS. Keep the case, now testing the explicit domain boundary.
    for side in [0, 1, 2, 3, u32::MAX] {
        let mut factory = Factory::default();
        let mut flame = Flame::<Effect>::from_initial(INITIAL);
        flame.place([3.0, 4.0, 5.0], false, side, 1).unwrap(); // null-effect early return
        assert_eq!(*flame.transform(), Xform::default());
        if side == 0 {
            flame.update(stage(0), &mut factory).unwrap();
            let events = factory.trace.take();
            assert_eq!(events[1], Event::Apply(1, temporary([3., 4., 5.])));
            if let Event::Apply(_, t) = events[1] {
                assert_eq!(
                    t.matrix.map(f32::to_bits),
                    [0x3f800000, 0x80000000, 0, 0, 0x3f800000, 0x80000000, 0, 0, 0x3f800000]
                );
            }
        } else {
            assert_eq!(
                flame.update(stage(0), &mut factory),
                Err(PlacementError::Unsupported(Unsupported::HardwareTrig))
            );
        }
    }
}

fn flame_record(version: u16) -> (Vec<u8>, Xform) {
    let mut out = ByteWriter::new();
    out.put_u16(version);
    out.put_u8(1);
    out.put_u32(3);
    for value in [10.0, 20.0, 30.0] {
        out.put_f32(value);
    }
    let transform = Xform {
        counter: 17,
        mask: XFORM_MATRIX | XFORM_SCALE | XFORM_TRANSLATION,
        matrix: [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0],
        translation: [-2.0, -3.0, -4.0],
        scale: 2.0,
    };
    if version >= 3 {
        transform.write(&mut out);
    } else {
        for value in transform.matrix {
            out.put_f32(value);
        }
        for value in transform.translation {
            out.put_f32(value);
        }
        out.put_f32(transform.scale);
        out.put_u32(17);
    }
    (out.into_bytes(), transform)
}

#[test]
fn read_modern_legacy_and_future_versions_preserves_saved_transform() {
    for version in [2, 3, 4, u16::MAX] {
        let (bytes, expected) = flame_record(version);
        let mut flame = Flame::<Effect>::from_initial(INITIAL);
        assert!(flame.read(&mut ByteReader::new(&bytes)));
        assert_eq!(
            (flame.wall(), flame.side(), flame.position()),
            (true, 3, [10.0, 20.0, 30.0])
        );
        assert_eq!(flame.transform().matrix, expected.matrix);
        assert_eq!(flame.transform().translation, expected.translation);
        assert_eq!(flame.transform().scale, 2.0);
        if version == 2 {
            assert_eq!(flame.transform().counter, 17);
        }
        assert_eq!((flame.made(), flame.visible(), flame.has_effect()), (9, true, false));
    }
}

#[test]
fn read_rejects_old_versions_and_every_truncation_without_effect_calls() {
    for version in [0_u16, 1] {
        let mut flame = Flame::<Effect>::from_initial(INITIAL);
        assert!(!flame.read(&mut ByteReader::new(&version.to_le_bytes())));
        assert_eq!((flame.wall(), flame.side()), (false, 0));
    }
    for version in [2, 3] {
        let (bytes, _) = flame_record(version);
        for length in 0..bytes.len() {
            let mut flame = Flame::<Effect>::from_initial(INITIAL);
            assert!(
                !flame.read(&mut ByteReader::new(&bytes[..length])),
                "version {version}, length {length}"
            );
        }
    }
    let mut factory = Factory::default();
    let mut flame = Flame::from_initial(INITIAL);
    flame.update(stage(0), &mut factory).unwrap();
    factory.trace.take();
    let (bytes, _) = flame_record(3);
    assert!(flame.read(&mut ByteReader::new(&bytes)));
    assert!(factory.trace.take().is_empty());
    assert!(flame.has_effect());
}

#[test]
fn published_side_vectors_include_ties_and_strict_wall_boundary() {
    for (normal, wall, side) in [
        ([0.0, 0.0, -1.0], false, 1),
        ([0.6, 0.0, 0.8], true, 3),
        ([1.0, 0.0, 0.0], false, 0),
        ([-1.0, 0.0, 0.0], false, 2),
        ([1.0, 0.0, 1.0], true, 0),
        ([-1.0, 0.0, -1.0], false, 0),
        ([-1.0, 0.0, 1.0], true, 2),
        ([0.0, 1.0, 0.0], false, 0),
        ([0.0, 0.0, -0.0], false, 0),
    ] {
        assert_eq!(super::model::classify(normal), (wall, side));
    }
}

#[test]
fn biased_barycentrics_offset_then_scale_rotate_translate_vector() {
    let mesh = TestMesh::triangle();
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let transform = Xform {
        matrix: [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0],
        scale: 2.0,
        translation: [10.0, 20.0, 30.0],
        ..Xform::default()
    };
    let instances = [
        ModelInstance::Other,
        ModelInstance::MeshContainer {
            transform,
            meshes: &meshes,
        },
    ];
    // Zero seed/clock gives u=.5, v=.25, w=.25. Local (1,0,1),
    // outward normal (0,-1,0), then 2*M*p+t = (12,18,28).
    assert_eq!(
        get_points_on_model(1, &instances, &SEEDS).unwrap().unwrap(),
        vec![ModelPoint {
            position: [12.0, 18.0, 28.0],
            wall: false,
            side: 0
        }]
    );
}

#[test]
fn only_first_primitive_and_ceil_partial_triples_are_used() {
    let mut mesh = TestMesh::triangle();
    // FF3 DIFFERS from the retained original assumption (see answered FF3):
    // offset 3 is ignored; ceil(1/3) reads physical indices 0,1,2.
    mesh.indices = vec![0, 1, 2, 999, 999, 999];
    mesh.first = Some(3..4);
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(
        get_points_on_model(1, &instances, &SEEDS).unwrap().unwrap()[0].position,
        [1.0, -1.0, 1.0]
    );
}

#[test]
fn missing_mesh_data_is_explicit_error_not_a_false_box_fallback() {
    let mut mesh = TestMesh::triangle();
    for (first, indices, expected) in [
        (
            None,
            vec![0, 1, 2],
            PlacementError::Unsupported(Unsupported::UnsafeIndex),
        ),
        (
            Some(0..2),
            vec![0, 1],
            PlacementError::Unsupported(Unsupported::UnsafeIndex),
        ),
        (
            Some(0..3),
            vec![0, 1, 999],
            PlacementError::Unsupported(Unsupported::UnsafeVertex),
        ),
    ] {
        mesh.first = first;
        mesh.indices = indices;
        let meshes: [&dyn Mesh; 1] = [&mesh];
        let instances = [ModelInstance::MeshContainer {
            transform: Xform::default(),
            meshes: &meshes,
        }];
        assert_eq!(get_points_on_model(1, &instances, &SEEDS), Err(expected));
    }
}

#[test]
fn physical_backing_not_primitive_count_limits_the_final_triple() {
    // First triple twice-area 16, second 48. r=.5*64 selects second.
    // A count of four still reads all SIX physical words; offset 100 is ignored.
    let mesh = TestMesh {
        vertices: vec![
            [0., 0., 0.],
            [4., 0., 0.],
            [0., 0., 4.],
            [100., 0., 0.],
            [112., 0., 0.],
            [100., 0., 4.],
        ],
        indices: vec![0, 1, 2, 3, 4, 5],
        first: Some(100..104),
    };
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(
        get_points_on_model(1, &instances, &SEEDS).unwrap().unwrap()[0].position,
        [103., -1., 1.]
    );
    let short = TestMesh {
        indices: vec![0, 1, 2, 3, 4],
        ..mesh
    };
    let meshes: [&dyn Mesh; 1] = [&short];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(
        get_points_on_model(0, &instances, &SEEDS),
        Err(PlacementError::Unsupported(Unsupported::UnsafeIndex))
    );
}

#[test]
fn no_triangles_is_false_but_zero_requested_points_on_a_mesh_is_true() {
    assert_eq!(get_points_on_model(0, &[], &SEEDS).unwrap(), None);
    assert_eq!(get_points_on_model(1, &[ModelInstance::Other], &SEEDS).unwrap(), None);
    let mesh = TestMesh::triangle();
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(get_points_on_model(0, &instances, &SEEDS).unwrap(), Some(Vec::new()));
}

#[test]
fn published_box_count_height_and_single_top_centre() {
    let mut random = random();
    let mut placement = placement(&[], &mut random);
    let mut fire = FireFlames::<Effect>::new(1, 6, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    assert!(!fire.on_model());
    assert_eq!(fire.flames().len(), 8);
    assert_eq!(fire.flames()[0].position(), [6.0, 19.0, 10.0]);
    assert!(!fire.flames()[0].wall());
    assert_eq!(fire.flames().iter().filter(|flame| flame.wall()).count(), 7);
    for flame in &fire.flames()[1..] {
        let [x, y, z] = flame.position();
        assert!((0.0..=20.0).contains(&y));
        match flame.side() {
            0 => assert_eq!(x, 10.0),
            1 => assert_eq!(z, 4.0),
            2 => assert_eq!(x, 2.0),
            3 => assert_eq!(z, 16.0),
            _ => panic!("invalid side"),
        }
    }
}

#[test]
fn flora_overrides_top_and_side_branches_and_consumes_no_random_draws() {
    let mut random = random();
    let before = (random.lcg_state(), random.mul_state(), random.cached_gaussian());
    let mut placement = placement(&[], &mut random);
    placement.target_type = FLORA_TYPE;
    let mut fire = FireFlames::<Effect>::new(2, 3, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    assert_eq!(fire.flames().len(), 6);
    for flame in fire.flames() {
        assert_eq!(flame.position(), [6.0, 0.0, 10.0]);
        assert!(!flame.wall());
    }
    assert_eq!(
        (random.lcg_state(), random.mul_state(), random.cached_gaussian()),
        before
    );
}

#[test]
fn zero_counts_still_make_one_box_wall_and_short_box_clamps_top_to_min() {
    let mut random = random();
    let mut placement = placement(&[], &mut random);
    let mut fire = FireFlames::<Effect>::new(0, 0, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    assert_eq!(fire.flames().len(), 1);
    assert!(fire.flames()[0].wall());
    let mut short = FireFlames::<Effect>::new(1, 0, INITIAL);
    placement.bounds = [[0.0, 4.0, 0.0], [2.0, 4.5, 2.0]];
    short.create_flames(&mut placement).unwrap();
    assert_eq!(short.flames()[0].position()[1], 4.0);
}

#[test]
fn set_stage_is_idempotent_and_init_orders_base_group_before_effect_updates() {
    let mut random = random();
    let mut placement = placement(&[], &mut random);
    let mut fire = FireFlames::new(1, 0, INITIAL);
    let mut factory = Factory::default();
    fire.set_stage(stage(0), &mut placement, &mut factory).unwrap();
    assert_eq!(fire.stage(), Some(stage(0)));
    factory.trace.take();
    fire.set_stage(stage(0), &mut placement, &mut factory).unwrap();
    assert!(factory.trace.take().is_empty());
    fire.init(&mut Base(factory.trace.clone()), &mut factory).unwrap();
    let events = factory.trace.take();
    assert_eq!(
        &events[..5],
        &[
            Event::BaseInit,
            Event::Group(0x5005),
            Event::Stop(1),
            Event::Release(1),
            Event::Create("fire_1".into())
        ]
    );
    assert_eq!(
        events.iter().filter(|event| matches!(event, Event::Start(_))).count(),
        2
    );
}

#[test]
fn view_repositions_model_flames_without_recreation_and_hides_once() {
    let mesh = TestMesh::triangle();
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    let moved = [ModelInstance::MeshContainer {
        transform: Xform {
            translation: [100.0, 0.0, 0.0],
            ..Xform::default()
        },
        meshes: &meshes,
    }];
    let mut random = random();
    let before = (random.lcg_state(), random.mul_state());
    let mut placement = placement(&instances, &mut random);
    let mut factory = Factory::default();
    let mut fire = FireFlames::new(1, 1, INITIAL);
    fire.set_stage(stage(0), &mut placement, &mut factory).unwrap();
    assert!(fire.on_model());
    assert_eq!(fire.flames().len(), 2); // Model does not get the box's extra one.
    assert_eq!(fire.flames()[0].position(), [1.0, -1.0, 1.0]);
    factory.trace.take();
    placement.instances = &moved;
    placement.view_level = 2;
    fire.view_update(&mut placement).unwrap();
    assert_eq!(fire.flames()[0].position(), [101.0, -1.0, 1.0]);
    assert!(fire.flames().iter().all(|flame| !flame.visible()));
    let events = factory.trace.take();
    assert_eq!(
        events,
        vec![
            Event::Apply(1, temporary([1., -1., 1.])),
            Event::Apply(1, temporary([101., -1., 1.])),
            Event::Stop(1),
            Event::Apply(2, temporary([1., -1., 1.])),
            Event::Apply(2, temporary([101., -1., 1.])),
            Event::Stop(2)
        ]
    );
    placement.view_level = 1;
    fire.view_update(&mut placement).unwrap();
    assert!(fire.flames().iter().all(|flame| !flame.visible()));
    assert!(factory
        .trace
        .take()
        .iter()
        .all(|event| matches!(event, Event::Apply(_, _))));
    assert_eq!((random.lcg_state(), random.mul_state()), before);
}

#[test]
fn view_update_no_mesh_leaves_model_positions_and_box_view_is_noop() {
    let mesh = TestMesh::triangle();
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    let mut random = random();
    let mut placement = placement(&instances, &mut random);
    let mut fire = FireFlames::<Effect>::new(1, 0, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    placement.instances = &[];
    placement.view_level = 4;
    fire.view_update(&mut placement).unwrap();
    assert_eq!(fire.flames()[0].position(), [1.0, -1.0, 1.0]);
    assert!(fire.flames()[0].visible());
    let mut box_fire = FireFlames::<Effect>::new(1, 0, INITIAL);
    box_fire.create_flames(&mut placement).unwrap();
    let before = box_fire.flames()[0].position();
    placement.instances = &instances;
    box_fire.view_update(&mut placement).unwrap();
    assert_eq!(box_fire.flames()[0].position(), before);
    assert!(!box_fire.on_model());
}

#[test]
fn invalid_portable_composition_is_explicit_and_stage_store_precedes_create_failure() {
    let mut random = random();
    let mut placement = placement(&[], &mut random);
    let mut factory = Factory::default();
    let mut fire = FireFlames::new(u32::MAX, 1, INITIAL);
    assert_eq!(
        fire.set_stage(stage(2), &mut placement, &mut factory),
        Err(PlacementError::CountOverflow)
    );
    assert_eq!(fire.stage(), Some(stage(2)));
    assert!(factory.trace.take().is_empty());
    let mut fire = FireFlames::new(1, 0, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    assert_eq!(
        fire.create_flames(&mut placement),
        Err(PlacementError::AlreadyPopulated)
    );
    assert_eq!(
        fire.init(&mut Base(factory.trace.clone()), &mut factory),
        Err(PlacementError::UnpublishedInitialStage)
    );
    assert_eq!(factory.trace.take(), vec![Event::BaseInit, Event::Group(0x5005)]);
}

#[test]
fn getters_can_be_shared_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Flame<Effect>>();
    assert_send_sync::<FireFlames<Effect>>();
}

#[test]
fn area_weighting_across_instances_uses_local_area_not_scaled_area() {
    let small = TestMesh::triangle(); // twice-area 16
    let mut large = TestMesh::triangle();
    large.vertices[1][0] = 12.0; // twice-area 48
    let small_meshes: [&dyn Mesh; 1] = [&small];
    let large_meshes: [&dyn Mesh; 1] = [&large];
    let instances = [
        ModelInstance::MeshContainer {
            transform: Xform {
                scale: 100.0,
                ..Xform::default()
            },
            meshes: &small_meshes,
        },
        ModelInstance::MeshContainer {
            transform: Xform {
                translation: [100.0, 0.0, 0.0],
                ..Xform::default()
            },
            meshes: &large_meshes,
        },
    ];
    // r=.5*64=32 >16, so choose the second triangle despite the first's scale.
    let point = get_points_on_model(1, &instances, &SEEDS).unwrap().unwrap()[0];
    assert_eq!(point.position, [103.0, -1.0, 1.0]);
}

#[test]
fn each_model_placement_call_constructs_a_fresh_clock_seeded_generator() {
    use crate::rng::SeedSource;
    use std::sync::atomic::{AtomicU32, Ordering};
    struct Seeds(AtomicU32);
    impl SeedSource for Seeds {
        fn wall_clock_seed(&self) -> u32 {
            self.0.fetch_add(0x8000_0000, Ordering::SeqCst)
        }
        fn process_clock(&self) -> u32 {
            0
        }
    }
    let seeds = Seeds(AtomicU32::new(0));
    let mesh = TestMesh::triangle();
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(
        get_points_on_model(1, &instances, &seeds).unwrap().unwrap()[0].position,
        [1.0, -1.0, 1.0]
    );
    // Seed 0x80000000 produces u=v=0, so the selected point is vertex 2 + normal.
    assert_eq!(
        get_points_on_model(1, &instances, &seeds).unwrap().unwrap()[0].position,
        [0.0, -1.0, 4.0]
    );
}

#[test]
fn zero_area_is_not_filtered_and_nonfinite_area_is_an_explicit_error() {
    let mut mesh = TestMesh::triangle();
    mesh.vertices = vec![[0.0; 3]; 3];
    {
        let meshes: [&dyn Mesh; 1] = [&mesh];
        let instances = [ModelInstance::MeshContainer {
            transform: Xform::default(),
            meshes: &meshes,
        }];
        // FF3 DIFFERS: native masked exceptions produce NaNs and unordered
        // selection. That nonnormal domain is explicitly unimplemented here.
        assert_eq!(
            get_points_on_model(1, &instances, &SEEDS),
            Err(PlacementError::Unsupported(Unsupported::DegenerateNormal))
        );
    }
    mesh.vertices[0][0] = f32::NAN;
    let meshes: [&dyn Mesh; 1] = [&mesh];
    let instances = [ModelInstance::MeshContainer {
        transform: Xform::default(),
        meshes: &meshes,
    }];
    assert_eq!(
        get_points_on_model(1, &instances, &SEEDS),
        Err(PlacementError::Unsupported(Unsupported::Numeric))
    );
}

#[test]
fn multiple_top_flames_draw_gaussian_x_then_z_and_cache_is_reused() {
    // The existing RNG exposes its Gaussian cache. A cached -4 clamps to the
    // lower endpoint, independently pinning the first x draw before z.
    let mut random = random();
    random.set_state(1, 1, Some(-4.0));
    let mut placement = placement(&[], &mut random);
    let mut fire = FireFlames::<Effect>::new(2, 0, INITIAL);
    fire.create_flames(&mut placement).unwrap();
    assert_eq!(fire.flames()[0].position()[0], BOUNDS[0][0]);
    assert_eq!(fire.flames().iter().filter(|flame| !flame.wall()).count(), 2);
    assert_eq!(fire.flames()[0].position()[1], 19.0);
    assert_eq!(fire.flames()[1].position()[1], 19.0);
    assert!(fire.flames().iter().take(2).all(|flame| {
        let [x, _, z] = flame.position();
        (2.0..=10.0).contains(&x) && (4.0..=16.0).contains(&z)
    }));
}

#[test]
fn stage_change_updates_existing_placements_and_hidden_effects_stay_stopped() {
    let mut random = random();
    let mut placement = placement(&[], &mut random);
    placement.view_level = 2;
    let mut fire = FireFlames::new(1, 0, INITIAL);
    let mut factory = Factory::default();
    fire.set_stage(stage(0), &mut placement, &mut factory).unwrap();
    let positions: Vec<_> = fire.flames().iter().map(Flame::position).collect();
    let events = factory.trace.take();
    assert!(!events.iter().any(|event| matches!(event, Event::Start(_))));
    placement.bounds = [[100.0; 3], [200.0; 3]];
    fire.set_stage(stage(4), &mut placement, &mut factory).unwrap();
    assert_eq!(fire.flames().iter().map(Flame::position).collect::<Vec<_>>(), positions);
    let events = factory.trace.take();
    assert!(!events.iter().any(|event| matches!(event, Event::Start(_))));
    assert_eq!(
        events.iter().filter(|event| matches!(event, Event::Release(_))).count(),
        2
    );
    assert!(events.contains(&Event::Create("fire_5".into())));
    assert!(events.contains(&Event::Create("wfire_5".into())));
}

#[test]
fn init_empty_does_not_create_or_require_a_valid_initial_stage() {
    let mut fire = FireFlames::<Effect>::new(1, 6, INITIAL);
    let mut factory = Factory::default();
    fire.init(&mut Base(factory.trace.clone()), &mut factory).unwrap();
    assert!(fire.flames().is_empty());
    assert_eq!(fire.stage(), None);
    assert_eq!(factory.trace.take(), vec![Event::BaseInit, Event::Group(0x5005)]);
}
