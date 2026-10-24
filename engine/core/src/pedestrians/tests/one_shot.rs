// SPDX-License-Identifier: GPL-3.0-or-later
//! pedestrians §8 PN1-I1–I10 and PN1-T1–T12, with controlled post-Advance observations.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Model {
    pub(super) speed: f32,
    frame: i32,
    frames: i32,
    accumulator: f32,
    mode: u8,
    after_advance: Option<i32>,
    advance_result: u32,
}

impl Model {
    fn new(frame: i32, frames: i32) -> Self {
        Self {
            speed: 0.25,
            frame,
            frames,
            accumulator: 0.75,
            mode: 2,
            after_advance: None,
            advance_result: 0,
        }
    }
}

impl OneShotWorld for World {
    fn opacity_updated(&mut self, dt_ms: f32, opacity: f32) {
        self.log(format!("fade:{dt_ms}"));
        self.opacity.push(opacity);
    }
    fn advance_model(&mut self, index: usize, dt_ms: f32) -> u32 {
        self.log(format!("advance:{index}:{dt_ms}"));
        let model = self.animation_models[index].as_mut().unwrap();
        if let Some(frame) = model.after_advance {
            model.frame = frame;
        }
        model.advance_result
    }
    fn model_frame(&mut self, index: usize) -> i32 {
        self.log(format!("frame:{index}"));
        self.animation_models[index].as_ref().unwrap().frame
    }
    fn model_frame_count(&mut self, index: usize) -> i32 {
        self.log(format!("frames:{index}"));
        self.animation_models[index].as_ref().unwrap().frames
    }
}

fn init_fixture(property: Vec<u8>, word: u32) -> (PedestrianOneShot, Random, World, Trace) {
    let trace = Trace::default();
    let random = Random {
        words: VecDeque::from([word]),
        trace: Some(trace.clone()),
        ..Random::default()
    };
    let world = World {
        property,
        animation_models: vec![None, Some(Model::new(2, 4)), Some(Model::new(1, 6))],
        trace: Some(trace.clone()),
        ..World::default()
    };
    let mut owner = PedestrianOneShot::default();
    owner.pedestrian.base_mut().occupant.status = 2;
    (owner, random, world, trace)
}

fn tick_fixture(count: u8, models: Vec<Option<Model>>) -> (PedestrianOneShot, World) {
    let owner = PedestrianOneShot {
        play_count: count,
        ..PedestrianOneShot::default()
    };
    let world = World {
        animation_models: models,
        ..World::default()
    };
    (owner, world)
}

#[test]
fn pn1_i1_constructor_count_five_without_model_setup() {
    let mut random = Random::default();
    let owner = PedestrianOneShot::construct(1, Tuning::default(), &mut random);
    assert_eq!(owner.play_count(), 5);
    assert_eq!(PedestrianOneShot::default().play_count(), 5);
    assert_eq!(random.integers, [(15, 20)]);
    assert!(random.doubles.is_empty() && random.spans.is_empty());
}

#[test]
fn pn1_i2_initial_parameter_base_speed_then_bounded_draw() {
    let (mut owner, mut random, mut world, trace) = init_fixture(vec![], 0);
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(
        owner.pedestrian.runtime.initial_parameter.to_bits(),
        (-0.5_f32).to_bits()
    );
    assert_eq!(owner.play_count(), 3);
    assert_eq!(random.spans, [8]);
    assert!(random.words.is_empty());
    assert_eq!(
        *trace.borrow(),
        [
            "range:-0.5:-0.5",
            "models",
            "effects",
            "occupant",
            "instance:1:0.01",
            "instance:2:0.01",
            "bounded:8",
        ]
    );
    for model in world.animation_models.iter().flatten() {
        assert_eq!(model.speed.to_bits(), 0x3C23_D70A);
    }
}

