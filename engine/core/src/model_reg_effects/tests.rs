// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Manager,
    Create(String),
    Start(usize, u32),
    Stop(usize, u32),
    Release(usize),
    Transform(usize, Transform, u8, u8),
    Parameter(usize, u32, Vector3, u32),
    Frame,
    InstanceReleased,
}
type Log = Arc<Mutex<Vec<Event>>>;

struct Effect {
    id: usize,
    log: Log,
    started: bool,
}
impl VisualEffect for Effect {
    fn is_started(&self) -> bool {
        self.started
    }
    fn start(&mut self, control: u32) {
        self.log.lock().unwrap().push(Event::Start(self.id, control));
        self.started = true;
    }
    fn stop(&mut self, control: u32) {
        self.log.lock().unwrap().push(Event::Stop(self.id, control));
        if control != 0 {
            self.started = false;
        }
    }
    fn set_transform(&mut self, transform: Transform, flags: u8, version: u8) {
        self.log
            .lock()
            .unwrap()
            .push(Event::Transform(self.id, transform, flags, version));
    }
    fn set_parameter(&mut self, id: u32, value: Vector3, count: u32) {
        self.log
            .lock()
            .unwrap()
            .push(Event::Parameter(self.id, id, value, count));
    }
}
impl Drop for Effect {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(Event::Release(self.id));
    }
}

struct Factory {
    log: Log,
    next: usize,
    manager: bool,
    outcomes: VecDeque<(bool, bool)>,
}
impl Factory {
    fn new() -> Self {
        Self {
            log: Arc::default(),
            next: 0,
            manager: true,
            outcomes: VecDeque::new(),
        }
    }
    fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }
}
impl EffectFactory for Factory {
    fn acquire_manager(&mut self) -> bool {
        self.log.lock().unwrap().push(Event::Manager);
        self.manager
    }
    fn create(&mut self, name: &str) -> Creation {
        self.log.lock().unwrap().push(Event::Create(name.into()));
        let (success, nonnull) = self
            .outcomes
            .pop_front()
            .unwrap_or((name != "missing", name != "missing"));
        let effect = nonnull.then(|| {
            let id = self.next;
            self.next += 1;
            Box::new(Effect {
                id,
                log: self.log.clone(),
                started: false,
            }) as Box<dyn VisualEffect>
        });
        Creation { success, effect }
    }
}

struct Instance {
    regions: Option<Regions>,
    frame: Mutex<InstanceFrame>,
    log: Log,
}
impl ModelInstance for Instance {
    fn regions(&self) -> Option<&Regions> {
        self.regions.as_ref()
    }
    fn frame(&self) -> InstanceFrame {
        self.log.lock().unwrap().push(Event::Frame);
        *self.frame.lock().unwrap()
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(Event::InstanceReleased);
    }
}
const RECORD: [f32; 7] = [1., 2., 3., 0., 0., 0., 1.];
fn instance(regions: Option<Regions>, log: &Log) -> Arc<Instance> {
    Arc::new(Instance {
        regions,
        log: log.clone(),
        frame: Mutex::new(InstanceFrame {
            transform: Transform {
                translation: [100., 0., 200.],
                ..Transform::IDENTITY
            },
            flags: 5,
            animated_frame: Some(0),
        }),
    })
}
fn regions(keys: &[&str]) -> Regions {
    keys.iter().map(|key| ((*key).into(), vec![RECORD])).collect()
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 1e-6, "{actual} != {expected}");
}
fn rotation(actual: Rotation, expected: Rotation) {
    for (a, b) in actual.into_iter().flatten().zip(expected.into_iter().flatten()) {
        close(a, b);
    }
}

