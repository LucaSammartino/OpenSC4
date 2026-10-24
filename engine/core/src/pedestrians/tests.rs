// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{State, StateExtra};
use crate::stream::{ByteReader, ByteWriter};
use crate::vehicles::{PathAdvance, TrafficWorld};

#[derive(Default)]
struct Random {
    integers: Vec<(i32, i32)>,
    doubles: Vec<(f64, f64)>,
}
impl VehicleRandom for Random {
    fn signed_range(&mut self, a: i32, b: i32) -> i32 {
        self.integers.push((a, b));
        b
    }
    fn float_range(&mut self, a: f64, b: f64) -> f64 {
        self.doubles.push((a, b));
        (a + b) * 0.5
    }
    fn uniform(&mut self, n: u32) -> u32 {
        n - 1
    }
}

#[derive(Default)]
struct World {
    calls: Vec<String>,
    neighbors: Vec<Neighbor>,
    pushes: Vec<(u64, Vec3)>,
    queries: Vec<Bounds>,
    property: Vec<u8>,
    models: usize,
    fail_init: bool,
    attractor: Option<Vec3>,
    target: Option<Bounds>,
    path: Option<Vec<RouteStep>>,
    requested: Vec<Bounds>,
    road: Option<[i32; 2]>,
    headings: Vec<u8>,
    leader: Option<Vec3>,
}
impl TrafficWorld for World {
    fn network_at(&mut self, _: RouteStep) -> Option<RefValue> {
        Some(reference(1))
    }
    fn placement(&mut self, _: &RefValue, _: RouteStep) -> Option<(Vec3, Vec3)> {
        Some(([0.0; 3], [1.0, 0.0, 0.0]))
    }
    fn look_ahead(&mut self, _: RouteStep) -> Option<Vec3> {
        None
    }
    fn advance_path(&mut self, _: Vec3, _: f32, _: bool) -> Option<PathAdvance> {
        None
    }
    fn intersection(&mut self, _: &RefValue, _: Vec3) -> (u32, f32) {
        (0, 0.0)
    }
    fn collision_allows(&mut self, _: Vec3, _: Vec3, _: u32) -> bool {
        true
    }
    fn move_models(&mut self, _: Vec3, _: Vec3) {
        self.calls.push("move".into());
    }
    fn effects(&mut self, _: EffectAction) {}
    fn event(&mut self, _: u32, _: u32) {
        self.calls.push("event".into());
    }
    fn removed(&mut self) {
        self.calls.push("remove".into());
    }
}
impl VehicleWorld for World {
    fn automata_range(&mut self, a: f64, b: f64) -> f64 {
        (a + b) * 0.5
    }
    fn in_city(&mut self, _: Vec3) -> bool {
        true
    }
    fn heights(&mut self, _: Vec3) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn road_height(&mut self, _: Vec3) -> f32 {
        0.0
    }
    fn controller_point(&mut self) -> Vec3 {
        [0.0; 3]
    }
    fn controller_continues(&mut self, _: Vec3) -> bool {
        true
    }
    fn traffic_value(&mut self, _: Vec3) -> u32 {
        0
    }
    fn splash(&mut self, _: Vec3) -> Option<u64> {
        None
    }
    fn destroy_effect(&mut self, _: u64) {}
    fn release_blur(&mut self, _: u64) {}
    fn post_message(&mut self, _: u32) {}
    fn create_accident(&mut self, _: Vec3) {}
    fn crash_transform(&mut self) {}
    fn set_visible(&mut self, _: bool) {}
    fn notify_occupant(&mut self) {}
    fn refresh_behavior(&mut self) {
        self.calls.push("refresh".into());
    }
}
impl InitWorld for World {
    fn create_models(&mut self) -> bool {
        self.calls.push("models".into());
        !self.fail_init
    }
    fn init_effects(&mut self) {
        self.calls.push("effects".into());
    }
    fn init_occupant(&mut self) {
        self.calls.push("occupant".into());
    }
    fn refresh_behavior(&mut self) {
        self.calls.push("refresh".into());
    }
    fn enable_lights(&mut self, _: bool, _: u32) {}
    fn in_family(&mut self, _: u32) -> bool {
        false
    }
}
impl PedestrianWorld for World {
    fn nearby_automata(&mut self, kind: u32, bounds: Bounds) -> Vec<Neighbor> {
        assert_eq!(kind, 0x496E5783);
        self.queries.push(bounds);
        self.neighbors.clone()
    }
    fn pending_push(&mut self, id: u64, push: Vec3) {
        self.pushes.push((id, push));
    }
    fn ground_position(&mut self, mut p: Vec3, flags: u32) -> Vec3 {
        assert_eq!(flags, 0xC49);
        p[1] = 2.0;
        self.calls.push("ground".into());
        p
    }
    fn play_animation(&mut self, a: u32) {
        self.calls.push(format!("anim:{a}"));
    }
    fn stopped_animation(&mut self) {
        self.calls.push("stopped".into());
    }
    fn play_idle(&mut self) {
        self.calls.push("idle".into());
    }
    fn model_count(&mut self) -> usize {
        self.models
    }
    fn instance_value(&mut self, i: usize, v: f32) {
        self.calls.push(format!("instance:{i}:{v}"));
    }
    fn exemplar_play_count(&mut self, p: u32) -> Vec<u8> {
        assert_eq!(p, 0xEA958845);
        self.property.clone()
    }
    fn controller_attractor(&mut self, _: &RefValue) -> Option<Vec3> {
        self.attractor
    }
    fn controller_target(&mut self, _: &RefValue) -> Option<Bounds> {
        self.target
    }
    fn queue_step(&mut self, _: &RefValue, _: Vec3) -> Option<Vec3> {
        self.calls.push("queue".into());
        self.attractor
    }
    fn queue_head(&mut self, _: &RefValue) -> Option<Vec3> {
        self.attractor
    }
    fn find_path_to_box(&mut self, _: Vec3, b: Bounds) -> Option<Vec<RouteStep>> {
        self.requested.push(b);
        self.path.clone()
    }
    fn return_to_source(&mut self) {
        self.calls.push("source".into());
    }
    fn reaction_animations(&mut self, kind: u32) -> Vec<u32> {
        vec![kind + 100, kind + 200]
    }
    fn animation_finished(&mut self) -> bool {
        true
    }
    fn road_cell(&mut self, _: Vec3) -> Option<[i32; 2]> {
        self.road
    }
    fn walkable_heading(&mut self, _: [i32; 2], h: u8) -> bool {
        self.headings.contains(&h)
    }
    fn acquire_leader(&mut self, iid: u32) -> RefValue {
        assert_eq!(iid, IID_AUTOMATON);
        reference(9)
    }
    fn leader_position(&mut self, _: &RefValue) -> Option<Vec3> {
        self.leader
    }
    fn ignite_target(&mut self, c: &RefValue, cause: u32) {
        assert_eq!(cause, 2);
        assert!(crate::automata::refs_equal(c, &reference(3)));
        self.calls.push("ignite".into());
    }
}
fn reference(token: u64) -> RefValue {
    RefValue::Host {
        token,
        class_id: CLSID_PEDESTRIAN,
    }
}
fn pedestrian() -> Pedestrian {
    Pedestrian::construct(1, Tuning::default(), &mut Random::default())
}
fn state(owner: &mut Pedestrian, kind: u32) {
    owner.base_mut().states = vec![State::new(kind)];
}
fn movement() -> Movement {
    Movement {
        speed_scale: 1.0,
        max_speed: 1.0,
        leader_speed: None,
    }
}

