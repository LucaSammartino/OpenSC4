// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Create(String),
    Start(usize, bool, u32),
    Stop(usize, bool),
    Transform(usize, Transform, u8),
    Parameter(usize, u32, f32),
    Release(usize),
    ModelReleased,
}

type Log = Arc<Mutex<Vec<Event>>>;

struct Effect {
    id: usize,
    log: Log,
}

impl VisualEffect for Effect {
    fn start(&mut self, control: bool, extra: u32) {
        self.log.lock().unwrap().push(Event::Start(self.id, control, extra));
    }

    fn stop(&mut self, control: bool) {
        self.log.lock().unwrap().push(Event::Stop(self.id, control));
    }

    fn set_transform(&mut self, transform: Transform, flags: u8) {
        self.log
            .lock()
            .unwrap()
            .push(Event::Transform(self.id, transform, flags));
    }

    fn set_parameter(&mut self, parameter: u32, value: f32) {
        self.log
            .lock()
            .unwrap()
            .push(Event::Parameter(self.id, parameter, value));
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(Event::Release(self.id));
    }
}

struct Factory {
    log: Log,
    fail: bool,
    next: usize,
}

impl Factory {
    fn new() -> Self {
        Self {
            log: Arc::default(),
            fail: false,
            next: 0,
        }
    }

    fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }
}

impl EffectFactory for Factory {
    fn create(&mut self, name: &str) -> Option<Box<dyn VisualEffect>> {
        self.log.lock().unwrap().push(Event::Create(name.to_owned()));
        if self.fail || name == "missing" {
            return None;
        }
        let id = self.next;
        self.next += 1;
        Some(Box::new(Effect {
            id,
            log: Arc::clone(&self.log),
        }))
    }
}

struct Instance {
    regions: Regions,
    frame: Mutex<InstanceFrame>,
    log: Log,
}

impl ModelInstance for Instance {
    fn regions(&self) -> &Regions {
        &self.regions
    }

    fn frame(&self) -> InstanceFrame {
        *self.frame.lock().unwrap()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(Event::ModelReleased);
    }
}

const IDENTITY_RECORD: [f32; 7] = [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0];

fn instance(regions: Regions, log: &Log) -> Arc<Instance> {
    Arc::new(Instance {
        regions,
        frame: Mutex::new(InstanceFrame {
            transform: Transform {
                translation: [100.0, 0.0, 200.0],
                ..Transform::IDENTITY
            },
            animated_frame: Some(0),
        }),
        log: Arc::clone(log),
    })
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 1e-6, "{actual} != {expected}");
}

fn rotation(actual: Rotation, expected: Rotation) {
    for (actual, expected) in actual.into_iter().flatten().zip(expected.into_iter().flatten()) {
        close(actual, expected);
    }
}

#[test]
fn vector48_names_and_edge_cases() {
    for (key, expected) in [
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
        ("fx_", Some("")),
        ("fx_0_", Some("")),
        ("fx_a_b", Some("a_b")),
        ("fx_\u{0661}_b", Some("\u{0661}_b")),
        ("FX_00_rotor", None), // Parser, not owner, owns lower-casing.
    ] {
        assert_eq!(effect_name(key), expected, "{key}");
    }
}

#[test]
fn name_sorted_creation_failed_entry_and_stopped_initial_state() {
    let mut factory = Factory::new();
    let model = instance(
        [
            ("fx_01_a", "a"),
            ("fx_00_b", "b"),
            ("fx_02_missing", "missing"),
            ("regpoint01", "ignored"),
        ]
        .into_iter()
        .map(|(key, _)| (key.to_owned(), vec![IDENTITY_RECORD]))
        .collect(),
        &factory.log,
    );
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model), &mut factory);
    assert_eq!(
        factory.take(),
        [
            Event::Create("b".into()),
            Event::Create("a".into()),
            Event::Create("missing".into())
        ]
    );
    assert_eq!(
        owner.entries().map(EffectEntry::key).collect::<Vec<_>>(),
        ["fx_00_b", "fx_01_a"]
    );
    assert!(owner.entries().all(|entry| !entry.started()));
    assert_eq!(owner.update(), UpdateReport::default());
    assert!(factory.take().is_empty());
}

