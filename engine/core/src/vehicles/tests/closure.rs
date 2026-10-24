// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::stream::{InStream, OutStream};
use std::collections::VecDeque;

struct Draws {
    signed: VecDeque<i32>,
    ranges: VecDeque<f64>,
    uniform: u32,
    calls: Vec<String>,
}
impl Default for Draws {
    fn default() -> Self {
        Self {
            signed: [36, 72].into(),
            ranges: [0.02].into(),
            uniform: 2,
            calls: vec![],
        }
    }
}
impl VehicleRandom for Draws {
    fn signed_range(&mut self, lower: i32, upper: i32) -> i32 {
        self.calls.push(format!("signed:{lower}:{upper}"));
        self.signed.pop_front().unwrap()
    }
    fn float_range(&mut self, lower: f64, upper: f64) -> f64 {
        self.calls.push(format!("range:{lower}:{upper}"));
        self.ranges.pop_front().unwrap()
    }
    fn uniform(&mut self, bound: u32) -> u32 {
        self.calls.push(format!("uniform:{bound}"));
        self.uniform
    }
}
impl CollisionRandom for Draws {
    fn seed(&mut self, seed: u32) {
        self.calls.push(format!("seed:{seed}"));
    }
    fn range(&mut self, lower: f64, upper: f64) -> f64 {
        self.calls.push(format!("collision:{lower}:{upper}"));
        self.ranges.pop_front().unwrap()
    }
}

fn construction() -> Construction {
    Construction {
        vehicle_speed: (10, 40),
        train_speed: (50, 80),
    }
}
fn initialization() -> Initialization {
    Initialization {
        collision: true,
        city_lights: false,
        force_lights: false,
        train_speed: (50, 80),
        freight_count: (3, 8),
        other_count: (4, 9),
    }
}

impl InitWorld for World {
    fn create_models(&mut self) -> bool {
        self.calls.push("models".into());
        self.allow
    }
    fn init_effects(&mut self) {
        self.calls.push("init_effects".into());
    }
    fn init_occupant(&mut self) {
        self.calls.push("init_occupant".into());
    }
    fn refresh_behavior(&mut self) {
        self.calls.push("init_refresh".into());
    }
    fn enable_lights(&mut self, enabled: bool, mode: u32) {
        self.calls.push(format!("lights:{enabled}:{mode}"));
    }
    fn in_family(&mut self, family: u32) -> bool {
        self.calls.push(format!("family:{family:x}"));
        self.families.contains(&family)
    }
}

#[test]
fn construction_vectors_and_cumulative_draws() {
    let mut random = Draws::default();
    let v = Vehicle::construct(construction(), &mut random);
    assert_eq!(v.record.traffic.base.f1[0], 36.0 * SPEED_UNIT);
    assert_eq!(tween_value(&v.record.traffic.base.f3[..4]), 36.0 * SPEED_UNIT);
    assert_eq!(v.blur_models, [None; 4]);
    assert_eq!(v.blur_index, 0);
    assert_eq!(v.runtime_vectors[..2], [UNSET_POINT; 2]);
    assert_eq!(v.sound_identity, 0xC7CC75F1);
    assert_eq!(v.traffic.path_word, 0xffff);
    assert_eq!(v.traffic.travel_type, 7);
    assert_ne!(v.record.traffic.base.occupant.flags & 0x10, 0);
    let mut random = Draws::default();
    let car = TrainCar::construct(construction(), &mut random);
    assert_eq!(random.calls, ["signed:10:40", "signed:50:80"]);
    assert_eq!(car.vehicle.record.traffic.base.f1[0], 72.0 * SPEED_UNIT);
    assert_eq!(car.rear_point, UNSET_POINT);
    assert!(!car.classification);
    let train = Train::construct(construction(), &mut Draws::default());
    assert_eq!(train.cached_cell, [-1; 3]);
    assert!(train.network.is_null());
    assert_eq!(train.target_count, 0);
}