#[test]
fn vector48_names_order_failure_and_repeated_discovery() {
    for (key, name) in [
        ("fx_00_planeprop", Some("planeprop")),
        ("fx_00_rotor", Some("rotor")),
        (
            "fx_straight_road_bridge_collumn_foam",
            Some("straight_road_bridge_collumn_foam"),
        ),
        ("fx_7", Some("7")),
        ("fx_1a_b_c", Some("b_c")),
        ("regpoint01", None),
        ("stop_car_x", None),
        ("fxa", None),
        ("FX_00_rotor", None),
        ("fx_", Some("")),
        ("fx_0_", Some("")),
        ("fx_١_b", Some("١_b")),
    ] {
        assert_eq!(effect_name(key), name);
    }
    let mut f = Factory::new();
    let model = instance(
        Some(regions(&["fx_01_a", "fx_00_b", "fx_02_missing", "regpoint01"])),
        &f.log,
    );
    let mut owner = ModelRegEffects::new();
    assert!(owner.instance().is_none());
    assert!(owner.init(&mut f));
    owner.set_model_instance(Some(model));
    assert!(f.take().is_empty()); // Attachment does not discover.
    assert!(owner.init(&mut f));
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Create("b".into()),
            Event::Manager,
            Event::Create("a".into()),
            Event::Manager,
            Event::Create("missing".into())
        ]
    );
    assert_eq!(
        owner.entries().map(|e| e.key()).collect::<Vec<_>>(),
        ["fx_00_b", "fx_01_a"]
    );
    assert!(owner.entries().all(|e| !e.effect().unwrap().is_started()));
    owner.add_effect("manual", "kept", None, &mut f);
    f.take();
    owner.init_effects_list(&mut f);
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Stop(0, 1),
            Event::Release(0),
            Event::Create("b".into()),
            Event::Manager,
            Event::Stop(1, 1),
            Event::Release(1),
            Event::Create("a".into()),
            Event::Manager,
            Event::Create("missing".into())
        ]
    );
    assert!(owner.get_effect("manual").is_some());
    owner.shutdown();
}

#[test]
fn missing_model_container_and_empty_map_do_not_discover() {
    let mut f = Factory::new();
    let mut owner = ModelRegEffects::new();
    for data in [None, Some(Regions::new())] {
        owner.set_model_instance(Some(instance(data, &f.log)));
        f.take();
        owner.init_effects_list(&mut f);
        assert!(f.take().is_empty());
        assert!(owner.add_effect_at_region("absent", "a", &mut f).is_some());
        assert!(owner.records("absent").is_none());
        owner.start_all(1);
        assert_eq!(owner.update().transformed, 1); // Null vectors need no usable model.
        owner.shutdown();
    }
    assert!(owner.add_effect_at_region("detached", "a", &mut f).is_some());
    owner.shutdown();
}

#[test]
fn borrowed_vectors_null_vectors_exact_keys_and_replacement() {
    let mut f = Factory::new();
    let data = [RECORD];
    let model = instance(Some(regions(&["regpoint01"])), &f.log);
    let mut owner = ModelRegEffects::new();
    owner.set_model_instance(Some(model.clone()));
    owner.add_effect_at_region("regpoint01", "Rotor", &mut f);
    assert!(std::ptr::eq(
        owner.records("regpoint01").unwrap(),
        model.regions().unwrap()["regpoint01"].as_slice()
    ));
    assert_eq!(Arc::strong_count(&model), 2);
    assert!(owner.get_effect("REGPOINT01").is_none());
    owner.add_effect("manual", "Rotor", Some(&data), &mut f);
    assert!(std::ptr::eq(owner.records("manual").unwrap(), data.as_slice()));
    f.take();
    f.outcomes.push_back((false, false));
    assert!(owner.add_effect("manual", "new", None, &mut f).is_none());
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Stop(1, 1),
            Event::Release(1),
            Event::Create("new".into())
        ]
    );
    assert!(!owner.entries().any(|e| e.key() == "manual"));
    assert!(!owner.remove("absent"));
    owner.shutdown();
}

