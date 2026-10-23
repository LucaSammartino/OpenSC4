// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::automata::{SetStateArgs, State, StateExtra, Vec3};
use crate::stream::{ByteReader, ByteWriter, RefValue};

struct World {
    calls: Vec<String>,
    intersection: (u32, f32),
    advance: Option<PathAdvance>,
    allow: bool,
    inside: bool,
    terrain: f32,
    water: f32,
    traffic: u32,
    controller: bool,
    fail_look_ahead: Option<RouteStep>,
}
impl Default for World {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            intersection: (0, 0.0),
            advance: None,
            allow: true,
            inside: true,
            terrain: 0.0,
            water: 0.0,
            traffic: 0,
            controller: true,
            fail_look_ahead: None,
        }
    }
}
impl TrafficWorld for World {
    fn network_at(&mut self, _: RouteStep) -> Option<RefValue> {
        Some(RefValue::Host {
            token: 1,
            class_id: 0xC9C05C6E,
        })
    }
    fn placement(&mut self, _: &RefValue, _: RouteStep) -> Option<(Vec3, Vec3)> {
        Some(([1.0, 0.0, 1.0], [1.0, 0.0, 0.0]))
    }
    fn look_ahead(&mut self, step: RouteStep) -> Option<Vec3> {
        if Some(step) == self.fail_look_ahead {
            None
        } else {
            Some(step.map(f32::from))
        }
    }
    fn advance_path(&mut self, _: Vec3, step: f32, vehicle: bool) -> Option<PathAdvance> {
        self.calls.push(format!("path:{step}:{vehicle}"));
        self.advance
    }
    fn intersection(&mut self, _: &RefValue, _: Vec3) -> (u32, f32) {
        self.calls.push("intersection".into());
        self.intersection
    }
    fn collision_allows(&mut self, _: Vec3, _: Vec3, _: u32) -> bool {
        self.allow
    }
    fn move_models(&mut self, _: Vec3, _: Vec3) {
        self.calls.push("move".into());
    }
    fn effects(&mut self, action: EffectAction) {
        self.calls.push(format!("effect:{action:?}"));
    }
    fn event(&mut self, id: u32, value: u32) {
        self.calls.push(format!("event:{id}:{value}"));
    }
    fn removed(&mut self) {
        self.calls.push("remove".into());
    }
}
impl VehicleWorld for World {
    fn in_city(&mut self, _: Vec3) -> bool {
        self.inside
    }
    fn heights(&mut self, _: Vec3) -> (f32, f32) {
        (self.terrain, self.water)
    }
    fn road_height(&mut self, _: Vec3) -> f32 {
        self.terrain
    }
    fn controller_point(&mut self) -> Vec3 {
        [0.0; 3]
    }
    fn controller_continues(&mut self, _: Vec3) -> bool {
        self.controller
    }
    fn traffic_value(&mut self, _: Vec3) -> u32 {
        self.traffic
    }
    fn splash(&mut self, _: Vec3) -> Option<u64> {
        self.calls.push("splash".into());
        Some(42)
    }
    fn destroy_effect(&mut self, effect: u64) {
        self.calls.push(format!("destroy:{effect}"));
    }
    fn post_message(&mut self, id: u32) {
        self.calls.push(format!("message:{id}"));
    }
    fn create_accident(&mut self, _: Vec3) {
        self.calls.push("accident".into());
    }
    fn crash_transform(&mut self) {
        self.calls.push("crash".into());
    }
    fn set_visible(&mut self, _: bool) {
        self.calls.push("visible".into());
    }
    fn notify_occupant(&mut self) {
        self.calls.push("occupant".into());
    }
    fn refresh_behavior(&mut self) {
        self.calls.push("refresh".into());
    }
}
fn movement() -> Movement {
    Movement {
        speed_scale: 1.0,
        max_speed: 10.0,
        leader_speed: None,
    }
}
fn moving() -> Vehicle {
    let mut v = Vehicle::default();
    v.record.traffic.base.f3[..4].copy_from_slice(&[0.01, 0.01, 0.0, 0.0]);
    v.record.traffic.base.f1 = [2.0, 0.02];
    v.record.traffic.base.vectors[1] = [100.0, 0.0, 0.0];
    v.record.traffic.base.vectors[2] = [1.0, 0.0, 0.0];
    v
}

