// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::excessive_precision)] // Retain the spec's decimal conformance vectors verbatim.
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::cell::Cell as Counter;

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}
fn vector(a: Vec3, b: Vec3) {
    for i in 0..3 {
        near(a[i], b[i]);
    }
}

#[test]
fn wc3_t01_through_t04_sampling() {
    let mut t = SpeedTween {
        start: 2.0,
        target: 10.0,
        duration: 4.0,
        elapsed: 1.0,
    };
    assert_eq!(t.sample(), 4.0);
    t.elapsed = -1.0;
    assert_eq!(t.sample(), 2.0);
    t.elapsed = 8.0;
    assert_eq!(t.sample(), 10.0);
    for duration in [0.0, -1.0, f32::NAN] {
        t.duration = duration;
        assert_eq!(t.sample(), 10.0);
    }
    t.duration = 4.0;
    t.elapsed = f32::NAN;
    assert!(t.sample().is_nan());
    t.elapsed = -1.0;
    t.target = f32::INFINITY;
    assert!(t.sample().is_nan());
}

#[test]
fn wc4_ai01_ai02_refill_and_strict_short_distance() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.travel.truncate(2);
    b.speed = 1.0;
    b.turn = 1.0;
    f.insert(1, b).unwrap();
    let target = f.ai_motion(1, 100.0, 0.0, |_| true, |_| [0.0; 3], &mut None).unwrap();
    assert_eq!(target, AiTargets::default());
    assert_eq!((f.get(1).unwrap().speed, f.get(1).unwrap().turn), (0.0, 0.0));
    let b = f.get_mut(1).unwrap();
    let target = b.ai_midpoints(100.0, [[0.05, 0.0, 0.0], [10.0, 0.0, 0.0]], 0.0);
    assert!(target.remove);
    assert!(b.travel.is_empty());
    let mut b = boat();
    b.inherited_heading = [1.0, 0.0, 0.0];
    b.travel = vec![
        TravelPoint {
            cell: Cell { x: 0, z: 0 },
            tried: 0,
            direction: 0
        };
        3
    ];
    assert!(!b.ai_midpoints(100.0, [[0.1, 0.0, 0.0], [10.0, 0.0, 0.0]], 0.0).remove);
}

#[test]
fn wc4_ai03_through_ai05_asymmetric_turn_signs_and_publication() {
    for (mid, latch, expected_sign, target_sign) in [
        ([[10.0, 0.0, 0.0], [20.0, 0.0, 0.0]], false, 1.0, 1.0),
        ([[-10.0, 0.0, 0.0], [-20.0, 0.0, 0.0]], false, -1.0, 1.0),
        ([[3.0, 0.0, 0.0], [-10.0, 0.0, 0.0]], true, -1.0, -1.0),
    ] {
        let mut b = boat();
        b.inherited_position = [0.0; 3];
        b.inherited_heading = [0.0, 0.0, 1.0];
        b.position = [4.0; 3];
        b.heading = [1.0, 0.0, 0.0];
        b.ai_turn_latch = latch;
        b.drive.0[2] = 0.002;
        let t = b.ai_midpoints(100.0, mid, 0.0);
        near(t.turn, 0.002 * target_sign);
        assert_eq!(t.speed, 0.0);
        near(b.heading[0], expected_sign * 0.17364818);
        near(b.heading[2], 0.98480775);
        assert_eq!(b.previous_position, [4.0; 3]);
        assert_eq!(b.previous_heading, [1.0, 0.0, 0.0]);
        assert!(matches!(b.events.last(), Some(Event::Publish(Publication::MoveTo(..)))));
    }
}

#[test]
fn wc4_ai06_through_ai10_translation_time_and_nan() {
    for (dt, water, mid, expected, speed) in [
        (100.0, 0.0, 7.0, [0.25, 0.0, 0.0], 0.1),
        (1000.0, 6.0, 8.0, [2.0, 1.5, 0.0], 0.1),
        (5000.0, 6.0, 8.0, [8.0, 6.0, 0.0], 0.08),
        (-100.0, 0.0, 7.0, [-0.25, 0.0, 0.0], 0.1),
    ] {
        let mut b = boat();
        b.inherited_position = [0.0; 3];
        b.inherited_heading = [1.0, 0.0, 0.0];
        let t = b.ai_midpoints(dt, [[mid, 0.0, 0.0], [mid + 10.0, 0.0, 0.0]], water);
        vector(b.position, expected);
        near(t.speed, speed);
        assert_eq!(t.turn, 0.0);
    }
    let mut b = boat();
    b.inherited_position = [0.0; 3];
    b.inherited_heading = [0.0, 0.0, 1.0];
    let t = b.ai_midpoints(0.0, [[10.0, 0.0, 0.0], [20.0, 0.0, 0.0]], 0.0);
    assert!(t.turn.is_nan());
    vector(b.heading, [0.0, 0.0, 1.0]);
}

