// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::cell::Cell as Counter;

impl GroundingTerrain for Water {
    fn normal(&self, _: Vec3) -> Vec3 {
        [0.0, 1.0, 0.0]
    }
    fn world_inside(&self, p: Vec3) -> bool {
        self.contains(self.cell_at(p))
    }
    fn cell_height(&self, _: Cell) -> f32 {
        -10.0
    }
}

struct Spatial;
impl SpatialServices for Spatial {
    fn centre(&self, c: Cell) -> Vec3 {
        [c.x as f32, 0.0, c.z as f32]
    }
    fn adjust_segment(&mut self, _: Vec3, _: &mut Vec3, mask: u32) {
        assert_eq!(mask, 0xC49);
    }
}

#[derive(Default)]
struct Queries(usize);
impl CollisionQueries for Queries {
    fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
        self.0 += 1;
        [Cell { x: 10, z: 10 }; 4]
    }
    fn occupied(&mut self, _: Cell) -> bool {
        false
    }
    fn is_wet(&mut self, _: Cell) -> bool {
        true
    }
    fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
        [0.0, 1.0, 0.0]
    }
}

#[derive(Default)]
struct Effects;
impl PlayerEffects for Effects {
    fn play_fire(&mut self, _: &'static str) -> bool {
        false
    }
    fn fire_model_hook(&mut self) {}
    fn has_waterski(&self) -> bool {
        false
    }
    fn waterski_transform(&mut self, _: &Transform) {}
    fn waterski(&mut self, _: bool) {}
    fn command_event(&mut self, _: usize, _: u32) {}
}
impl CrashEffects for Effects {
    fn refresh(&mut self, _: CrashEffect, _: &Transform) {}
    fn splash_present(&self) -> bool {
        false
    }
}

#[test]
fn wc6_10_full_drive_ignores_base_false_and_uses_two_times() {
    for denied in [false, true] {
        let mut f = Fleet::default();
        let mut b = boat();
        b.position = [10.0, 0.0, 10.0];
        b.inherited_position = b.position;
        b.record.base.automata_flags = u32::from(denied);
        b.speed = 0.1;
        f.insert(1, b).unwrap();
        let mut queries = Queries::default();
        let mut motion = MotionServices {
            terrain: &Water,
            queries: &mut queries,
            spatial: &mut Spatial,
            ai_collisions: true,
        };
        let mut command = DriveCommand::default();
        let mut drive = DriveServices {
            command: &mut command,
            effects: &mut Effects,
            removal_predicate: true,
            cell: Cell { x: -1, z: -1 },
        };
        assert!(f
            .player_drive_tick(1, 200.0, &mut motion, &mut drive, &mut None)
            .unwrap());
        assert_eq!(drive.cell, Cell { x: -1, z: -1 });
        let b = f.get(1).unwrap();
        assert!((b.speed - 0.09).abs() < 1e-6);
        if denied {
            assert_eq!(b.position, [10.0, 0.0, 10.0]);
        } else {
            // Straight movement: 0.09 * 100 ms * p[4]=0.2.
            assert!((b.position[0] - 11.8).abs() < 1e-5);
        }
    }
}

#[test]
fn inherited_channels_saturate_and_preserve_negative_sentinels() {
    let mut c = DriveCommand {
        throttle: f32::MAX,
        reverse: -1.0,
        left: 0.0,
        ..DriveCommand::default()
    };
    c.advance_channels(f32::MAX);
    assert_eq!(c.throttle, f32::MAX);
    assert_eq!(c.left, f32::MAX);
    assert_eq!(c.reverse, -1.0);
    c.left = 0.0;
    c.advance_channels(-10.0);
    assert_eq!(c.left, -10.0);
}