#[test]
fn pn1_i3_upper_endpoint_preserves_playback_state() {
    let (mut owner, mut random, mut world, _) = init_fixture(vec![], u32::MAX);
    let mut expected = world.animation_models.clone();
    for model in expected.iter_mut().flatten() {
        model.speed = 0.01;
    }
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(owner.play_count(), 10);
    assert_eq!(world.animation_models, expected);
    assert_eq!(random.spans, [8]);
}

#[test]
fn pn1_i4_repeat_init_resets_speed_and_count_not_initial_parameter() {
    let (mut owner, mut random, mut world, trace) = init_fixture(vec![2, 4, 99], 0x8000_0000);
    owner.pedestrian.runtime.traffic.initialized = true;
    owner.pedestrian.runtime.initial_parameter = 0.75;
    owner.set_play_count(1);
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(owner.play_count(), 3);
    assert_eq!(owner.pedestrian.runtime.initial_parameter, 0.75);
    assert!(random.doubles.is_empty());
    assert_eq!(*trace.borrow(), ["instance:1:0.01", "instance:2:0.01", "bounded:3"]);
    assert!(world
        .animation_models
        .iter()
        .flatten()
        .all(|m| m.speed.to_bits() == 0x3C23_D70A));
}

#[test]
fn pn1_i5_failed_init_retains_parameter_count_speed_and_retry_draw() {
    let (mut owner, mut random, mut world, trace) = init_fixture(vec![], 0);
    world.fail_init = true;
    let models = world.animation_models.clone();
    for _ in 0..2 {
        // The vector explicitly supplies a false initialized query on the retry.
        owner.pedestrian.runtime.traffic.initialized = false;
        assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(false));
        assert_eq!(owner.pedestrian.runtime.initial_parameter, -0.5);
        assert_eq!(owner.play_count(), 5);
        assert_eq!(world.animation_models, models);
    }
    assert_eq!(random.doubles, [(-0.5, -0.5); 2]);
    assert!(random.spans.is_empty());
    assert_eq!(random.words.len(), 1);
    assert_eq!(
        *trace.borrow(),
        [
            "range:-0.5:-0.5",
            "models",
            "occupant",
            "range:-0.5:-0.5",
            "models",
            "occupant",
        ]
    );
}

#[test]
fn pn1_i6_missing_or_short_property_uses_both_tuning_bytes() {
    // No exemplar, missing property and an empty property all yield no bytes at this boundary.
    for (case, property) in [
        ("no exemplar", vec![]),
        ("missing", vec![]),
        ("empty", vec![]),
        ("one byte", vec![99]),
    ] {
        for (word, count) in [(0, 3), (u32::MAX, 10)] {
            let (mut owner, mut random, mut world, _) = init_fixture(property.clone(), word);
            assert_eq!(
                owner.init(Tuning::default(), &mut random, &mut world),
                Ok(true),
                "{case}"
            );
            assert_eq!(owner.play_count(), count, "{case}");
            assert_eq!(random.spans, [8], "{case}");
        }
    }
}

#[test]
fn pn1_i7_zero_equal_bounds_empty_models_still_draw_without_removal() {
    let (mut owner, mut random, mut world, _) = init_fixture(vec![0, 0], u32::MAX);
    world.animation_models.clear();
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(owner.play_count(), 0);
    assert_eq!(random.spans, [1]);
    assert!(random.words.is_empty());
    assert_eq!(world.calls, ["models", "effects", "occupant"]);
    assert_eq!(owner.pedestrian.base().automata_flags & 1, 0);
}

#[test]
fn pn1_i8_equal_255_bounds_are_unsigned_and_still_draw() {
    let (mut owner, mut random, mut world, _) = init_fixture(vec![255, 255], 0);
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(owner.play_count(), 255);
    assert_eq!(random.spans, [1]);
    assert!(random.words.is_empty());
}