#[test]
fn lifetime_same_pointer_shutdown_and_destructor_are_distinct() {
    let mut f = Factory::new();
    let model = instance(Some(regions(&["fx_a", "fx_b"])), &f.log);
    let mut owner = ModelRegEffects::new();
    owner.set_model_instance(Some(model.clone()));
    owner.init(&mut f);
    f.take();
    owner.set_model_instance(Some(model.clone()));
    assert_eq!(Arc::strong_count(&model), 2);
    assert!(owner.instance().is_some());
    assert!(f.take().is_empty());
    drop(model);
    assert!(owner.shutdown());
    assert_eq!(
        f.take(),
        [
            Event::Stop(0, 0),
            Event::Release(0),
            Event::Stop(1, 0),
            Event::Release(1),
            Event::InstanceReleased
        ]
    );
    assert!(owner.shutdown());
    owner.set_model_instance(None);
    assert!(f.take().is_empty());
    owner.set_model_instance(Some(instance(Some(regions(&["fx_z"])), &f.log)));
    owner.init(&mut f);
    f.take();
    owner.set_model_instance(Some(instance(None, &f.log)));
    assert_eq!(
        f.take(),
        [Event::Stop(2, 0), Event::Release(2), Event::InstanceReleased]
    );
    owner.add_effect("a", "a", None, &mut f);
    f.take();
    drop(owner);
    // Native destructor frees nodes and releases instance, but not effects.
    assert_eq!(f.take(), [Event::InstanceReleased]);
}

#[test]
fn controls_query_effect_state_and_preserve_full_words() {
    let mut f = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.add_effect("b", "Rotor", None, &mut f);
    owner.add_effect("a", "ROTOR", None, &mut f);
    f.take();
    owner.stop_all(7);
    assert!(f.take().is_empty());
    owner.start_all(0xdeadbeef);
    owner.start_all(0);
    assert_eq!(f.take(), [Event::Start(1, 0xdeadbeef), Event::Start(0, 0xdeadbeef)]);
    owner.stop_all(0);
    owner.stop_all(0);
    assert_eq!(
        f.take(),
        [
            Event::Stop(1, 0),
            Event::Stop(0, 0),
            Event::Stop(1, 0),
            Event::Stop(0, 0)
        ]
    );
    owner.stop_named("rotor", 2);
    owner.stop_named("rotor", 1);
    assert_eq!(f.take(), [Event::Stop(1, 2), Event::Stop(0, 2)]);
    assert!(owner.start_named("rotor", 0, false, &mut f));
    assert!(owner.start_named("rotor", 9, true, &mut f));
    assert_eq!(
        f.take(),
        [
            Event::Start(1, 0),
            Event::Start(0, 0),
            Event::Start(1, 9),
            Event::Start(0, 9)
        ]
    );
    owner.toggle_named("rotor", 3, true, &mut f);
    owner.toggle_named("rotor", 4, true, &mut f);
    assert_eq!(
        f.take(),
        [
            Event::Stop(1, 3),
            Event::Stop(0, 3),
            Event::Start(1, 4),
            Event::Start(0, 4)
        ]
    );
    owner.shutdown();
}

#[test]
fn named_fallbacks_creation_failure_and_crt_byte_comparison() {
    let mut f = Factory::new();
    let mut owner = ModelRegEffects::new();
    assert!(!owner.start_named("a", 3, false, &mut f));
    assert!(f.take().is_empty());
    assert!(!owner.start_named("missing", 3, true, &mut f));
    assert!(owner.start_named("a", 3, true, &mut f));
    owner.toggle_named("b", 5, true, &mut f);
    owner.recreate_named("c", 6, true, &mut f);
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Create("missing".into()),
            Event::Manager,
            Event::Create("a".into()),
            Event::Start(0, 3),
            Event::Manager,
            Event::Create("b".into()),
            Event::Start(1, 5),
            Event::Manager,
            Event::Manager,
            Event::Create("c".into()),
            Event::Start(2, 6)
        ]
    );
    owner.add_effect("locale", "Ä", None, &mut f);
    assert!(!owner.start_named("ä", 1, false, &mut f)); // No Unicode folding.
    let mut table = owner.lowercase;
    table[0x84] = 0xa4; // Explicit foreign byte table can equate UTF-8 trail bytes.
    owner.set_locale_lowercase(table);
    assert!(owner.start_named("ä", 1, false, &mut f));
    assert!(names_equal(&owner.lowercase, "ABC\0one", "abc\0two"));
    owner.shutdown();
}