struct Ground {
    normal: Vec3,
    inside: bool,
    height: f32,
    samples: Counter<u32>,
}
impl Terrain for Ground {
    fn contains(&self, _: Cell) -> bool {
        self.inside
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        self.samples.set(self.samples.get() + 1);
        Cell { x: 0, z: 0 }
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        self.height
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        self.height
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        0.0
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}
impl GroundingTerrain for Ground {
    fn normal(&self, _: Vec3) -> Vec3 {
        self.samples.set(self.samples.get() + 1);
        self.normal
    }
    fn world_inside(&self, _: Vec3) -> bool {
        self.inside
    }
    fn cell_height(&self, _: Cell) -> f32 {
        self.height
    }
}
fn ground() -> Ground {
    Ground {
        normal: [0.0, 1.0, 0.0],
        inside: true,
        height: 22.0,
        samples: Counter::new(0),
    }
}
fn heeling() -> Watercraft {
    let mut b = boat();
    b.position = [10.0, 20.0, 30.0];
    b.heading = [0.0, 0.0, 1.0];
    b.previous_heading = b.heading;
    b.drive.0[24] = 1.0;
    b.drive.0[25] = 1.0;
    b.crash_elapsed = 800.0;
    b
}
fn publications(b: &Watercraft) -> usize {
    b.events
        .iter()
        .filter(|e| matches!(e, Event::Publish(Publication::Transform(_))))
        .count()
}

#[test]
fn wc5_h01_through_h05_heel_axes_time_and_boundaries() {
    let g = ground();
    let mut b = heeling();
    b.crash_elapsed = 8000.0;
    b.shore_contact = true;
    b.speed = 1.0;
    assert!(!b.heel_tick(100.0, &g));
    assert_eq!(g.samples.get(), 0);
    assert_eq!(b.speed, 1.0);
    let mut b = heeling();
    b.crash_angle = 0.25;
    assert!(b.heel_tick(100.0, &g));
    vector(b.heading, [0.0, 0.0, 1.0]);
    vector(b.transform.basis[0], [0.25_f32.cos(), 0.25_f32.sin(), 0.0]);
    vector(b.transform.translation, [10.0, 20.0, 30.0]);
    for dt in [0.0, 1000.0] {
        let mut b = heeling();
        assert!(b.heel_tick(dt, &g));
        vector(b.heading, [0.24740396, 0.0, 0.96891242]);
        assert_eq!(b.inherited_heading, b.heading);
        assert_eq!(b.previous_heading, [0.0, 0.0, 1.0]);
    }
    let mut b = heeling();
    b.crash_elapsed = 3200.0;
    assert!(!b.heel_tick(10.0, &g));
    assert_eq!(b.crash_angle, 1.0);
    assert_eq!(publications(&b), 0);
}

#[test]
fn wc5_h06_through_h10_contact_order_and_strict_gates() {
    let mut g = ground();
    g.normal = [0.0, 0.0, -1.0];
    let mut b = heeling();
    b.crash_elapsed = 0.0;
    b.shore_contact = true;
    b.drive.0[25] = 0.0;
    b.speed = 1.0;
    assert!(b.heel_tick(10.0, &g));
    assert!(!b.shore_contact);
    assert_eq!(b.speed, 1.0);
    for (inside, height, count, sliding) in [(false, 22.0, 1, false), (true, 30.0, 1, false), (true, 22.0, 2, true)] {
        let mut g = ground();
        g.inside = inside;
        g.height = height;
        let mut b = heeling();
        b.crash_elapsed = 0.0;
        b.shore_contact = true;
        b.speed = 1.0;
        b.drive.0[23] = 0.5;
        // WC12: incoming C must not affect the banked second terrain normal.
        b.transform.basis[2] = [0.0, -1.0, 0.0];
        b.transform.scale = 2.0;
        assert!(b.heel_tick(10.0, &g));
        assert_eq!(b.speed, 0.5);
        assert_eq!(b.shore_contact, sliding);
        assert_eq!(publications(&b), count);
        assert_eq!(b.transform.scale, 2.0);
        if sliding {
            vector(b.position, [10.0, 22.0, 31.0]);
            vector(b.previous_position, [10.0, 20.0, 30.0]);
        }
    }
    let mut b = heeling();
    b.crash_elapsed = 0.0;
    b.shore_contact = true;
    b.speed = 1.0;
    b.drive.0[23] = 0.5;
    b.drive.0[24] = 0.0;
    b.transform.basis[2] = [0.0, -1.0, 0.0];
    assert!(!b.heel_tick(10.0, &ground()));
    assert_eq!(publications(&b), 1);
    vector(b.position, [10.0, 22.0, 31.0]);
}

#[test]
fn wc12_h11_endpoint_depends_only_on_p24() {
    for endpoint in [1.0, 2.0] {
        let mut b = heeling();
        b.crash_elapsed = 3200.0;
        b.drive.0[24] = endpoint;
        b.drive.0[25] = 2.0;
        assert!(!b.heel_tick(10.0, &ground()));
        assert_eq!(b.crash_angle, endpoint);
        assert_eq!(publications(&b), 0);
    }
}

#[test]
fn wc12_h12_contact_ignores_incoming_rotation_and_preserves_scale() {
    for rotation in [
        Transform::default().basis,
        [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
    ] {
        let mut b = heeling();
        b.crash_elapsed = 0.0;
        b.shore_contact = true;
        b.speed = 1.0;
        b.drive.0[23] = 0.5;
        b.transform.basis = rotation;
        b.transform.scale = 3.0;
        let revision = b.transform.revision;
        let g = ground();
        assert!(b.heel_tick(10.0, &g));
        let first = b
            .events
            .iter()
            .find_map(|e| match e {
                Event::Publish(Publication::Transform(t)) => Some(t),
                _ => None,
            })
            .unwrap();
        assert_eq!(first.basis, Transform::default().basis);
        assert_eq!(first.scale, 3.0);
        assert_eq!(first.revision, revision.wrapping_add(2));
        assert_eq!(g.samples.get(), 4); // two normals, conversion, heel normal
    }
}

struct ChangingNormal {
    ground: Ground,
    calls: Counter<u32>,
}
impl Terrain for ChangingNormal {
    fn contains(&self, c: Cell) -> bool {
        self.ground.contains(c)
    }
    fn cell_at(&self, p: Vec3) -> Cell {
        self.ground.cell_at(p)
    }
    fn terrain_height(&self, p: Vec3) -> f32 {
        self.ground.terrain_height(p)
    }
    fn terrain_height_at_cell(&self, c: Cell) -> f32 {
        self.ground.terrain_height_at_cell(c)
    }
    fn sea_level(&self) -> f32 {
        self.ground.sea_level()
    }
    fn water_height(&self, p: Vec3) -> f32 {
        self.ground.water_height(p)
    }
    fn navigable(&self, c: Cell, r: u32, lot: Option<&Resource>) -> bool {
        self.ground.navigable(c, r, lot)
    }
}
impl GroundingTerrain for ChangingNormal {
    fn normal(&self, _: Vec3) -> Vec3 {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call == 0 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        }
    }
    fn world_inside(&self, p: Vec3) -> bool {
        self.ground.world_inside(p)
    }
    fn cell_height(&self, c: Cell) -> f32 {
        self.ground.cell_height(c)
    }
}
#[test]
fn wc12_h13_second_normal_parallel_to_heading_keeps_unguarded_basis() {
    let mut b = heeling();
    b.crash_elapsed = 0.0;
    b.shore_contact = true;
    b.speed = 1.0;
    b.drive.0[23] = 0.5;
    let g = ChangingNormal {
        ground: ground(),
        calls: Counter::new(0),
    };
    assert!(b.heel_tick(10.0, &g));
    let first = b
        .events
        .iter()
        .find_map(|e| match e {
            Event::Publish(Publication::Transform(t)) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(first.basis[0].iter().all(|x| x.is_nan()));
    assert_eq!(g.calls.get(), 3);
    assert_eq!(b.position, [10.0, 22.0, 31.0]);
}

#[derive(Default)]
struct Fx(Vec<CrashEffect>);
impl CrashEffects for Fx {
    fn refresh(&mut self, kind: CrashEffect, _: &Transform) {
        self.0.push(kind);
    }
    fn splash_present(&self) -> bool {
        true
    }
}
#[test]
fn crash_dispatch_double_time_outside_and_effect_order() {
    let mut f = Fleet::default();
    let mut b = heeling();
    b.crash_elapsed = 0.0;
    b.record.base.states.push(State::new(state::SINK));
    b.sunk_by_collision = true;
    b.player.speed_tween.target = 1.0;
    f.insert(1, b).unwrap();
    let mut fx = Fx::default();
    let cells = [Cell { x: 0, z: 0 }; 4];
    assert!(f
        .crash_state_tick(1, 10.0, cells, &ground(), &mut fx, &mut None)
        .unwrap());
    let b = f.get(1).unwrap();
    assert_eq!(b.crash_elapsed, 20.0);
    assert_eq!(b.player.speed_tween.target, 0.0);
    assert_eq!(fx.0, [CrashEffect::Mushroom, CrashEffect::Splash]);
    let mut g = ground();
    g.inside = false;
    fx.0.clear();
    f.crash_state_tick(1, 10.0, cells, &g, &mut fx, &mut None).unwrap();
    assert_eq!(f.get(1).unwrap().crash_elapsed, 30.0);
    assert!(fx.0.is_empty());
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::IMMEDIATE_REMOVE));
}

#[test]
fn wc4_i01_i03_inherited_dispatch_and_completion() {
    let mut b = boat();
    b.record.base.automata_flags = 0;
    b.player.speed_tween = SpeedTween {
        start: 0.0,
        target: 8.0,
        duration: 1.0,
        elapsed: 0.9,
    };
    let mut calls = vec![];
    assert!(b.inherited_animation(
        5.0,
        200.0,
        |_, slot, a| calls.push((slot, a)),
        |_, _| panic!("state flag clear"),
        1.0,
        |_, _, _| {}
    ));
    assert_eq!(calls, [(83, 200.0), (103, 200.0)]);
    assert_eq!(b.player.speed_tween.start, 8.0);
    assert_eq!(b.events.last(), Some(&Event::Notify(0, 0)));
    b.record.base.automata_flags = 2;
    b.inherited_position = [1.0, 2.0, 3.0];
    b.inherited_destination = [-1.0, 20.0, 30.0];
    b.events.clear();
    b.inherited_animation(
        0.0,
        7.0,
        |_, _, _| {},
        |_, a| {
            assert_eq!(a, 7.0);
            false
        },
        1.0,
        |_, _, _| {},
    );
    assert_eq!(b.inherited_destination, b.inherited_position);
    assert_eq!(b.events, [Event::Notify(4, 0), Event::Notify(0, 0)]);
    let _ = Water;
}

#[derive(Default)]
struct Model {
    visible: Vec<(bool, f32)>,
    times: Vec<f32>,
}
impl AnimatedModel for Model {
    fn visibility(&mut self, translucent: bool, value: f32) {
        self.visible.push((translucent, value));
    }
    fn advance(&mut self, time: f32) {
        self.times.push(time);
    }
    fn frame_count(&self) -> f32 {
        5.0
    }
    fn frame(&self) -> f32 {
        4.0
    }
}
struct Animation {
    first: bool,
    second: bool,
    calls: Vec<u32>,
}
impl AnimationService for Animation {
    fn slot17(&mut self) -> bool {
        self.calls.push(17);
        self.first
    }
    fn slot15(&mut self) -> bool {
        self.calls.push(15);
        self.second
    }
}
#[derive(Default)]
struct Lights(u32);
impl Lighting for Lights {
    fn light_model(&mut self, control: bool) {
        assert!(!control);
        self.0 += 1;
    }
}

#[test]
fn subsidiary_visibility_suppression_completion_and_strict_lighting() {
    let mut s = Subsidiary {
        visibility: SpeedTween {
            start: 0.0,
            target: 2.0,
            duration: 1.0,
            elapsed: 0.0,
        },
        attachments_present: true,
        ..Subsidiary::default()
    };
    let mut m = Model::default();
    s.visibility_tick(500.0, &mut [None, Some(&mut m)]);
    assert_eq!(m.visible, [(false, 1.0)]);
    s.visibility_tick(500.0, &mut [Some(&mut m)]);
    assert_eq!(s.visibility.duration, 0.0);
    let mut b = boat();
    b.events.clear();
    let mut a = Animation {
        first: false,
        second: true,
        calls: vec![],
    };
    assert!(s.animation_tick(7.0, &mut a, &mut [Some(&mut m)], &mut b));
    assert!(m.times.is_empty());
    assert_eq!(a.calls, [17, 15]);
    assert_eq!(s.animation_elapsed, 0.0);
    a.first = true;
    a.calls.clear();
    s.animation_tick(7.0, &mut a, &mut [Some(&mut m), None], &mut b);
    assert_eq!(m.times, [7.0]);
    assert_eq!(a.calls, [17]);
    assert_eq!(s.animation_elapsed, 7.0);
    assert_eq!(b.events, [Event::Notify(1, 0)]);
    let mut lights = Lights::default();
    b.record.base.automata_flags = 0;
    b.inherited_heading = [1.0, 0.0, 0.0];
    b.inherited_position = [2.0, 0.0, 0.0];
    s.lighting_tick(&b, true, [1.0; 3], 4.0, Some(&mut lights));
    assert_eq!(lights.0, 0);
    b.inherited_position[0] = 3.0;
    s.lighting_tick(&b, true, [1.0; 3], 4.0, Some(&mut lights));
    assert_eq!(lights.0, 1);
    assert_eq!(s.last_lit_position, b.inherited_position);
}