#[test]
fn initialization_failure_loaded_and_already_initialized_vectors() {
    let mut a = TrafficAutomaton::construct();
    let mut w = World {
        allow: false,
        ..World::default()
    };
    assert_eq!(a.init(&mut w), Ok(false));
    assert!(a.runtime.initialized);
    assert_eq!(w.calls, ["models", "init_occupant"]);
    w.calls.clear();
    assert_eq!(a.init(&mut w), Ok(true));
    assert!(w.calls.is_empty());
    a.runtime.initialized = false;
    a.record.base.occupant.status = 2;
    w.allow = true;
    assert_eq!(a.init(&mut w), Ok(true));
    assert_eq!(w.calls, ["models", "init_effects", "init_occupant"]);
    let mut car = TrainCar::default();
    let mut w = World {
        allow: false,
        families: vec![0x4208],
        ..World::default()
    };
    assert_eq!(car.init(initialization(), &mut w), Err(Unsupported::InitVh10));
    assert_eq!(car.vehicle.record.traffic.base.automata_flags & 0x10014000, 0x10014000);
    assert!(car.classification);
    assert!(car.vehicle.traffic.initialized);
    let mut train = Train::default();
    let mut w = World {
        families: vec![0x4216, 0x4215],
        ..World::default()
    };
    let mut r = Draws::default();
    assert_eq!(train.init(initialization(), &mut r, &mut w), Err(Unsupported::InitVh10));
    assert_eq!(train.car_family, 0x4203);
    assert_eq!(train.target_count, 5);
    assert!(train.cars.is_empty());
    assert_eq!(r.calls.last().unwrap(), "uniform:5");
    assert_eq!(train.vehicle.record.traffic.base.automata_flags & 0x10000000, 0);
    let n = r.calls.len();
    let m = w.calls.len();
    assert_eq!(train.init(initialization(), &mut r, &mut w), Err(Unsupported::InitVh10));
    assert_eq!((r.calls.len(), w.calls.len()), (n, m));
}

#[test]
fn init_open_contract_is_explicit_and_survives_repeat_until_shutdown() {
    let mut a = TrafficAutomaton::construct();
    let mut w = World::default();
    assert_eq!(a.init(&mut w), Err(Unsupported::InitVh10));
    assert_eq!(w.calls, ["models", "init_effects", "init_occupant", "init_refresh"]);
    w.calls.clear();
    assert_eq!(a.init(&mut w), Err(Unsupported::InitVh10));
    assert!(w.calls.is_empty());
    a.owner().shutdown(&mut w);
    a.record.base.automata_flags |= STRAIGHT;
    assert_eq!(a.init(&mut w), Ok(true));

    let mut v = Vehicle::default();
    v.record.traffic.base.occupant.status = 2;
    let mut tuning = initialization();
    tuning.force_lights = true;
    w.calls.clear();
    assert_eq!(v.init(tuning, &mut w), Err(Unsupported::InitVh10));
    assert_eq!(w.calls, ["models", "init_effects", "init_occupant", "lights:true:0"]);
    w.calls.clear();
    assert_eq!(v.init(tuning, &mut w), Err(Unsupported::InitVh10));
    assert!(w.calls.is_empty());
}

#[test]
fn shutdown_releases_blur_and_splash_once_and_live_blur_blocks_read() {
    let mut v = Vehicle {
        blur_models: [Some(10), None, Some(12), Some(13)],
        splash_effect: Some(42),
        ..Vehicle::default()
    };
    let mut out = ByteWriter::new();
    assert!(Vehicle::default().write(&mut out));
    let bytes = out.into_bytes();
    let mut input = ByteReader::new(&bytes);
    assert!(!v.read(&mut input));
    assert_eq!(input.position(), 0);
    v.splash_effect = None;
    assert!(!v.read(&mut input));
    assert_eq!(input.position(), 0);
    v.splash_effect = Some(42);
    let mut w = World::default();
    v.shutdown(&mut w);
    v.shutdown(&mut w);
    for model in [10, 12, 13] {
        assert_eq!(
            w.calls
                .iter()
                .filter(|s| **s == format!("blur_release:{model}"))
                .count(),
            1
        );
    }
    assert_eq!(w.calls.iter().filter(|s| *s == "destroy:42").count(), 1);
    assert!(v.read(&mut input));
}