#[test]
fn pn1_i9_adjacent_reversed_bounds_consume_zero_span_draw() {
    for word in [0, 2, u32::MAX] {
        let (mut owner, mut random, mut world, _) = init_fixture(vec![10, 9], word);
        assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
        assert_eq!(owner.play_count(), 10);
        assert_eq!(random.spans, [0]);
        assert!(random.words.is_empty());
    }
}

#[test]
fn pn1_i10_reversed_bounds_wrap_span_and_use_product_high_half() {
    let (mut owner, mut random, mut world, _) = init_fixture(vec![10, 3], 2);
    assert_eq!(owner.init(Tuning::default(), &mut random, &mut world), Ok(true));
    assert_eq!(random.spans, [0xFFFF_FFFA]);
    assert_eq!(owner.play_count(), 11);
    assert!(random.words.is_empty());
}

#[test]
fn pn1_t1_fade_then_advance_ms_then_post_advance_frame_sample() {
    let mut model = Model::new(0, 4);
    model.after_advance = Some(2);
    model.advance_result = u32::MAX;
    let (mut owner, mut world) = tick_fixture(3, vec![Some(model)]);
    owner.pedestrian.base_mut().f3[4..8].copy_from_slice(&[0.25, 0.75, 1.0, 0.0]);
    let runtime = owner.pedestrian.runtime.clone();
    let speed_tween = owner.pedestrian.base().f3[..4].to_vec();
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 3);
    assert_eq!(world.calls, ["fade:25", "advance:0:25", "frame:0", "frames:0"]);
    assert_eq!(world.opacity, [0.2625]);
    assert_eq!(owner.pedestrian.base().f3[7], 0.025);
    assert_eq!(owner.pedestrian.base().f3[..4], speed_tween);
    assert_eq!(owner.pedestrian.runtime, runtime);
}

#[test]
fn pn1_t2_nulls_skipped_and_only_first_live_model_charges() {
    let (mut owner, mut world) = tick_fixture(3, vec![None, Some(Model::new(3, 4)), Some(Model::new(3, 4))]);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 2);
    assert_eq!(
        world.calls,
        ["fade:25", "advance:1:25", "frame:1", "frames:1", "advance:2:25"]
    );
}

#[test]
fn pn1_t3_later_final_model_cannot_charge() {
    let (mut owner, mut world) = tick_fixture(3, vec![Some(Model::new(2, 4)), Some(Model::new(3, 4))]);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 3);
    assert_eq!(
        world.calls,
        ["fade:25", "advance:0:25", "frame:0", "frames:0", "advance:1:25"]
    );
}

#[test]
fn pn1_t4_remove_false_precedes_remaining_model_advances() {
    let (mut owner, mut world) = tick_fixture(1, vec![None, Some(Model::new(3, 4)), Some(Model::new(3, 4))]);
    assert!(!owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 0);
    assert_eq!(
        world.calls,
        [
            "fade:25",
            "advance:1:25",
            "frame:1",
            "frames:1",
            "remove",
            "advance:2:25"
        ]
    );
    assert_eq!(owner.pedestrian.base().automata_flags & 1, 1);
    assert_eq!(world.animation_models.len(), 3); // deferred removal, no destruction
}

#[test]
fn pn1_t5_zero_waits_for_final_frame_and_never_wraps() {
    let (mut owner, mut world) = tick_fixture(0, vec![Some(Model::new(2, 4))]);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 0);
    assert!(!world.calls.iter().any(|call| call == "remove"));
    world.calls.clear();
    world.animation_models[0].as_mut().unwrap().after_advance = Some(3);
    assert!(!owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 0);
    assert_eq!(
        world.calls,
        ["fade:25", "advance:0:25", "frame:0", "frames:0", "remove"]
    );
}

#[test]
fn pn1_t6_empty_and_all_null_models_only_update_fade() {
    for models in [vec![], vec![None, None]] {
        let (mut owner, mut world) = tick_fixture(0, models);
        assert!(owner.animation_tick(123456, 25.0, &mut world));
        assert_eq!(owner.play_count(), 0);
        assert_eq!(world.calls, ["fade:25"]);
    }
}