#[test]
fn tween_seconds_and_straight_arrival_without_snap() {
    let mut v = moving();
    let mut w = World::default();
    tween_speed(&mut v.record.traffic.base, 0.03, 1.0);
    advance_tweens(&mut v.record.traffic.base, 500.0);
    assert!((tween_value(&v.record.traffic.base.f3[..4]) - 0.02).abs() < 1e-6);
    advance_tweens(&mut v.record.traffic.base, 500.0);
    assert_eq!(tween_value(&v.record.traffic.base.f3[..4]), 0.03);
    v.record.traffic.base.vectors[1] = [1.0, 50.0, 0.0];
    v.traffic_owner().straight_tick(100.0, 1.0, &mut w);
    assert_eq!(v.record.traffic.base.vectors[0], [0.0; 3]);
    assert_eq!(v.record.traffic.base.vectors[1][1], 0.0);
    assert_ne!(v.record.traffic.base.automata_flags & REACHED, 0);
}

#[test]
fn route_consumes_first_and_stops_failed_lookahead() {
    let mut a = TrafficAutomaton::default();
    let mut w = World::default();
    a.owner().set_route_info(&[[1, 2, 3], [4, 5, 6], [7, 8, 9]], 1.0);
    a.owner().begin_route(&mut w).unwrap();
    assert_eq!(a.record.route.len(), 2);
    w.fail_look_ahead = Some([7, 8, 9]);
    a.owner().fill_look_ahead(5, &mut w);
    assert_eq!(a.runtime.look_ahead.len(), 1);
    a.owner().set_route_info(&[[1, 2, 3]], 1.0);
    a.owner().begin_route(&mut w).unwrap();
    assert!(a.runtime.look_ahead.is_empty());
    assert_ne!(a.record.base.automata_flags & REACHED, 0);
}

#[test]
fn blocked_wait_obeys_stopped_flag_and_intersection() {
    let mut a = TrafficAutomaton::default();
    let mut w = World::default();
    a.record.base.push_state(
        13,
        SetStateArgs {
            timeout_ms: 100.0,
            ..SetStateArgs::default()
        },
    );
    assert!(!a.owner().blocked_wait_tick(50.0, 1.0, &mut w));
    assert_eq!(a.record.base.states[0].base.countdown_ms, 100.0);
    a.record.base.automata_flags = STOPPED | BLOCKED;
    assert!(a.owner().blocked_wait_tick(50.0, 1.0, &mut w));
    w.intersection.0 = 2;
    assert!(!a.owner().blocked_wait_tick(50.0, 1.0, &mut w));
    assert_eq!(a.record.base.states.len(), 1);
    assert_eq!(a.record.base.automata_flags & (STOPPED | BLOCKED), 0);
    w.intersection.0 = 0;
    a.owner().blocked_wait_tick(1.0, 1.0, &mut w);
    assert!(a.record.base.states.is_empty());
}

#[test]
fn collision_flags_wait_and_default_speed() {
    let mut a = TrafficAutomaton::default();
    let mut w = World::default();
    let context = CollisionContext {
        occupant_flags: 0,
        leader_gone: true,
        too_close: true,
        speed_modifier: 1.0,
    };
    a.record.base.automata_flags = COLLISION | STOPPED;
    w.intersection = (1, 2.0);
    assert!(a.owner().update_collision(context, &mut w));
    assert_eq!(a.record.base.states[0].base.countdown_ms, 2000.0);
    assert_ne!(a.record.base.automata_flags & BLOCKED, 0);
    a.record.base.states.clear();
    w.intersection.0 = 0;
    a.owner().update_collision(context, &mut w);
    assert_ne!(a.record.base.automata_flags & BLOCKED, 0);
    a.owner().update_collision(
        CollisionContext {
            too_close: false,
            ..context
        },
        &mut w,
    );
    assert_eq!(a.record.base.automata_flags & BLOCKED, 0);
    a.record.base.automata_flags |= BLOCKED | 0x800;
    a.owner().update_collision(context, &mut w);
    assert_eq!(a.record.base.automata_flags & BLOCKED, 0);
}