#[test]
fn train_runtime_identity_is_saved_and_unresolved_collision_is_explicit() {
    let mut train = Train {
        cars: vec![TrainCarSlot::owned(host(10, CLSID_TRAIN_CAR), TrainCar::default())],
        ..Train::default()
    };
    let registry = std::rc::Rc::new(std::cell::RefCell::new(crate::stream::SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(registry);
    assert!(train.write(&mut out));
    let bytes = out.into_bytes();
    let mut loaded = Train::default();
    struct Cars;
    impl crate::stream::RefResolver for Cars {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            Some(host(u64::from(object_id) * 10, class_id))
        }
    }
    assert!(loaded.read(&mut ByteReader::with_resolver(&bytes, &mut Cars)));
    assert_eq!(loaded.cars.len(), 1);
    assert!(!loaded.cars[0].reference.is_null());
    assert!(loaded.cars[0].runtime.is_none());
    let mut w = World::default();
    let mut random = Draws::default();
    assert_eq!(
        loaded.handle_collision(&mut random, &mut w),
        Err(Unsupported::TrainCompositionVh5)
    );
    assert!(random.calls.is_empty());
    assert!(w.calls.is_empty());
    assert!(loaded.vehicle.record.traffic.base.states.is_empty());
    train.cars[0].reference = RefValue::Null;
    let mut out = ByteWriter::new();
    assert!(!train.write(&mut out));
    assert!(out.into_bytes().is_empty());
    train.cars[0].runtime.as_mut().unwrap().vehicle.blur_models[0] = Some(99);
    train.shutdown(&mut w);
    train.shutdown(&mut w);
    assert!(train.cars.is_empty());
    assert_eq!(w.calls.iter().filter(|s| *s == "blur_release:99").count(), 1);
}

fn host(token: u64, class_id: u32) -> RefValue {
    RefValue::Host { token, class_id }
}

#[test]
fn train_wire_order_version_failure_and_append_null_quirks() {
    let mut car = TrainCar::default();
    let mut bad = ByteReader::new(&[1, 0, 99, 0]);
    assert!(!car.read(&mut bad));
    assert_eq!(bad.get_u16(), Some(99));
    let mut train = Train::default();
    let mut bad = ByteReader::new(&[2, 0, 3, 0, 99, 0]);
    assert!(!train.read(&mut bad));
    assert_eq!(bad.get_u16(), Some(99));
    let mut prefix = ByteWriter::new();
    prefix.put_u16(2);
    prefix.put_u16(2);
    prefix.put_u16(99);
    prefix.put_u32(0xabcdef01);
    let bytes = prefix.into_bytes();
    let mut bad = ByteReader::new(&bytes);
    assert!(!train.read(&mut bad));
    assert_eq!(bad.position(), 6);
    let source = Train {
        cars: vec![host(10, CLSID_TRAIN_CAR), RefValue::Null, host(20, CLSID_TRAIN_CAR)]
            .into_iter()
            .map(TrainCarSlot::reference)
            .collect(),
        raw_2c0: 0xfeedbeef,
        vector_2d4: [1.25, -2.5, 3.75],
        raw_2e4: 0xff800001,
        ..Train::default()
    };
    let registry = std::rc::Rc::new(std::cell::RefCell::new(crate::stream::SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(registry);
    assert!(source.write(&mut out));
    let bytes = out.into_bytes();
    assert_eq!(&bytes[..4], &[2, 0, 2, 0]);
    let mut target = Train {
        cars: vec![TrainCarSlot::reference(host(30, CLSID_TRAIN_CAR))],
        network: host(99, 0xA821EF94),
        ..Train::default()
    };
    struct Cars;
    impl crate::stream::RefResolver for Cars {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            Some(host(u64::from(object_id) * 10, class_id))
        }
    }
    assert!(target.read(&mut ByteReader::with_resolver(&bytes, &mut Cars)));
    assert_eq!(target.cars.len(), 3);
    assert!(crate::automata::refs_equal(
        &target.cars[0].reference,
        &host(30, CLSID_TRAIN_CAR)
    ));
    assert!(crate::automata::refs_equal(
        &target.cars[1].reference,
        &host(10, CLSID_TRAIN_CAR)
    ));
    assert!(crate::automata::refs_equal(
        &target.cars[2].reference,
        &host(20, CLSID_TRAIN_CAR)
    ));
    assert!(target.network.is_null());
    assert_eq!(target.raw_2c0, 0xfeedbeef);
    assert_eq!(target.vector_2d4, [1.25, -2.5, 3.75]);
    assert_eq!(target.raw_2e4, 0xff800001);
    let empty = Train::default();
    let mut out = ByteWriter::new();
    assert!(empty.write(&mut out));
    let bytes = out.into_bytes();
    let mut input = ByteReader::new(&bytes);
    assert_eq!(input.get_u16(), Some(2));
    assert_eq!(input.get_u16(), Some(2));
    assert!(crate::automata::VehicleRecord::read(&mut input).is_some());
    assert_eq!(input.get_u32(), Some(0));
    assert_eq!(input.get_u32(), Some(0));
    assert_eq!([input.get_f32(), input.get_f32(), input.get_f32()], [Some(0.0); 3]);
    assert!(input.get_ref().unwrap().is_null());
    assert_eq!(input.get_u32(), Some(0));
    assert!(input.get_u8().is_none());
}

#[test]
fn plummet_entry_splash_draw_and_loaded_pull_vectors() {
    let mut v = Vehicle::default();
    let mut w = World {
        range_result: Some(1.75),
        ..World::default()
    };
    v.record.traffic.base.vectors[2] = [1.0, 0.25, 0.0];
    assert_eq!(v.set_state(14, SetStateArgs::default(), &mut w), Ok(true));
    assert_eq!(
        v.record.traffic.base.states[0].extra,
        StateExtra::T0E {
            descent: f32::from_bits(0xbb88888a),
            roll_rate: 0.5,
            roll_centre: 0.25,
            tumble_ms: 1750.0,
            post_splash_ms: 0.0
        }
    );
    v.record.traffic.base.states[0].extra = StateExtra::T0E {
        descent: -0.01,
        roll_rate: 1.0,
        roll_centre: 2.0,
        tumble_ms: 0.0,
        post_splash_ms: 0.0,
    };
    v.record.traffic.base.vectors[0] = [0.0, 2.0, 0.0];
    w.water = 3.0;
    w.range_result = Some(0.25);
    w.calls.clear();
    v.plummet_tick(1.0, &mut w).unwrap();
    assert_eq!(v.record.traffic.base.f3[6], 0.25);
    match v.record.traffic.base.states[0].extra {
        StateExtra::T0E {
            descent,
            post_splash_ms,
            ..
        } => {
            assert_eq!(descent.to_bits(), 0x3991a2b5);
            assert_eq!(post_splash_ms, 250.0);
        }
        _ => panic!(),
    }
    assert_eq!(w.calls.iter().filter(|s| s.starts_with("rng:")).count(), 1);
    let mut source = Vehicle::default();
    source.record.traffic.base.states = vec![State::new(21)];
    source.record.traffic.base.states[0].extra = StateExtra::T15(250.0);
    let mut out = ByteWriter::new();
    assert!(source.write(&mut out));
    let mut target = Vehicle {
        pull_heading: [0.0, 0.0, 1.0],
        pull_sideways: [1.0, 0.0, 0.0],
        ..Vehicle::default()
    };
    assert!(target.read(&mut ByteReader::new(&out.into_bytes())));
    assert_eq!(target.pull_heading, [0.0, 0.0, 1.0]);
    assert_eq!(target.pull_sideways, [1.0, 0.0, 0.0]);
}

#[test]
fn timeout_strict_boundary_controller_and_hop_floor() {
    let mut v = Vehicle::default();
    let mut w = World::default();
    v.record.traffic.base.states = vec![State::new(10)];
    v.record.traffic.base.states[0].base.countdown_ms = 20.0;
    v.state_tick(20.0, movement(), &mut w).unwrap();
    v.state_tick(1.0, movement(), &mut w).unwrap();
    assert_eq!(v.record.traffic.base.states[0].base.countdown_ms, 0.0);
    let mut exposed = State::new(15);
    exposed.extra = StateExtra::T0F([10.0, -0.01]);
    exposed.base.controller = host(3, 0x6A1857F8);
    let mut expiring = State::new(10);
    expiring.base.countdown_ms = 20.0;
    v.record.traffic.base.states = vec![exposed, expiring];
    v.record.traffic.base.vectors[0] = [2.0, 10.1, 3.0];
    w.controller_handled = true;
    v.state_tick(21.0, movement(), &mut w).unwrap();
    assert_eq!(v.record.traffic.base.vectors[0], [2.0, 10.1, 3.0]);
    assert!(w.calls.contains(&"controller:15:21".into()));
    w.controller_handled = false;
    v.state_tick(20.0, movement(), &mut w).unwrap();
    assert_eq!(v.record.traffic.base.vectors[0], [2.0, 10.0, 3.0]);
    assert_eq!(v.record.traffic.base.states[0].extra, StateExtra::T0F([10.0, -0.01]));
    v.record.traffic.base.states[0].extra = StateExtra::T0F([10.0, 0.0]);
    v.state_tick(20.0, movement(), &mut w).unwrap();
    match v.record.traffic.base.states[0].extra {
        StateExtra::T0F([_, velocity]) => assert!((velocity + 0.0011111113).abs() < 1e-9),
        _ => panic!(),
    }
}

#[test]
fn attach_animation_vectors_and_failure_keeps_state() {
    for selector in [1, 2, 3] {
        let mut v = Vehicle::default();
        let mut w = World {
            animation: Some((0.8, 4.0, 5.0)),
            ..World::default()
        };
        assert_eq!(
            v.attach_anim(Default::default(), selector, false, 0.0, &mut w),
            Ok(true)
        );
        assert_eq!(v.attach_tick(200.0, &mut w), selector == 1);
        assert_eq!(v.record.traffic.base.states.is_empty(), selector == 1);
        assert!(w.calls.contains(&"animation:1:1".into()));
    }
    for handled in [false, true] {
        let mut v = Vehicle::default();
        let mut w = World {
            allow: false,
            ..World::default()
        };
        assert_eq!(v.attach_anim(Default::default(), 1, handled, 0.0, &mut w), Ok(false));
        assert_eq!(v.attach_tick(200.0, &mut w), handled);
        assert_eq!(v.record.traffic.base.states.len(), 1);
    }
}

impl DamageWorld for World {
    fn accident(&mut self, _: Vec3, kind: u32, duration: u32) -> Option<u64> {
        self.calls.push(format!("accident:{kind}:{duration}"));
        Some(99)
    }
    fn request_effect(&mut self, name: &'static str) -> Option<u64> {
        self.calls.push(format!("effect:{name}"));
        self.allow.then_some(42)
    }
    fn stop_effect(&mut self, effect: u64, argument: u32) {
        self.calls.push(format!("stop:{effect}:{argument}"));
    }
    fn release_effect(&mut self, effect: u64) {
        self.calls.push(format!("release:{effect}"));
    }
    fn transform_effect(&mut self, _: u64, _: Vec3, _: Vec3, scale: Vec3) {
        assert_eq!(scale, [1.0; 3]);
        self.calls.push("effect_transform".into());
    }
    fn start_effect(&mut self, effect: u64, argument: u32) {
        self.calls.push(format!("start:{effect}:{argument}"));
    }
    fn blur_model(&mut self, _: crate::stream::ResKey) -> Option<u64> {
        self.calls.push("blur_request".into());
        self.allow.then_some(77)
    }
    fn place_blur(&mut self, model: u64, _: Vec3, _: Vec3, new: bool) {
        self.calls.push(format!("blur_place:{model}:{new}"));
    }
    fn blur_alpha(&mut self, model: u64, alpha: f32) {
        self.calls.push(format!("alpha:{model}:{alpha}"));
    }
}

#[test]
fn collision_message_direction_final_and_train_early_success() {
    let mut v = moving();
    let mut w = World::default();
    let mut r = Draws {
        ranges: [0.0, 1.0, 0.0].into(),
        ..Draws::default()
    };
    assert_eq!(v.handle_collision(false, &mut r, &mut w), Ok(true));
    assert!(w.calls.is_empty());
    assert!(r.calls.is_empty());
    assert_eq!(v.handle_collision(true, &mut r, &mut w), Ok(true));
    assert!((f64::from(v.record.traffic.base.vectors[2][0]) - 0.89442719).abs() < 1e-7);
    assert!((f64::from(v.record.traffic.base.vectors[2][1]) - 0.44721360).abs() < 1e-7);
    assert_eq!(v.record.traffic.base.states[0].base.countdown_ms, 5000.0);
    let draws = r.calls.len();
    v.handle_collision(true, &mut r, &mut w).unwrap();
    assert_eq!(r.calls.len(), draws);
    assert_eq!(
        w.calls
            .iter()
            .filter(|s| *s == &format!("message:{COLLISION_MESSAGE}"))
            .count(),
        2
    );
    let mut train = Train {
        cars: vec![
            TrainCarSlot::owned(
                host(10, CLSID_TRAIN_CAR),
                TrainCar {
                    vehicle: moving(),
                    ..TrainCar::default()
                }
            );
            2
        ],
        vehicle: moving(),
        ..Train::default()
    };
    train.vehicle.measured_speed = 0.75;
    let mut r = Draws {
        ranges: [0.0, 1.0, 0.0, 0.0, 1.0, 0.0].into(),
        ..Draws::default()
    };
    train.handle_collision(&mut r, &mut w).unwrap();
    assert_eq!(train.vehicle.record.traffic.base.vectors[2], [1.0, 0.0, 0.0]);
    assert_eq!(tween_value(&train.vehicle.record.traffic.base.f3[..4]), 0.0);
    for c in &train.cars {
        let c = c.runtime.as_ref().unwrap();
        assert_eq!(c.vehicle.measured_speed, 0.75);
        assert_eq!(c.vehicle.record.traffic.base.states[0].base.countdown_ms, 20000.0);
    }
    train.vehicle.record.traffic.base.f3[..4].copy_from_slice(&[0.02, 0.02, 0.0, 0.0]);
    train.cars[1].runtime.as_mut().unwrap().vehicle.measured_speed = 3.0;
    let mut r = Draws {
        ranges: [0.0, 1.0, 0.0].into(),
        ..Draws::default()
    };
    assert_eq!(train.handle_collision(&mut r, &mut w), Ok(true));
    assert_eq!(train.cars[1].runtime.as_ref().unwrap().vehicle.measured_speed, 3.0);
    assert_eq!(tween_value(&train.vehicle.record.traffic.base.f3[..4]), 0.02);
}

#[test]
fn damage_thresholds_effect_ownership_repetition_and_speed() {
    for (damage, expected) in [
        (49, None),
        (50, Some("udriveit_injured")),
        (99, Some("udriveit_injured")),
        (100, Some("udriveit_crash")),
    ] {
        let mut v = moving();
        let mut w = World::default();
        let mut s = DriveDamage {
            damage,
            effect: Some(7),
            ..DriveDamage::default()
        };
        v.update_after_damage(&mut s, 100, &mut w);
        assert_eq!(
            w.calls.iter().find(|s| s.starts_with("effect:")).map(String::as_str),
            expected.map(|name| if name == "udriveit_crash" {
                "effect:udriveit_crash"
            } else {
                "effect:udriveit_injured"
            })
        );
        assert_eq!(w.calls.contains(&"accident:2:30000".into()), damage == 100);
        if expected.is_some() {
            assert!(w.calls.contains(&"stop:7:0".into()));
            assert!(w.calls.contains(&"release:7".into()));
        }
    }
    let mut v = moving();
    let mut w = World {
        allow: false,
        ..World::default()
    };
    let mut s = DriveDamage {
        damage: 100,
        effect: Some(7),
        ..DriveDamage::default()
    };
    v.update_after_damage(&mut s, 100, &mut w);
    v.update_after_damage(&mut s, 100, &mut w);
    assert_eq!(s.effect, Some(7));
    assert_eq!(w.calls.iter().filter(|s| s.as_str() == "accident:2:30000").count(), 2);
    assert_eq!(w.calls.iter().filter(|s| s.as_str() == "event:11:1").count(), 2);
    v.record.traffic.base.f3[..4].copy_from_slice(&[0.02, 0.02, 0.0, 0.0]);
    v.resolve_collision([-0.01, 0.0, 0.0], 1.0, 1.0, 1000, &mut s, &mut w);
    assert!((s.raw_speed - 0.01).abs() < 1e-9);
    s.damage = 100;
    v.resolve_collision([0.0; 3], 3.0, 1.0, 1000, &mut s, &mut w);
    assert_eq!(s.damage, 80);
}

#[test]
fn blur_ring_alpha_negative_failed_request_and_release() {
    let mut v = moving();
    let mut w = World::default();
    v.blur_models = [Some(10), Some(11), Some(12), Some(13)];
    v.blur_index = 2;
    v.record.traffic.base.f3[4..].copy_from_slice(&[1.0, 1.0, 0.0, 0.0]);
    v.update_motion_blur(&mut w);
    let samples: Vec<_> = w.calls.iter().filter(|s| s.starts_with("alpha:")).cloned().collect();
    assert_eq!(samples.len(), 4);
    assert!(samples[0].starts_with("alpha:12:1"));
    assert!(samples[1].starts_with("alpha:11:0.666"));
    assert!(samples[2].starts_with("alpha:10:0.333"));
    assert_eq!(samples[3], "alpha:13:0");
    assert_eq!(v.blur_index, 3);
    v.blur_models[3] = None;
    v.record.traffic.base.f3[4..].copy_from_slice(&[-0.25, -0.25, 0.0, 0.0]);
    w.allow = false;
    w.calls.clear();
    v.update_motion_blur(&mut w);
    assert_eq!(v.blur_index, 0);
    assert_eq!(v.blur_models[3], None);
    assert_eq!(w.calls.iter().filter(|s| s.starts_with("alpha:")).count(), 3);
    assert!(w
        .calls
        .iter()
        .filter(|s| s.starts_with("alpha:"))
        .all(|s| s.ends_with(":0")));
    v.release_motion_blur(&mut w);
    v.release_motion_blur(&mut w);
    assert_eq!(w.calls.iter().filter(|s| s.starts_with("blur_release:")).count(), 3);
}