#[test]
fn vector49_axis_angle_normalisation_and_zero_axis_choice() {
    let mut factory = Factory::new();
    let model = instance(
        [(
            "fx_00_rotor".into(),
            vec![
                [1.0, 2.0, 3.0, 0.0, 0.0, 2.0, std::f32::consts::FRAC_PI_2],
                IDENTITY_RECORD,
            ],
        )]
        .into(),
        &factory.log,
    );
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model.clone()), &mut factory);
    owner.start_all(false, 0);
    factory.take();
    assert_eq!(owner.update().transformed, 1);
    let Event::Transform(0, actual, flags) = factory.take()[0] else {
        panic!("missing transform")
    };
    assert_eq!(flags, 5);
    assert_eq!(actual.translation, [101.0, 2.0, 203.0]);
    rotation(actual.rotation, [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
    model.frame.lock().unwrap().animated_frame = Some(1);
    owner.update();
    let Event::Transform(_, zero, _) = factory.take()[0] else {
        panic!("missing transform")
    };
    assert_eq!(zero.translation, [101.0, 2.0, 203.0]);
    assert_eq!(zero.rotation, Transform::IDENTITY.rotation);
}

#[test]
fn noncommuting_model_local_and_effect_placement_are_in_correct_order() {
    // Independent geometry: model +90° about X, local +90° about Z.
    // Combined basis columns become +Z, -X, -Y. Reverse multiplication differs.
    let model = Transform {
        rotation: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
        translation: [10.0, 20.0, 30.0],
        scale: 2.0,
    };
    let local = Transform::from_record([1.0, 2.0, 3.0, 0.0, 0.0, 7.0, std::f32::consts::FRAC_PI_2]);
    let world = model.compose(local);
    assert_eq!(world.translation, [12.0, 14.0, 34.0]);
    rotation(world.rotation, [[0.0, -1.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]]);
    close(world.scale, 2.0);
    // Independent outer placement +90° about Y with scale 3.
    let placement = Transform {
        rotation: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
        translation: [-5.0, 4.0, 9.0],
        scale: 3.0,
    };
    let placed = placement.compose(world);
    assert_eq!(placed.translation, [97.0, 46.0, -27.0]);
    rotation(placed.rotation, [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]]);
    close(placed.scale, 6.0);
}

#[test]
fn owner_update_combines_rotated_scaled_instance_with_selected_local_record() {
    let mut factory = Factory::new();
    let record = [1.0, 2.0, 3.0, 0.0, 0.0, 7.0, std::f32::consts::FRAC_PI_2];
    let model = instance([("fx_rotor".into(), vec![record])].into(), &factory.log);
    model.frame.lock().unwrap().transform = Transform {
        rotation: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
        translation: [10.0, 20.0, 30.0],
        scale: 2.0,
    };
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model), &mut factory);
    owner.start_all(false, 0);
    factory.take();
    assert_eq!(owner.update().transformed, 1);
    let Event::Transform(_, actual, 5) = factory.take()[0] else {
        panic!("missing transform")
    };
    assert_eq!(actual.translation, [12.0, 14.0, 34.0]);
    rotation(actual.rotation, [[0.0, -1.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]]);
    assert_eq!(actual.scale, 2.0);
}

#[test]
fn oblique_axis_is_normalised_and_zero_angle_leaves_identity() {
    // A half-turn around (1,1,0) swaps X/Y and reverses Z, independently
    // distinguishing axis normalisation from treating the record as a quaternion.
    let record = [0.0, 0.0, 0.0, 3.0, 3.0, 0.0, std::f32::consts::PI];
    rotation(
        Transform::from_record(record).rotation,
        [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
    );
    let mut zero_angle = record;
    zero_angle[6] = 0.0;
    assert_eq!(
        Transform::from_record(zero_angle).rotation,
        Transform::IDENTITY.rotation
    );
}

#[test]
fn frame_selection_never_interpolates_clamps_or_uses_file_playback_fields() {
    let mut factory = Factory::new();
    let mut second = IDENTITY_RECORD;
    second[0] = 9.0;
    let model = instance(
        [
            ("fx_a".into(), vec![IDENTITY_RECORD, second]),
            ("fx_empty".into(), vec![]),
        ]
        .into(),
        &factory.log,
    );
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model.clone()), &mut factory);
    owner.start_all(true, 123);
    factory.take();
    model.frame.lock().unwrap().animated_frame = Some(1);
    let report = owner.update();
    assert_eq!(report.transformed, 1);
    assert_eq!(
        report.missing_records,
        [MissingRecord {
            key: "fx_empty".into(),
            frame: 1,
            record_count: 0
        }]
    );
    let Event::Transform(_, actual, _) = factory.take()[0] else {
        panic!("missing transform")
    };
    assert_eq!(actual.translation, [109.0, 2.0, 203.0]);
    model.frame.lock().unwrap().animated_frame = Some(5);
    let report = owner.update();
    assert_eq!(report.transformed, 0);
    assert_eq!(report.missing_records.len(), 2);
    assert_eq!(report.missing_records[0].frame, 5);
    assert!(factory.take().is_empty());
    model.frame.lock().unwrap().animated_frame = None;
    assert_eq!(owner.update().transformed, 1);
    let Event::Transform(_, actual, _) = factory.take()[0] else {
        panic!("missing transform")
    };
    assert_eq!(actual.translation, [101.0, 2.0, 203.0]);
}