#[test]
fn picker_fills_ten_core_points_and_ai_consumes_without_host_targets() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.position = [10.0, 0.0, 10.0];
    b.inherited_position = b.position;
    f.insert(1, b).unwrap();
    assert!(f.pick_location(1, &Water, |c| Spatial.centre(c), &mut None).unwrap());
    let b = f.get(1).unwrap();
    assert_eq!(b.travel.len(), 10);
    let direction = quantize_heading(b.heading);
    assert_eq!(
        b.travel[0].cell,
        direction_step(Cell { x: 10, z: 10 }, direction as i32)
    );
    assert_eq!(b.travel[0].direction, direction);
    f.ai_motion_core(1, 10.0, &Water, |c| Spatial.centre(c), &mut None)
        .unwrap();
    assert_eq!(f.get(1).unwrap().travel.len(), 9);
}

struct Blocked(Counter<usize>);
impl Terrain for Blocked {
    fn contains(&self, _: Cell) -> bool {
        true
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        Cell { x: 10, z: 10 }
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        -10.0
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        -10.0
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        0.0
    }
    fn navigable(&self, _: Cell, radius: u32, _: Option<&Resource>) -> bool {
        assert_eq!(radius, 1);
        self.0.set(self.0.get() + 1);
        false
    }
}

#[test]
fn wandering_search_stops_after_fifteen_rounds_and_resets_failed_targets() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.speed = 1.0;
    b.turn = 2.0;
    f.insert(1, b).unwrap();
    let terrain = Blocked(Counter::new(0));
    assert_eq!(
        f.ai_motion_core(1, 10.0, &terrain, |c| Spatial.centre(c), &mut None)
            .unwrap(),
        AiTargets::default()
    );
    assert_eq!(terrain.0.get(), 15 * 7);
    let b = f.get(1).unwrap();
    assert!(b.travel.is_empty());
    assert_eq!((b.speed, b.turn), (0.0, 0.0));
}

#[derive(Default)]
struct Instance {
    calls: Vec<&'static str>,
    running: bool,
}
impl CrashInstance for Instance {
    fn set_transform(&mut self, t: &Transform) {
        assert_eq!(t.scale, 2.0);
        self.calls.push("transform");
    }
    fn running(&self) -> bool {
        self.running
    }
    fn start(&mut self, control: bool) {
        assert!(!control);
        self.calls.push("start(false)");
        self.running = true;
    }
}

#[test]
fn crash_effect_refresh_orders_transform_before_start_and_requires_gated_effects() {
    let mut grounding = Instance::default();
    let transform = Transform {
        scale: 2.0,
        ..Transform::default()
    };
    let mut effects = CrashEffectSet {
        grounding: Some(&mut grounding),
        mushroom: None,
        splash: None,
    };
    assert!(!effects.splash_present());
    effects.refresh(CrashEffect::Grounding, &transform);
    effects.refresh(CrashEffect::Grounding, &transform);
    assert_eq!(grounding.calls, ["transform", "start(false)", "transform"]);
    let result = std::panic::catch_unwind(|| {
        CrashEffectSet {
            grounding: None,
            mushroom: None,
            splash: None,
        }
        .refresh(CrashEffect::Mushroom, &transform);
    });
    assert!(result.is_err());
}

#[test]
fn common_dispatch_crash_completion_pops_and_idle_fallthrough_adds_only_once() {
    let mut f = Fleet::default();
    f.insert(1, boat()).unwrap();
    f.push_state(1, State::new(state::SINK), &mut None).unwrap();
    f.get_mut(1).unwrap().crash_elapsed = 7990.0;
    let mut motion = MotionServices {
        terrain: &Water,
        queries: &mut Queries::default(),
        spatial: &mut Spatial,
        ai_collisions: true,
    };
    assert!(f
        .runtime_state_tick(1, 10.0, &mut motion, &mut Effects, &mut None)
        .unwrap());
    assert_eq!(f.get(1).unwrap().crash_elapsed, 8010.0);
    assert_eq!(f.get(1).unwrap().top_state(), None);
    f.push_state(1, State::new(state::IDLE), &mut None).unwrap();
    f.get_mut(1).unwrap().idle_elapsed = 60_000.0;
    f.runtime_state_tick(1, 10.0, &mut motion, &mut Effects, &mut None)
        .unwrap();
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
    assert_eq!(f.get(1).unwrap().crash_elapsed, 8020.0);
}