#[test]
fn recreate_releases_without_stop_retains_null_entries_and_continues() {
    let mut f = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.add_effect("b", "Rotor", Some(&[]), &mut f);
    owner.add_effect("a", "Rotor", None, &mut f);
    f.take();
    f.manager = false;
    owner.recreate_named("rotor", 8, true, &mut f);
    assert_eq!(f.take(), [Event::Manager]);
    f.manager = true;
    f.outcomes.extend([(false, false), (true, true)]);
    owner.recreate_named("rOtOr", 0x100, true, &mut f);
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Release(1),
            Event::Create("rOtOr".into()),
            Event::Release(0),
            Event::Create("rOtOr".into()),
            Event::Start(2, 0x100)
        ]
    );
    assert_eq!(owner.entries().len(), 2);
    assert!(owner.get_effect("a").is_none());
    assert_eq!(owner.entries().next().unwrap().name(), "Rotor");
    assert_eq!(owner.records("b"), Some([].as_slice()));
    f.outcomes.push_back((false, false));
    owner.recreate_named("Rotor", 1, true, &mut f);
    assert_eq!(
        f.take(),
        [Event::Manager, Event::Release(2), Event::Create("Rotor".into())]
    );
    // Null entries do not count as matches, so fallback now inserts a new key.
    assert!(owner.start_named("Rotor", 7, true, &mut f));
    assert!(owner.get_effect("Rotor").is_some());
    owner.shutdown();
}

#[test]
fn independent_creation_outputs_and_manager_precondition() {
    let mut f = Factory::new();
    let mut owner = ModelRegEffects::new();
    f.outcomes.extend([(true, false), (false, true)]);
    assert!(owner.add_effect("null", "a", None, &mut f).is_none());
    assert_eq!(owner.entries().len(), 1); // true/null keeps metadata.
    assert!(owner.add_effect("false", "b", None, &mut f).is_none());
    assert_eq!(owner.entries().len(), 1);
    assert_eq!(
        f.take(),
        [
            Event::Manager,
            Event::Create("a".into()),
            Event::Manager,
            Event::Create("b".into())
        ]
    );
    owner.start_all(1);
    owner.stop_all(1);
    owner.toggle_named("a", 1, false, &mut f);
    assert!(!owner.start_named("a", 1, false, &mut f));
    assert!(f.take().is_empty());
    f.manager = false;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owner.add_effect("null", "replacement", None, &mut f);
    }))
    .is_err());
    assert_eq!(owner.entries().len(), 1); // Manager acquisition precedes removal.
    owner.shutdown();
}

#[test]
fn vector49_axis_angle_zero_axis_and_extreme_axis_magnitudes() {
    let expected = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
    for axis in [2., f32::MAX, f32::from_bits(1)] {
        let local = Transform::from_record([1., 2., 3., 0., 0., axis, std::f32::consts::FRAC_PI_2]);
        rotation(local.rotation, expected);
        assert_eq!(local.translation, [1., 2., 3.]);
    }
    assert_eq!(Transform::from_record(RECORD).rotation, Transform::IDENTITY.rotation);
    let mut f = Factory::new();
    let model = instance(
        Some(
            [(
                "fx_a".into(),
                vec![[1., 2., 3., 0., 0., 2., std::f32::consts::FRAC_PI_2], RECORD],
            )]
            .into(),
        ),
        &f.log,
    );
    let mut owner = ModelRegEffects::new();
    owner.set_model_instance(Some(model.clone()));
    owner.init(&mut f);
    owner.start_all(1);
    f.take();
    assert_eq!(owner.update().transformed, 1);
    let events = f.take();
    let Event::Transform(0, world, 5, 0) = events[1] else {
        panic!("{events:?}")
    };
    assert_eq!(world.translation, [101., 2., 203.]);
    rotation(world.rotation, expected);
    model.frame.lock().unwrap().animated_frame = Some(1);
    owner.update();
    let events = f.take();
    let Event::Transform(0, world, 5, 0) = events[1] else {
        panic!("{events:?}")
    };
    assert_eq!(world.translation, [101., 2., 203.]);
    rotation(world.rotation, Transform::IDENTITY.rotation);
    owner.shutdown();
}

