// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent literal traces from public FF3/FF4, not production goldens.
use super::*;
use crate::network_occupants::Xform;
use crate::rng::{FixedSeeds, RzRandom};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
enum Call {
    Create,
    Start(u32),
    Stop(u32),
    Activity,
    Release,
    Transform([f32; 3], u8, u8),
}
#[derive(Clone, Default)]
struct Trace(Arc<Mutex<Vec<Call>>>);
impl Trace {
    fn push(&self, call: Call) {
        self.0.lock().unwrap().push(call);
    }
    fn take(&self) -> Vec<Call> {
        std::mem::take(&mut self.0.lock().unwrap())
    }
}
struct Effect {
    trace: Trace,
    active: bool,
}
impl VisualEffect for Effect {
    fn start(&mut self, a: u32) {
        self.trace.push(Call::Start(a));
    }
    fn stop(&mut self, a: u32) {
        self.trace.push(Call::Stop(a));
    }
    fn is_active(&mut self) -> bool {
        self.trace.push(Call::Activity);
        self.active
    }
    fn set_transform(&mut self, t: &Xform) {
        self.trace.push(Call::Transform(t.translation, t.counter, t.mask));
    }
}
impl Drop for Effect {
    fn drop(&mut self) {
        self.trace.push(Call::Release);
    }
}
struct Factory {
    trace: Trace,
    active: bool,
    fail: bool,
}
impl EffectFactory for Factory {
    type Effect = Effect;
    fn create(&mut self, _: &str) -> Option<Effect> {
        self.trace.push(Call::Create);
        if self.fail {
            None
        } else {
            Some(Effect {
                trace: self.trace.clone(),
                active: self.active,
            })
        }
    }
}
fn factory() -> Factory {
    Factory {
        trace: Trace::default(),
        active: true,
        fail: false,
    }
}
fn stage() -> Stage {
    Stage::new(0).unwrap()
}

#[test]
fn fresh_constructor_raw_visibility_and_replacement_failure() {
    let mut f = Flame::from_initial(FlameInitial::native(SharedPositionWord(-7.)));
    assert_eq!(
        (f.made(), f.visible_byte(), f.wall(), f.side(), f.has_effect()),
        (0, 1, false, 0, false)
    );
    assert_eq!(f.position(), [-7.; 3]);
    assert_eq!(*f.transform(), Xform::default());
    f.set_visibility(1); // equal, no effect
    f.set_visibility(2); // RAW byte must change although both are true
    assert_eq!(f.visible_byte(), 2);
    let mut factory = factory();
    f.update(stage(), &mut factory).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![Call::Create, Call::Transform([-7.; 3], 2, 5), Call::Start(1)]
    );
    f.set_visibility(2);
    assert!(factory.trace.take().is_empty());
    f.set_visibility(1);
    f.set_visibility(0);
    f.set_visibility(0);
    assert_eq!(factory.trace.take(), vec![Call::Start(1), Call::Stop(1)]);
    factory.fail = true;
    f.update(stage(), &mut factory).unwrap();
    assert_eq!(factory.trace.take(), vec![Call::Stop(0), Call::Release, Call::Create]);
    assert!(!f.has_effect());
}

#[test]
fn live_view_sends_old_then_new_position_and_preserves_embedded_identity() {
    let mut f = Flame::from_initial(FlameInitial::native(SharedPositionWord(-7.)));
    let mut factory = factory();
    f.update(stage(), &mut factory).unwrap();
    factory.trace.take();
    f.place([3., 4., 5.], true, 0, 2).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![
            Call::Transform([-7.; 3], 2, 5),
            Call::Transform([3., 4., 5.], 2, 5),
            Call::Stop(1)
        ]
    );
    f.place([6., 7., 8.], false, 0, 1).unwrap();
    assert_eq!(
        factory.trace.take(),
        vec![Call::Transform([3., 4., 5.], 2, 5), Call::Transform([6., 7., 8.], 2, 5)]
    );
    assert_eq!(f.visible_byte(), 0);
    assert_eq!(*f.transform(), Xform::default());
}

#[test]
fn shutdown_made_and_activity_paths_and_direct_storage_transfer() {
    for (made, active, expected) in [
        (0, true, vec![Call::Release]),
        (1, false, vec![Call::Activity, Call::Release]),
        (1, true, vec![Call::Activity, Call::Stop(0), Call::Release]),
    ] {
        let mut f = Flame::from_initial(FlameInitial {
            made,
            ..FlameInitial::native(SharedPositionWord(0.))
        });
        let mut factory = factory();
        factory.active = active;
        f.update(stage(), &mut factory).unwrap();
        factory.trace.take();
        f.shutdown();
        assert_eq!(factory.trace.take(), expected);
        f.shutdown();
        assert!(factory.trace.take().is_empty());
    }
    let mut f = Flame::from_initial(FlameInitial::native(SharedPositionWord(0.)));
    let mut factory = factory();
    f.update(stage(), &mut factory).unwrap();
    factory.trace.take();
    let retained = f.into_effect();
    assert!(factory.trace.take().is_empty());
    drop(retained); // Rust-owned reference release, explicitly NOT native destruction
    assert_eq!(factory.trace.take(), vec![Call::Release]);
}