#[test]
fn named_controls_match_all_effect_names_and_forward_opaque_arguments() {
    let mut factory = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.add_effect("b", "RoToR", &[], &mut factory);
    owner.add_effect("a", "rotor", &[], &mut factory);
    owner.add_effect("rotor", "other", &[], &mut factory);
    factory.take();
    assert_eq!(owner.start_named("ROTOR", true, 0xdeadbeef), 2);
    assert_eq!(
        factory.take(),
        [Event::Start(1, true, 0xdeadbeef), Event::Start(0, true, 0xdeadbeef)]
    );
    assert_eq!(owner.stop_named("rOtOr", false), 2);
    assert_eq!(factory.take(), [Event::Stop(1, false), Event::Stop(0, false)]);
    assert_eq!(owner.start_named("missing", false, 9), 0);
    owner.start_all(false, 7);
    assert_eq!(
        factory.take(),
        [
            Event::Start(1, false, 7),
            Event::Start(0, false, 7),
            Event::Start(2, false, 7)
        ]
    );
    owner.stop_all(true);
    assert_eq!(
        factory.take(),
        [Event::Stop(1, true), Event::Stop(0, true), Event::Stop(2, true)]
    );
    assert!(owner.entries().all(|entry| !entry.started()));
}

#[test]
fn stopped_effect_has_no_transform_then_restarts_and_motion_reaches_every_entry() {
    let mut factory = Factory::new();
    let model = instance([("fx_a".into(), vec![IDENTITY_RECORD])].into(), &factory.log);
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model), &mut factory);
    factory.take();
    assert_eq!(owner.update_with_motion(2.5, -1.0).transformed, 0);
    assert_eq!(factory.take(), [Event::Parameter(0, 15, -2.5)]);
    owner.start_named("a", false, 0);
    factory.take();
    assert_eq!(owner.update().transformed, 1);
    factory.take();
    owner.stop_named("a", false);
    factory.take();
    assert_eq!(owner.update().transformed, 0);
    assert!(factory.take().is_empty());
    owner.start_named("a", false, 0);
    factory.take();
    assert_eq!(owner.update_with_motion(3.0, 0.5).transformed, 1);
    let events = factory.take();
    assert!(matches!(events[0], Event::Transform(0, _, 5)));
    assert_eq!(events[1], Event::Parameter(0, 15, 1.5));
}

#[test]
fn replacement_stops_releases_before_factory_and_failure_removes_entry() {
    let mut factory = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.add_effect("key", "first", &[IDENTITY_RECORD], &mut factory);
    owner.start_all(false, 0);
    factory.take();
    owner.add_effect("key", "second", &[], &mut factory);
    assert_eq!(
        factory.take(),
        [Event::Stop(0, false), Event::Release(0), Event::Create("second".into())]
    );
    assert!(!owner.entries().next().unwrap().started());
    factory.fail = true;
    assert!(owner.add_effect("key", "third", &[], &mut factory).is_none());
    assert_eq!(
        factory.take(),
        [Event::Stop(1, false), Event::Release(1), Event::Create("third".into())]
    );
    assert!(owner.get_effect("key").is_none());
    assert_eq!(owner.entries().len(), 0);
    assert!(!owner.remove("key"));
}

