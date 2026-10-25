// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::boat;
use crate::model_reg_effects::{self as fx, EffectFactory, ModelInstance, VisualEffect};
use std::sync::{Arc, Mutex};

struct Instance;
impl ModelInstance for Instance {
    fn regions(&self) -> Option<&fx::Regions> {
        None
    }
    fn frame(&self) -> fx::InstanceFrame {
        fx::InstanceFrame {
            transform: fx::Transform::IDENTITY,
            flags: 0,
            animated_frame: Some(3),
        }
    }
}
#[derive(Debug, PartialEq)]
enum Call {
    Transform(String, fx::Transform),
    Motion(String, [f32; 3]),
    Start(String, u32),
    Stop(String, u32),
}
struct Effect {
    name: String,
    started: bool,
    calls: Arc<Mutex<Vec<Call>>>,
}
impl VisualEffect for Effect {
    fn is_started(&self) -> bool {
        self.started
    }
    fn start(&mut self, control: u32) {
        self.started = true;
        self.calls.lock().unwrap().push(Call::Start(self.name.clone(), control));
    }
    fn stop(&mut self, control: u32) {
        self.started = false;
        self.calls.lock().unwrap().push(Call::Stop(self.name.clone(), control));
    }
    fn set_transform(&mut self, transform: fx::Transform, _: u8, version: u8) {
        assert_eq!(version, 0);
        self.calls
            .lock()
            .unwrap()
            .push(Call::Transform(self.name.clone(), transform));
    }
    fn set_parameter(&mut self, parameter: u32, value: [f32; 3], count: u32) {
        assert_eq!((parameter, count), (15, 1));
        self.calls.lock().unwrap().push(Call::Motion(self.name.clone(), value));
    }
}
#[derive(Default)]
struct Factory(Arc<Mutex<Vec<Call>>>);
impl EffectFactory for Factory {
    fn acquire_manager(&mut self) -> bool {
        true
    }
    fn create(&mut self, name: &str) -> fx::Creation {
        fx::Creation {
            success: true,
            effect: Some(Box::new(Effect {
                name: name.into(),
                started: name != "stopped",
                calls: self.0.clone(),
            })),
        }
    }
}

#[test]
fn wake_requests_execute_case_insensitive_names_and_six_interleaved_effect_refreshes() {
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    controller.add_effect("A", "smlwake", None, &mut factory);
    controller.add_effect("B", "other", None, &mut factory);
    let mut b = boat();
    b.inherited_heading = [2.0, 3.0, 4.0];
    b.inherited_measured_speed = -0.5;
    b.still_elapsed = 250.0;
    b.update_wakes_with_controller(1.0, 16.0, &mut controller, &mut factory);
    let mut expected = vec![Call::Stop("smlwake".into(), 0)];
    for _ in 0..6 {
        expected.push(Call::Transform("other".into(), fx::Transform::IDENTITY));
        expected.push(Call::Motion("other".into(), [-1.0, -1.5, -2.0]));
    }
    assert_eq!(*factory.0.lock().unwrap(), expected);
    assert_eq!(b.still_elapsed, 251.0);
    factory.0.lock().unwrap().clear();
    b.speed = 0.1;
    b.update_wakes_with_controller(200.0, 16.0, &mut controller, &mut factory);
    assert_eq!(*factory.0.lock().unwrap(), [Call::Start("smlwake".into(), 0)]);
    assert_eq!(b.still_elapsed, 0.0);
    controller.shutdown();
}

#[test]
fn wc4_fx01_owner_uses_inherited_heading_and_measured_speed_after_transform() {
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    controller.add_effect("A", "wake", None, &mut factory);
    let mut b = boat();
    b.inherited_heading = [2.0, 3.0, 4.0];
    b.inherited_measured_speed = -0.5;
    b.speed = 123.0;
    b.position = [99.0; 3];
    let report = b.update_model_effects(&mut controller);
    assert!(report.success);
    assert_eq!(report.transformed, 1);
    assert_eq!(
        *factory.0.lock().unwrap(),
        [
            Call::Transform("wake".into(), fx::Transform::IDENTITY),
            Call::Motion("wake".into(), [-1.0, -1.5, -2.0]),
        ]
    );
}

#[test]
fn wc4_fx02_owner_skips_stopped_effect_and_samples_exact_integer_frame_three() {
    let records = [
        [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [1.0, 2.0, 3.0, 0.0, 1.0, 0.0, 0.0],
        [4.0, 5.0, 6.0, 0.0, 1.0, 0.0, 0.0],
        [7.0, 8.0, 9.0, 0.0, 1.0, 0.0, 0.0],
    ];
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    // Reverse insertion order: the controller still traverses key A before B.
    controller.add_effect("B", "running", Some(&records), &mut factory);
    controller.add_effect("A", "stopped", None, &mut factory);
    let mut b = boat();
    b.inherited_heading = [2.0, 3.0, 4.0];
    b.inherited_measured_speed = -0.5;
    let report = b.update_model_effects(&mut controller);
    assert!(report.success);
    assert_eq!(report.transformed, 1);
    assert!(report.missing_records.is_empty());
    let expected = fx::Transform {
        translation: [7.0, 8.0, 9.0],
        ..fx::Transform::IDENTITY
    };
    assert_eq!(
        *factory.0.lock().unwrap(),
        [
            Call::Transform("running".into(), expected),
            Call::Motion("running".into(), [-1.0, -1.5, -2.0]),
        ]
    );
}

#[test]
fn wc4_fx03_detached_and_attached_empty_controller_updates_succeed_without_effect_calls() {
    let mut controller = fx::ModelRegEffects::new();
    let b = boat();
    for attached in [false, true] {
        if attached {
            controller.set_model_instance(Some(Arc::new(Instance)));
        }
        let report = b.update_model_effects(&mut controller);
        assert!(report.success);
        assert_eq!(report.transformed, 0);
        assert!(report.missing_records.is_empty());
    }
}