#[test]
fn construction_and_first_parameter_conformance() {
    let mut random = Random::default();
    let mut p = Pedestrian::construct(1, Tuning::default(), &mut random);
    assert_eq!(p.base().occupant.occupant_type, 0x496E5783);
    assert_eq!(p.base().f1[0], 20.0 * SPEED_UNIT);
    assert_eq!(p.base().f3[1], p.base().f1[0]);
    assert_eq!(random.integers, vec![(15, 20)]);
    let mut world = World::default();
    assert_eq!(p.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(p.runtime.initial_parameter, -0.5);
    assert_eq!(random.doubles, vec![(-0.5, -0.5)]);
    assert_eq!(&world.calls[..4], &["models", "effects", "occupant", "refresh"]);
    p.init(Tuning::default(), &mut random, &mut world).unwrap();
    assert_eq!(random.doubles.len(), 1);
    let mut p = pedestrian();
    let tuning = Tuning {
        initial_parameter: (-1.0, 3.0),
        ..Tuning::default()
    };
    p.init(tuning, &mut random, &mut world).unwrap();
    assert_eq!(p.runtime.initial_parameter, 1.0);
    assert_eq!(random.doubles.last(), Some(&(-1.0, 3.0)));
}

#[test]
fn one_shot_init_success_failure_and_property_bounds() {
    let mut random = Random::default();
    let mut p = PedestrianOneShot::construct(1, Tuning::default(), &mut random);
    let mut world = World {
        models: 2,
        property: vec![4, 7, 255],
        ..World::default()
    };
    p.init(Tuning::default(), &mut random, &mut world).unwrap();
    assert_eq!(p.play_count, 7);
    assert!(world.calls.contains(&"instance:1:0.01".into()));
    assert_eq!(random.integers.last(), Some(&(4, 7)));
    p.init(Tuning::default(), &mut random, &mut world).unwrap();
    assert_eq!(random.doubles.len(), 1);
    assert_eq!(random.integers.len(), 3); // derived setup follows each successful base Init
    assert_eq!(p.consume_play_count(), Err(Unsupported::PlayCountConsumerPn1));
    let mut p = PedestrianOneShot::default();
    world.property = vec![1];
    p.init(Tuning::default(), &mut random, &mut world).unwrap();
    assert_eq!(p.play_count, 10);
    let mut p = PedestrianOneShot::default();
    world.fail_init = true;
    world.calls.clear();
    assert_eq!(p.init(Tuning::default(), &mut random, &mut world), Ok(false));
    assert!(!world.calls.iter().any(|s| s.starts_with("instance:")));
}

#[test]
fn enter_state_flag_table_and_empty_stack() {
    for (kind, on, off) in [
        (1, 0x26000, 0),
        (2, 0x2000, 0x4000),
        (3, 0x2000, 0x4000),
        (4, 0x2002, 0x4000),
        (6, 2, 0x2000),
        (8, 0x13012002, 0x4000),
        (9, 0x2002, 0x4000),
        (55, 0, 0x6000),
    ] {
        let mut p = pedestrian();
        let mut world = World {
            road: Some([0, 0]),
            headings: vec![0],
            ..World::default()
        };
        state(&mut p, kind);
        p.base_mut().automata_flags = 0x6000;
        p.enter_state(false, &mut world).unwrap();
        assert_eq!(p.base().automata_flags & on, on, "{kind}");
        assert_eq!(p.base().automata_flags & off, 0, "{kind}");
        if kind == 8 || kind == 9 {
            assert!(world.calls.contains(&format!("anim:{RIOT_ANIMATION}")));
        }
        if kind == 6 || kind == 55 {
            assert_eq!(world.requested.len(), 1);
        }
    }
    let mut p = pedestrian();
    let mut world = World::default();
    state(&mut p, 7);
    p.base_mut().automata_flags = 0xBEEF;
    p.enter_state(false, &mut world).unwrap();
    assert_eq!(p.base().automata_flags, 0xBEEF);
    p.base_mut().states.clear();
    p.enter_state(false, &mut world).unwrap();
    assert_eq!(p.base().automata_flags & 0x6000, 0);
    state(&mut p, 4);
    p.base_mut().automata_flags = 0;
    p.enter_state(true, &mut world).unwrap();
    assert_eq!(p.base().automata_flags, 0);
}

#[test]
fn collision_cadence_box_and_close_random_push() {
    let mut p = pedestrian();
    let mut w = World {
        neighbors: vec![
            Neighbor {
                id: 1,
                position: [0.0; 3],
            },
            Neighbor {
                id: 2,
                position: [0.005, 0.0, 0.0],
            },
            Neighbor {
                id: 3,
                position: [0.9, 0.0, 0.0],
            },
            Neighbor {
                id: 4,
                position: [1.0, 0.0, 0.0],
            },
        ],
        ..World::default()
    };
    assert!(!p.test_collision_expensive([0.0; 3], 499.0, &mut w));
    assert!(p.test_collision_expensive([0.0; 3], 1.0, &mut w));
    assert_eq!(
        w.queries,
        vec![Bounds {
            min: [-1.0, 0.0, -1.0],
            max: [1.0, 2.0, 1.0]
        }]
    );
    assert_eq!(w.pushes.len(), 2);
    assert!(w.pushes[0].1[0].abs() <= 0.5 && w.pushes[0].1[2].abs() <= 0.5);
    assert_eq!(w.pushes[0].1[1], 0.0);
    assert_eq!(w.pushes[1], (3, [0.9, 0.0, 0.0]));
    assert!(!p.test_collision_expensive([0.0; 3], 0.0, &mut w));
}

#[test]
fn collision_movement_rejection_retry_and_ground_flag() {
    let mut p = pedestrian();
    let mut w = World::default();
    p.base_mut().f3[..4].copy_from_slice(&[0.001, 0.001, 0.0, 0.0]);
    w.neighbors = vec![Neighbor {
        id: 2,
        position: [0.5, 2.0, 0.0],
    }];
    assert!(!p.move_forward_with_collision([2.0, 0.0, 0.0], 500.0, 1.0, &mut w));
    assert_eq!(p.base().vectors[0], [0.0; 3]);
    p.runtime.pending_push = Some([0.1, 0.0, 0.0]);
    assert!(p.move_forward_with_collision([2.0, 0.0, 0.0], 500.0, 1.0, &mut w));
    assert_eq!(p.base().vectors[0], [0.6, 2.0, 0.0]);
    assert_eq!(p.runtime.pending_push, None);
    w.calls.clear();
    p.base_mut().automata_flags |= 0x20000;
    p.move_forward_with_collision([0.0, 0.0, 1.0], 1.0, 2.0, &mut w);
    assert!(!w.calls.contains(&"ground".into()));
}

#[test]
fn bee_line_crowd_reaction_and_animation_transitions() {
    let mut p = pedestrian();
    let mut w = World::default();
    assert!(!p.crowd_reaction(true, 1, [4.0, 0.0, 0.0], 1.0, &mut w));
    assert!(p.crowd_reaction(false, 0, [4.0, 0.0, 0.0], 1.0, &mut w));
    assert_eq!(p.runtime.reaction_kind, 2);
    assert_eq!(p.base().vectors[2], [1.0, 0.0, 0.0]);
    state(&mut p, 3);
    w.attractor = Some([10.0, 0.0, 0.0]);
    p.runtime.pending_push = Some([0.25, 0.0, 0.0]);
    assert_eq!(p.state_tick(1.0, movement(), &mut w), Ok(true));
    assert_eq!(p.runtime.pending_push, None);
    assert!(p.base().vectors[0][0] > 0.25);
    assert!(w.calls.contains(&"anim:202".into()));
    state(&mut p, 2);
    p.base_mut().v2 = [1.0, 0.0, 0.0];
    assert!(p.bee_line_tick(1.0, 1.0, &mut w));
    assert!(w.calls.contains(&"refresh".into()));
    p.base_mut().automata_flags |= 0x20;
    p.animation_tick(1.0, movement(), true, false, &mut w).unwrap();
    assert!(w.calls.contains(&"anim:0".into()));
    w.calls.clear();
    p.animation_tick(1.0, movement(), false, false, &mut w).unwrap();
    assert!(!w.calls.contains(&"anim:0".into()));
}

#[test]
fn queue_strike_and_ignite_dispatch() {
    let mut p = pedestrian();
    let mut w = World {
        attractor: Some([20.0, 0.0, 0.0]),
        ..World::default()
    };
    state(&mut p, 4);
    p.base_mut().states[0].base.controller = reference(2);
    p.base_mut().automata_flags |= 0x20;
    p.state_tick(1.0, movement(), &mut w).unwrap();
    assert!(w.calls.contains(&"queue".into()) && w.calls.contains(&"idle".into()));
    p.base_mut().states[0].base.controller = RefValue::Null;
    p.queue_tick(1.0, 1.0, &mut w).unwrap();
    assert!(p.base().states.is_empty());
    state(&mut p, 6);
    w.target = Some(Bounds {
        min: [-32.0, 0.0, -32.0],
        max: [32.0, 0.0, 32.0],
    });
    p.strike_tick(&mut w).unwrap();
    assert!(w.calls.contains(&"source".into()));
    p.base_mut().vectors[0] = [50.0, 0.0, 0.0];
    w.calls.clear();
    p.strike_tick(&mut w).unwrap();
    assert_eq!(
        w.requested.last(),
        Some(&Bounds {
            min: [-16.0, 0.0, -16.0],
            max: [16.0, 0.0, 16.0]
        })
    );
    assert!(w.calls.contains(&"source".into()));
    w.calls.clear();
    w.path = Some(vec![[1, 2, 3], [2, 2, 3]]);
    p.strike_tick(&mut w).unwrap();
    assert!(p.runtime.traffic.has_path);
    assert!(!w.calls.contains(&"source".into()));
    p.base_mut().vectors[0] = [50.0, 0.0, 0.0];
    w.path = Some(Vec::new());
    p.strike_tick(&mut w).unwrap();
    assert!(w.calls.contains(&"source".into()));
    state(&mut p, 12);
    p.base_mut().states[0].base.controller = reference(3);
    p.ignite_tick(&mut w).unwrap();
    assert!(w.calls.contains(&"ignite".into()));
    assert_eq!(p.base().states.last().unwrap().state_type, 1);
}

#[test]
fn riot_leader_cell_memory_and_follower_distance() {
    let mut p = pedestrian();
    let mut w = World {
        road: Some([4, 5]),
        headings: vec![2],
        ..World::default()
    };
    state(&mut p, 8);
    p.riot_leader_tick(1.0, 1.0, true, &mut w);
    assert!(matches!(
        p.base().states[0].extra,
        StateExtra::RiotLeader { a: 4, b: 5, c: 2 }
    ));
    assert_eq!(p.base().vectors[2], [-1.0, 0.0, 0.0]);
    w.headings = vec![1];
    p.riot_leader_tick(1.0, 1.0, false, &mut w);
    assert_eq!(p.base().vectors[2], [-1.0, 0.0, 0.0]);
    w.road = Some([4, 6]);
    p.riot_leader_tick(1.0, 1.0, false, &mut w);
    assert_eq!(p.base().vectors[2], [0.0, 0.0, 1.0]);
    w.road = None;
    p.riot_leader_tick(1.0, 1.0, false, &mut w);
    assert!(w.calls.contains(&"remove".into()));
    let mut p = pedestrian();
    state(&mut p, 9);
    w.leader = Some([20.0, 0.0, 0.0]);
    p.riot_follower_tick(1.0, 1.0, &mut w);
    assert!(p.base().vectors[2][0] > 0.99);
    w.leader = Some(p.base().vectors[0]);
    p.riot_follower_tick(1.0, 1.0, &mut w);
    assert!(p.base().vectors[2][0] < -0.99);
}

#[test]
fn record_roundtrip_and_empty_inherited_tick() {
    let mut p = pedestrian();
    let mut w = World::default();
    p.record.flag = true;
    p.record.v = [3.0, 4.0, 5.0];
    p.record.f = 6.0;
    p.record.u = 7;
    let mut bytes = ByteWriter::new();
    assert!(p.write(&mut bytes));
    let mut read = Pedestrian::default();
    assert!(read.read(&mut ByteReader::new(bytes.bytes())));
    assert_eq!(read.record, p.record);
    assert_eq!(read.runtime, Runtime::default());
    assert_eq!(read.state_tick(1.0, movement(), &mut w), Ok(false));
    assert!(w.calls.is_empty());
    state(&mut read, 1);
    assert_eq!(read.state_tick(1.0, movement(), &mut w), Ok(true));
    assert!(w.calls.contains(&"remove".into()));
}

#[test]
fn record_version_and_one_shot_record_inheritance() {
    let mut p = pedestrian();
    p.record.version = 4;
    let mut output = ByteWriter::new();
    assert!(p.write(&mut output));
    let mut loaded = Pedestrian::default();
    assert!(!loaded.read(&mut ByteReader::new(output.bytes())));
    p.record.version = 5;
    let one = PedestrianOneShot {
        pedestrian: p,
        play_count: 9,
    };
    let mut output = ByteWriter::new();
    assert!(one.write(&mut output));
    let mut loaded = PedestrianOneShot::default();
    assert!(loaded.read(&mut ByteReader::new(output.bytes())));
    assert_eq!(loaded.pedestrian.record, one.pedestrian.record);
    assert_eq!(loaded.play_count, 0);
}