#[test]
fn region_lookup_copies_records_and_does_not_require_fx_prefix() {
    let mut factory = Factory::new();
    let model = instance([("regpoint01".into(), vec![IDENTITY_RECORD])].into(), &factory.log);
    let mut owner = ModelRegEffects::new();
    owner.attach(Some(model.clone()), &mut factory);
    assert_eq!(owner.entries().len(), 0);
    assert!(owner.add_effect_at_region("absent", "rotor", &mut factory).is_none());
    assert!(factory.take().is_empty());
    assert!(owner
        .add_effect_at_region("regpoint01", "rotor", &mut factory)
        .is_some());
    assert!(owner.get_effect("REGPOINT01").is_none());
    assert!(owner.get_effect("regpoint01").is_some());
    assert_eq!(owner.entries().next().unwrap().records(), &[IDENTITY_RECORD]);
    let mut records = vec![IDENTITY_RECORD];
    owner.add_effect("manual", "rotor", &records, &mut factory);
    records[0][0] = 999.0;
    assert_eq!(owner.entries().next().unwrap().records()[0][0], 1.0);
}

#[test]
fn attachment_acquires_one_reference_getters_borrow_and_drop_releases_in_order() {
    let mut factory = Factory::new();
    let model = instance([("fx_a".into(), vec![IDENTITY_RECORD])].into(), &factory.log);
    let weak = Arc::downgrade(&model);
    let mut owner = ModelRegEffects::new();
    assert_eq!(Arc::strong_count(&model), 1);
    owner.attach(Some(model.clone()), &mut factory);
    assert_eq!(Arc::strong_count(&model), 2);
    assert!(owner.instance().is_some());
    assert!(owner.get_effect("fx_a").is_some());
    assert_eq!(Arc::strong_count(&model), 2);
    drop(model);
    factory.take();
    drop(owner);
    assert_eq!(
        factory.take(),
        [Event::Stop(0, false), Event::Release(0), Event::ModelReleased]
    );
    assert!(weak.upgrade().is_none());
}

#[test]
fn reattach_and_detach_clear_old_effects_before_releasing_old_model() {
    let mut factory = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.attach(
        Some(instance([("fx_a".into(), vec![IDENTITY_RECORD])].into(), &factory.log)),
        &mut factory,
    );
    let next = instance([("fx_b".into(), vec![IDENTITY_RECORD])].into(), &factory.log);
    factory.take();
    owner.attach(Some(next), &mut factory);
    assert_eq!(
        factory.take(),
        [
            Event::Stop(0, false),
            Event::Release(0),
            Event::ModelReleased,
            Event::Create("b".into())
        ]
    );
    owner.attach(None, &mut factory);
    assert_eq!(
        factory.take(),
        [Event::Stop(1, false), Event::Release(1), Event::ModelReleased]
    );
    assert!(owner.instance().is_none());
    owner.init(&mut factory);
    assert_eq!(owner.update(), UpdateReport::default());
    assert!(owner.add_effect_at_region("fx_b", "b", &mut factory).is_none());
    assert!(factory.take().is_empty());
}

#[test]
fn init_replaces_filtered_entries_but_does_not_clear_unrelated_manual_entry() {
    let mut factory = Factory::new();
    let mut owner = ModelRegEffects::new();
    owner.attach(
        Some(instance([("fx_a".into(), vec![IDENTITY_RECORD])].into(), &factory.log)),
        &mut factory,
    );
    owner.add_effect("manual", "b", &[], &mut factory);
    factory.take();
    owner.init(&mut factory);
    assert_eq!(
        factory.take(),
        [Event::Stop(0, false), Event::Release(0), Event::Create("a".into())]
    );
    assert_eq!(owner.entries().len(), 2);
    owner.clear();
    assert_eq!(
        factory.take(),
        [
            Event::Stop(2, false),
            Event::Release(2),
            Event::Stop(1, false),
            Event::Release(1)
        ]
    );
    owner.clear();
    assert!(factory.take().is_empty());
}

#[test]
fn incoming_nan_orientation_cannot_contaminate_composed_translation() {
    let incoming = Transform {
        rotation: [[f32::NAN; 3]; 3],
        translation: [1.0, 2.0, 3.0],
        scale: 2.0,
    };
    let outer = Transform {
        translation: [100.0, 0.0, 200.0],
        ..Transform::IDENTITY
    };
    let composed = outer.compose(incoming);
    assert_eq!(composed.translation, [101.0, 2.0, 203.0]);
    assert!(composed.rotation.into_iter().flatten().all(f32::is_nan));
}

#[test]
fn owner_and_immutable_getters_are_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<ModelRegEffects>();
    check::<EffectEntry>();
}