#[test]
fn transform_flag_shortcuts_composition_scale_and_version() {
    let mut f = Factory::new();
    let model = instance(Some(regions(&["fx_a"])), &f.log);
    let mut owner = ModelRegEffects::new();
    owner.set_model_instance(Some(model.clone()));
    owner.init(&mut f);
    owner.add_effect_at_region("missing", "b", &mut f);
    owner.start_all(1);
    for flags in [0, 0x80] {
        let outer = Transform {
            rotation: [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]],
            translation: [100., 0., 200.],
            scale: 2.,
        };
        {
            let mut frame = model.frame.lock().unwrap();
            frame.flags = flags;
            frame.transform = outer;
        }
        f.take();
        owner.update();
        let events = f.take();
        let local = Transform::from_record(RECORD);
        assert_eq!(
            events[1],
            Event::Transform(0, if flags == 0 { local } else { outer.compose(local) }, flags | 5, 0)
        );
        assert_eq!(events[2], Event::Transform(1, outer, flags, 0));
        if flags != 0 {
            assert_eq!(outer.compose(local).translation, [96., 2., 206.]);
            assert_eq!(outer.compose(local).scale, 2.);
        }
    }
    owner.shutdown();
}

#[test]
fn motion_update_order_started_gate_and_unchecked_record_domain() {
    let mut f = Factory::new();
    let model = instance(Some([("fx_a".into(), vec![RECORD, [9.; 7]])].into()), &f.log);
    let mut owner = ModelRegEffects::new();
    assert!(owner.update().success);
    owner.set_model_instance(Some(model.clone()));
    assert!(owner.update().success);
    assert!(f.take().is_empty()); // Empty update does not query instance.
    owner.init(&mut f);
    owner.add_effect("empty", "b", Some(&[]), &mut f);
    owner.add_effect_at_region("null", "c", &mut f);
    f.take();
    assert_eq!(owner.update_with_motion(999, [2., -3., 4.], 2.5).transformed, 0);
    assert_eq!(f.take(), [Event::Frame]);
    owner.start_all(1);
    model.frame.lock().unwrap().animated_frame = Some(9);
    f.take();
    let report = owner.update_with_motion(999, [2., -3., 4.], 2.5);
    assert!(report.success);
    assert_eq!(report.transformed, 1);
    assert_eq!(
        report.missing_records,
        [
            MissingRecord {
                key: "empty".into(),
                frame: 9,
                record_count: 0
            },
            MissingRecord {
                key: "fx_a".into(),
                frame: 9,
                record_count: 2
            },
        ]
    );
    assert_eq!(
        f.take(),
        [
            Event::Frame,
            Event::Transform(2, model.frame.lock().unwrap().transform, 5, 0),
            Event::Parameter(2, 15, [5., -7.5, 10.], 1)
        ]
    );
    model.frame.lock().unwrap().animated_frame = None;
    f.take();
    assert_eq!(owner.update_with_motion(0, [2., -3., 4.], 2.5).transformed, 2);
    let events = f.take();
    assert!(matches!(events[1], Event::Transform(0, _, 5, 0)));
    assert_eq!(events[2], Event::Parameter(0, 15, [5., -7.5, 10.], 1));
    owner.stop_all(0); // Deferred stop leaves IsStarted true.
    assert_eq!(owner.update().transformed, 2);
    owner.stop_all(1);
    assert_eq!(owner.update().transformed, 0);
    owner.shutdown();
}

#[test]
fn noncommuting_rotations_and_effect_placement_order() {
    let instance = Transform {
        rotation: [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]],
        translation: [10., 20., 30.],
        scale: 2.,
    };
    let local = Transform::from_record([1., 2., 3., 1., 0., 0., std::f32::consts::FRAC_PI_2]);
    let incoming = instance.compose(local);
    rotation(incoming.rotation, [[0., 0., 1.], [1., 0., 0.], [0., 1., 0.]]);
    assert_eq!(incoming.translation, [6., 22., 36.]);
    let placement = Transform {
        translation: [100., 0., 0.],
        scale: 3.,
        ..Transform::IDENTITY
    };
    assert_eq!(placement.compose(incoming).translation, [118., 66., 108.]);
}