struct Base;
impl OccupantInit for Base {
    fn init(&mut self) {}
    fn join_group(&mut self, _: u32) {}
}
#[test]
fn initial_minus_one_quiet_changed_invalid_stage_stored_and_shutdown_guarded() {
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 0,
    };
    let mut random = RzRandom::new(0, &seeds);
    let mut p = Placement {
        instances: &[],
        bounds: [[0.; 3], [2.; 3]],
        target_type: FLORA_TYPE,
        view_level: 1,
        seeds: &seeds,
        fire_random: &mut random,
    };
    let mut fire = FireFlames::new(0, 0, FlameInitial::native(SharedPositionWord(0.)));
    let mut factory = factory();
    assert_eq!(fire.raw_stage(), -1);
    fire.set_raw_stage(-1, &mut p, &mut factory).unwrap();
    assert!(fire.flames().is_empty());
    assert!(factory.trace.take().is_empty());
    fire.set_stage(stage(), &mut p, &mut factory).unwrap();
    factory.trace.take();
    assert!(fire.shutdown()); // initialized=false: no cleanup at all
    assert_eq!(fire.flames().len(), 1);
    assert!(factory.trace.take().is_empty());
    fire.init(&mut Base, &mut factory).unwrap();
    factory.trace.take();
    assert!(fire.shutdown());
    assert!(fire.flames().is_empty());
    assert_eq!(factory.trace.take(), vec![Call::Release]); // made=0, no activity query
    assert_eq!(
        fire.set_raw_stage(9, &mut p, &mut factory),
        Err(PlacementError::Unsupported(Unsupported::EffectTable))
    );
    assert_eq!(fire.raw_stage(), 9);
    assert!(factory.trace.take().is_empty());
}

#[test]
fn box_draw_literal_prefix_is_side_horizontal_height_and_flora_keeps_defaults() {
    // LCG seed 0 -> 12345, raw=0, side=0; double state 0/process0
    // gives .5 on both draws. Thus z=10 then y=10, x=max=10.
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 0,
    };
    let mut random = RzRandom::new(0, &seeds);
    let mut p = Placement {
        instances: &[],
        bounds: [[2., 0., 4.], [10., 20., 16.]],
        target_type: 0,
        view_level: 1,
        seeds: &seeds,
        fire_random: &mut random,
    };
    let mut fire = FireFlames::<Effect>::new(0, 0, FlameInitial::native(SharedPositionWord(-7.)));
    fire.create_flames(&mut p).unwrap();
    assert_eq!(fire.flames()[0].position(), [10., 10., 10.]);
    assert_eq!((fire.flames()[0].wall(), fire.flames()[0].side()), (true, 0));
    assert_eq!(p.fire_random.lcg_state(), 12345);
    assert_eq!(p.fire_random.mul_state(), 0);
    assert_eq!(*fire.flames()[0].transform(), Xform::default());
    p.target_type = FLORA_TYPE;
    let mut flora = FireFlames::<Effect>::new(0, 0, FlameInitial::native(SharedPositionWord(-7.)));
    flora.create_flames(&mut p).unwrap();
    assert_eq!(flora.flames()[0].position(), [6., 0., 10.]);
    assert_eq!((flora.flames()[0].wall(), flora.flames()[0].side()), (false, 0));
    assert_eq!(p.fire_random.lcg_state(), 12345);
}

#[test]
fn model_creation_sends_old_side_before_writing_new_side() {
    let mut f = Flame::from_initial(FlameInitial::native(SharedPositionWord(-7.)));
    let mut factory = factory();
    f.update(stage(), &mut factory).unwrap();
    factory.trace.take();
    // New side1 is outside the hardware-trig domain, but the first call is
    // positively qualified side0 at NEW position. This is a prefix assertion,
    // never a claim to execute the native nonzero-angle path.
    assert_eq!(
        f.place_model([3., 4., 5.], true, 1, 1),
        Err(PlacementError::Unsupported(Unsupported::HardwareTrig))
    );
    assert_eq!(factory.trace.take(), vec![Call::Transform([3., 4., 5.], 2, 5)]);
    assert_eq!(f.side(), 1);
}

#[test]
fn nonconstant_raw_rng_prefix_distinguishes_horizontal_from_height() {
    // Independent rational derivation retained in build-runs/literal-prefix.py:
    // seed1 multiplier states 278dde6d, fb533a69, 59ebecb5.
    // Uniform words 3f278dde, 3ef6a675, 3f59ebec (PC24, nearest/even).
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 0,
    };
    let mut random = RzRandom::new(1, &seeds);
    let mut p = Placement {
        instances: &[],
        bounds: [[0.; 3], [1.; 3]],
        target_type: 0,
        view_level: 1,
        seeds: &seeds,
        fire_random: &mut random,
    };
    let mut fire = FireFlames::<Effect>::new(0, 0, FlameInitial::native(SharedPositionWord(0.)));
    fire.create_flames(&mut p).unwrap();
    // Integer LCG draw is side0; multiplicative path begins independently.
    assert_eq!(
        fire.flames()[0].position().map(f32::to_bits),
        [0x3f800000, 0x3ef6a675, 0x3f278dde]
    );
    assert_eq!(p.fire_random.mul_state(), 0xfb533a69);
    assert_eq!(p.fire_random.uniform_f64() as f32, f32::from_bits(0x3f59ebec));
}