#[test]
fn full_animation_orders_subsidiaries_wakes_caller_removal_and_lighting_with_two_times() {
    #[derive(Debug, PartialEq)]
    enum ModelCall {
        Visibility(bool, f32),
        Advance(f32),
    }
    struct Model(Vec<ModelCall>);
    impl AnimatedModel for Model {
        fn visibility(&mut self, translucent: bool, value: f32) {
            self.0.push(ModelCall::Visibility(translucent, value));
        }
        fn advance(&mut self, time: f32) {
            self.0.push(ModelCall::Advance(time));
        }
        fn frame_count(&self) -> f32 {
            10.0
        }
        fn frame(&self) -> f32 {
            0.0
        }
    }
    struct Service;
    impl AnimationService for Service {
        fn slot17(&mut self) -> bool {
            true
        }
        fn slot15(&mut self) -> bool {
            panic!("slot 17 true must short-circuit slot 15")
        }
    }
    struct Rope;
    impl RopeEffect for Rope {
        fn set_transform(&mut self, _: &Transform) {
            panic!("no tow")
        }
        fn running(&self) -> bool {
            false
        }
        fn start(&mut self, _: bool) {
            panic!("no tow")
        }
        fn stop(&mut self, _: bool) {
            panic!("already stopped")
        }
    }
    #[derive(Default)]
    struct Lights(Vec<bool>);
    impl Lighting for Lights {
        fn light_model(&mut self, control: bool) {
            self.0.push(control);
        }
    }
    for protected in [false, true] {
        let mut f = Fleet::default();
        let mut b = boat();
        b.record.base.automata_flags = 0;
        b.still_elapsed = 5000.0;
        b.player.speed_tween = SpeedTween {
            start: 2.0,
            target: 4.0,
            duration: 1.0,
            elapsed: 0.0,
        };
        if protected {
            b.record.base.states.push(State::new(state::IDLE));
        }
        f.insert(1, b).unwrap();
        let mut subsidiary = Subsidiary {
            visibility: SpeedTween {
                start: 0.0,
                target: 1.0,
                duration: 1.0,
                elapsed: 0.0,
            },
            ..Subsidiary::default()
        };
        let mut model = Model(vec![]);
        let mut models: [Option<&mut dyn AnimatedModel>; 1] = [Some(&mut model)];
        let mut lights = Lights::default();
        let mut animation = AnimationServices {
            subsidiary: &mut subsidiary,
            models: &mut models,
            service: &mut Service,
            rope: Some(&mut Rope),
            lighting: Some(&mut lights),
            processed_model: true,
            heading_threshold: [0.1; 3],
            distance_squared: 1.0,
        };
        let mut queries = Queries::default();
        let mut motion = MotionServices {
            terrain: &Water,
            queries: &mut queries,
            spatial: &mut Spatial,
            ai_collisions: true,
        };
        assert_eq!(
            f.animation_tick_core(
                1,
                AnimationTimes {
                    movement: 100.0,
                    animation: 200.0,
                    scale: 1.0,
                    cell_size: 16.0,
                },
                &mut motion,
                &mut animation,
                &mut Effects,
                &mut None,
            ),
            Ok(protected)
        );
        let b = f.get(1).unwrap();
        assert_eq!(b.still_elapsed, 5100.0);
        assert_eq!(b.player.speed_tween.elapsed, 0.2);
        assert_eq!(b.models.event_counter, 200.0);
        assert_eq!(b.position, [0.0; 3]);
        let mut expected = vec![Event::Notify(0, 0)];
        for name in WAKES {
            expected.extend([Event::StopWake(name), Event::EffectUpdate]);
        }
        if protected {
            expected.extend([Event::EffectUpdate, Event::ModelUpdate]);
        } else {
            expected.push(Event::Remove(false));
        }
        assert_eq!(b.events, expected);
        assert_eq!(model.0, [ModelCall::Visibility(true, 0.2), ModelCall::Advance(200.0)]);
        assert_eq!(lights.0, if protected { vec![false] } else { vec![] });
        assert_eq!(queries.0, 0);
    }
}