#[test]
fn pn1_t7_held_final_frame_charges_on_each_zero_ms_tick() {
    let mut model = Model::new(3, 4);
    model.accumulator = 0.0;
    let (mut owner, mut world) = tick_fixture(3, vec![Some(model.clone())]);
    for (count, keep) in [(2, true), (1, true), (0, false)] {
        world.calls.clear();
        assert_eq!(owner.animation_tick(123456, 0.0, &mut world), keep);
        assert_eq!(owner.play_count(), count);
        let mut calls = vec!["fade:0", "advance:0:0", "frame:0", "frames:0"];
        if !keep {
            calls.push("remove");
        }
        assert_eq!(world.calls, calls);
        assert_eq!(world.animation_models[0], Some(model.clone()));
    }
}

#[test]
fn pn1_t8_single_frame_model_charges_without_advancing() {
    let (mut owner, mut world) = tick_fixture(2, vec![Some(Model::new(0, 1))]);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 1);
    assert!(!world.calls.iter().any(|call| call == "remove"));
    assert!(!owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 0);
    assert_eq!(world.calls.iter().filter(|call| *call == "remove").count(), 1);
}

#[test]
fn pn1_t9_zero_frame_count_uses_signed_negative_threshold() {
    let (mut owner, mut world) = tick_fixture(2, vec![Some(Model::new(0, 0))]);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 1);
    assert_eq!(world.calls, ["fade:25", "advance:0:25", "frame:0", "frames:0"]);
}

#[test]
fn pn1_t10_large_delta_and_advance_result_charge_at_most_once() {
    let mut model = Model::new(30, 4); // at/past final, without an equality-only shortcut
    model.advance_result = u32::MAX;
    let (mut owner, mut world) = tick_fixture(255, vec![Some(model)]);
    assert!(owner.animation_tick(123456, 1_000_000.0, &mut world));
    assert_eq!(owner.play_count(), 254);
    assert_eq!(
        world.calls,
        ["fade:1000000", "advance:0:1000000", "frame:0", "frames:0"]
    );
}

#[test]
fn pn1_t11_direct_tick_ignores_pause_and_stationary_flags() {
    for paused in [false, true] {
        for flags in [0, 0x20] {
            for (frame, count, remaining, keep) in [(2, 3, 3, true), (3, 3, 2, true), (3, 1, 0, false)] {
                let (mut owner, mut world) = tick_fixture(count, vec![Some(Model::new(frame, 4))]);
                world.paused = paused;
                owner.pedestrian.base_mut().automata_flags = flags;
                let runtime = owner.pedestrian.runtime.clone();
                assert_eq!(owner.animation_tick(123456, 25.0, &mut world), keep);
                assert_eq!(owner.play_count(), remaining);
                assert_eq!(owner.pedestrian.runtime, runtime);
                assert_eq!(world.paused, paused);
                let mut calls = vec!["fade:25", "advance:0:25", "frame:0", "frames:0"];
                if !keep {
                    calls.push("remove");
                }
                assert_eq!(world.calls, calls);
            }
        }
    }
}

#[test]
fn pn1_t12_byte_accessors_do_not_touch_playback_or_lifetime() {
    let (mut owner, mut world) = tick_fixture(3, vec![Some(Model::new(2, 4))]);
    let models = world.animation_models.clone();
    let pedestrian = owner.pedestrian.clone();
    owner.set_play_count(255);
    assert_eq!(owner.play_count(), 255);
    owner.set_play_count(0);
    assert_eq!(owner.play_count(), 0);
    assert!(world.calls.is_empty());
    assert_eq!(world.animation_models, models);
    assert_eq!(owner.pedestrian, pedestrian);
    assert!(owner.animation_tick(123456, 25.0, &mut world));
    assert_eq!(owner.play_count(), 0);
    assert_eq!(world.calls, ["fade:25", "advance:0:25", "frame:0", "frames:0"]);
}