#[test]
fn vehicle_path_anchors_and_measures_but_traffic_does_not() {
    let mut v = moving();
    let mut w = World {
        advance: Some(PathAdvance {
            position: [3.0, 0.0, 4.0],
            facing: [1.0, 0.0, 0.0],
            remaining: 15.0,
            at_end: true,
            stopped: true,
        }),
        ..World::default()
    };
    v.path_tick(
        100.0,
        Movement {
            leader_speed: Some(0.005),
            ..movement()
        },
        &mut w,
    )
    .unwrap();
    assert_eq!(v.measured_speed, 0.05);
    assert_eq!(v.record.traffic.base.vectors[1], [2.0, 0.0, 4.0]);
    assert_eq!(
        v.record.traffic.base.automata_flags & (STOPPED | REACHED),
        STOPPED | REACHED
    );
    assert_eq!(v.record.traffic.base.f3[1], 0.005);
    assert_eq!(v.traffic_owner().path_tick(100.0, movement(), false, &mut w), Ok(None));
    assert_eq!(v.record.traffic.base.vectors[1], [3.0, 0.0, 4.0]);
}

#[test]
fn movement_failures_remove_only_once_and_special_route_survives_reverse() {
    let mut v = moving();
    let mut w = World {
        allow: false,
        ..World::default()
    };
    v.traffic_owner().straight_tick(1.0, 1.0, &mut w);
    v.traffic_owner().remove(&mut w);
    assert_eq!(w.calls.iter().filter(|c| c.as_str() == "remove").count(), 1);
    v.record.traffic.base.automata_flags = SPECIAL_ROUTE;
    v.record.traffic.base.f3[..4].copy_from_slice(&[-20.0, -20.0, 0.0, 0.0]);
    assert_eq!(v.path_tick(1.0, movement(), &mut w), Err(Unsupported::RouteGeometryVh7));
    assert_eq!(v.record.traffic.base.automata_flags & REMOVE, 0);
    v.record.traffic.base.automata_flags = 0;
    assert_eq!(v.path_tick(1.0, movement(), &mut w), Ok(()));
    assert_ne!(v.record.traffic.base.automata_flags & REMOVE, 0);
}

#[test]
fn default_and_chase_seek_scripted_does_not() {
    let mut v = moving();
    let mut w = World::default();
    for kind in [0, 2, 17, 52] {
        v.record.traffic.base.states = vec![State::new(kind)];
        let old = v.record.traffic.base.vectors[0][0];
        v.animation_tick(100.0, movement(), true, false, &mut w).unwrap();
        assert_eq!(v.record.traffic.base.vectors[0][0], old + 1.0);
    }
    v.record.traffic.base.states = vec![State::new(10)];
    let old = v.record.traffic.base.vectors[0];
    v.animation_tick(100.0, movement(), true, false, &mut w).unwrap();
    assert_eq!(v.record.traffic.base.vectors[0], old);
}

#[test]
fn collision_final_timeout_disappears_and_train_minimum() {
    let mut v = moving();
    let mut w = World::default();
    v.enter_collision([1.0, 0.0, 0.0], &mut w).unwrap();
    assert_eq!(v.record.traffic.base.states[0].state_type, 20);
    assert_eq!(v.set_state(0, SetStateArgs::default(), &mut w), Ok(false));
    v.collide_tick(5000.0, &mut w);
    assert_eq!(v.record.traffic.base.states.last().unwrap().state_type, 1);
    assert!(v.state_tick(1.0, movement(), &mut w).unwrap());
    assert_ne!(v.record.traffic.base.automata_flags & REMOVE, 0);
    let mut train = Train::default();
    train.vehicle.enter_collision([1.0, 0.0, 0.0], &mut w).unwrap();
    train.collide_tick(100.0, &mut w);
    assert_eq!(train.vehicle.record.traffic.base.states[0].base.countdown_ms, 9900.0);
}

#[test]
fn plummet_tumble_then_land_removes_without_pop() {
    let mut v = moving();
    let mut w = World::default();
    let mut state = State::new(14);
    state.extra = StateExtra::T0E {
        descent: -0.01,
        roll_rate: 0.5,
        roll_centre: 0.0,
        tumble_ms: 10.0,
        post_splash_ms: 0.0,
    };
    v.record.traffic.base.states.push(state);
    v.record.traffic.base.vectors[0] = [0.0, 1.0, 0.0];
    v.plummet_tick(100.0, &mut w).unwrap();
    assert_eq!(v.record.traffic.base.vectors[0][1], 1.0);
    v.plummet_tick(200.0, &mut w).unwrap();
    assert_ne!(v.record.traffic.base.automata_flags & REMOVE, 0);
    assert_eq!(v.record.traffic.base.states.len(), 1);
}

#[test]
fn splash_survives_until_timer_then_releases_exactly_once() {
    let mut v = moving();
    let mut w = World {
        water: 3.0,
        ..World::default()
    };
    let mut state = State::new(14);
    state.extra = StateExtra::T0E {
        descent: -0.01,
        roll_rate: 0.5,
        roll_centre: 0.0,
        tumble_ms: 0.0,
        post_splash_ms: 0.0,
    };
    v.record.traffic.base.states.push(state);
    v.record.traffic.base.vectors[0] = [0.0, 2.0, 0.0];
    v.splash_seconds = Some(1.0);
    v.plummet_tick(1.0, &mut w).unwrap();
    assert_eq!(v.splash_effect, Some(42));
    assert_eq!(v.record.traffic.base.automata_flags & REMOVE, 0);
    v.plummet_tick(1000.0, &mut w).unwrap();
    v.shutdown(&mut w);
    assert_eq!(w.calls.iter().filter(|c| c.as_str() == "destroy:42").count(), 1);
    assert_ne!(v.record.traffic.base.automata_flags & REMOVE, 0);
}

#[test]
fn pull_over_initializes_moves_and_may_pop_twice() {
    let mut v = moving();
    let mut w = World::default();
    v.record.traffic.base.states = vec![State::new(10), State::new(21)];
    v.pull_over_tick(500.0, &mut w);
    assert_eq!(v.record.traffic.base.vectors[0], [0.0; 3]);
    v.pull_over_tick(500.0, &mut w);
    assert!(v.record.traffic.base.vectors[0][0] > 0.0);
    assert!(v.record.traffic.base.vectors[0][2] > 0.0);
    assert_eq!(v.measured_speed, 0.0);
    w.traffic = 4;
    w.controller = false;
    v.pull_over_tick(1.0, &mut w);
    assert!(v.record.traffic.base.states.is_empty());
    assert_eq!(v.record.traffic.base.automata_flags & REMOVE, 0);
}

#[test]
fn record_roundtrip_and_every_truncation_atomic_before_init() {
    let mut original = moving();
    // Occupant Read sets the loaded-status bit; compare a loaded-form record.
    original.record.traffic.base.occupant.status = 2;
    original.record.path = vec![(7, -9, 123)];
    original.record.traffic.route = vec![[2, 3, 4]];
    original.record.traffic.base.states = vec![State::new(14), State::new(21)];
    let mut out = ByteWriter::new();
    assert!(original.write(&mut out));
    let bytes = out.into_bytes();
    let mut loaded = Vehicle::default();
    assert!(loaded.read(&mut ByteReader::new(&bytes)));
    assert_eq!(loaded.record, original.record);
    assert_eq!(loaded.splash_effect, None);
    for end in 0..bytes.len() {
        let mut target = moving();
        let before = target.clone();
        assert!(!target.read(&mut ByteReader::new(&bytes[..end])), "end={end}");
        assert_eq!(target, before);
    }
}

#[test]
fn train_shutdown_owns_cars_and_copies_measured_speed() {
    let mut t = Train {
        cars: vec![TrainCar::default(), TrainCar::default()],
        ..Train::default()
    };
    let mut w = World::default();
    t.vehicle.measured_speed = 0.75;
    t.enter_collision(&[[1.0, 0.0, 0.0]; 3], &mut w).unwrap();
    assert!(t.cars.iter().all(|c| c.vehicle.measured_speed == 0.75));
    t.cars[0].vehicle.splash_effect = Some(42);
    t.shutdown(&mut w);
    assert!(t.cars.is_empty());
    assert_eq!(w.calls.iter().filter(|c| c.as_str() == "destroy:42").count(), 1);
}

#[test]
fn unresolved_train_records_touch_neither_stream_nor_owner() {
    let mut train = Train::default();
    let before = train.clone();
    let mut out = ByteWriter::new();
    assert_eq!(train.write(&mut out), Err(Unsupported::TrainRecordVh3));
    assert!(out.into_bytes().is_empty());
    assert_eq!(train.read(&mut ByteReader::new(&[])), Err(Unsupported::TrainRecordVh3));
    assert_eq!(train, before);
    assert_eq!(train.init(), Err(Unsupported::TrainCompositionVh5));
}
