// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::tokens;
use super::*;
use std::collections::VecDeque;

#[derive(Default)]
pub(super) struct Host {
    pub(super) expected: VecDeque<(Call, Result<Reply>)>,
    pub(super) calls: Vec<Call>,
}
impl Host {
    pub(super) fn returning(calls: Vec<(Call, Reply)>) -> Self {
        Self {
            expected: calls.into_iter().map(|(c, r)| (c, Ok(r))).collect(),
            calls: Vec::new(),
        }
    }
    pub(super) fn done(&self) {
        assert!(self.expected.is_empty(), "{:?}", self.expected);
    }
}
impl Services for Host {
    fn call(&mut self, call: Call) -> Result<Reply> {
        self.calls.push(call.clone());
        let (expected, reply) = self.expected.pop_front().expect("unexpected foreign call");
        assert_eq!(call, expected);
        reply
    }
}
pub(super) fn bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}
fn u(s: &str, v: u32) -> (Call, Reply) {
    (Call::Uint(bytes(s)), Reply::Uint(v))
}
fn i(s: &str, v: i32) -> (Call, Reply) {
    (Call::Int(bytes(s)), Reply::Int(v))
}
fn f(s: &str, v: f64) -> (Call, Reply) {
    (Call::Float(bytes(s)), Reply::Float(v))
}

#[test]
fn ec1_ec2_ec3_ec4_renderer_precedence_and_arity() {
    let mut c = Commands {
        renderer: Some(Identity(1)),
        ..Commands::default()
    };
    for (category, prefix, conversion, value) in [
        (
            Category::Bool,
            vec![],
            (Call::Bool(bytes("value")), Reply::Bool(true)),
            Reply::Bool(true),
        ),
        (
            Category::Int,
            vec![(
                Call::PropertyIndex(Identity(1), Category::Bool, bytes("property")),
                Reply::Int(-1),
            )],
            i("value", -9),
            Reply::Int(-9),
        ),
    ] {
        let mut calls = prefix;
        calls.push((
            Call::PropertyIndex(Identity(1), category, bytes("property")),
            Reply::Int(7),
        ));
        calls.push(conversion);
        calls.push((
            Call::SetProperty(Identity(1), category, 7, Box::new(value)),
            Reply::Void,
        ));
        let mut h = Host::returning(calls);
        c.render_property(&tokens(&["renderprop", "property", "value"]), &mut h)
            .unwrap();
        h.done();
    }
    let mut h = Host::default();
    assert!(c.render_property(&tokens(&["renderprop", "property"]), &mut h).is_err());
    assert!(h.calls.is_empty());
    let mut h = Host::returning(
        [Category::Bool, Category::Int, Category::Float]
            .into_iter()
            .map(|category| {
                (
                    Call::PropertyIndex(Identity(1), category, bytes("property")),
                    Reply::Int(-1),
                )
            })
            .collect(),
    );
    assert_eq!(
        c.render_property(&tokens(&["renderprop", "property", "value"]), &mut h),
        Err(Error::Parse("Unknown render property".into()))
    );
    h.done();
}
#[test]
fn ec5_through_ec15_identifier_order_and_exception_prefixes() {
    let mut c = Commands::default();
    let mut h = Host::returning(vec![u("id", 0xfedcba98)]);
    c.enter_group(&tokens(&["effectGroup", "id", "surplus"]), &mut h)
        .unwrap();
    assert_eq!(c.group, 0xfedcba98);
    assert!(c.enter_group(&tokens(&["effectGroup"]), &mut h).is_err());
    h.done();
    let mut h = Host::returning(vec![u("7", 7)]);
    c.declare_id(0, &tokens(&["textureID", "Smoke", "7"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.id_maps[0][b"smoke".as_slice()], 7);
    let mut h = Host::default();
    h.expected
        .push_back((Call::Uint(bytes("bad")), Err(Error::Foreign("uint".into()))));
    assert!(c
        .declare_id(1, &tokens(&["modelID", "smoke", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.id_maps[1][b"smoke".as_slice()], 0);
    c.id_maps[1].insert(bytes("smoke"), 9);
    h.expected
        .push_back((Call::Uint(bytes("bad")), Err(Error::Foreign("uint".into()))));
    assert!(c
        .declare_id(1, &tokens(&["modelID", "smoke", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.id_maps[1][b"smoke".as_slice()], 9);
    for (name, value) in [("Bell", 8), ("BELL", 10)] {
        h.expected.push_back((Call::Uint(bytes("n")), Ok(Reply::Uint(value))));
        c.declare_id(2, &tokens(&["soundID", name, "n"]), &mut Ascii, &mut h)
            .unwrap();
    }
    assert_eq!(c.id_maps[2].len(), 1);
    assert_eq!(c.id_maps[2][b"bell".as_slice()], 10);
    for map in 0..4 {
        assert!(c.declare_id(map, &tokens(&["id", "one"]), &mut Ascii, &mut h).is_err());
        assert!(c
            .declare_id(map, &tokens(&["id", "one", "two", "three"]), &mut Ascii, &mut h)
            .is_err());
    }
    c.collection = Some(Identity(1));
    let mut h = Host::returning(vec![
        u("G", 0xfedcba98),
        u("I", 0x89abcdef),
        (
            Call::EffectId(Identity(1), bytes("smoke"), 0xfedcba98, 0x89abcdef),
            Reply::Void,
        ),
    ]);
    c.effect_id(&tokens(&["effectID", "G", "I", "Smoke"]), false, &mut Ascii, &mut h)
        .unwrap();
    h.done();
    c.collection = None;
    let mut h = Host::returning(vec![u("I", 7)]);
    c.effect_id(&tokens(&["instance", "I", "Smoke"]), true, &mut Ascii, &mut h)
        .unwrap();
    h.done();
    let mut h = Host::returning(vec![
        u("9", 9),
        u("7", 7),
        (Call::EffectId(Identity(1), bytes("smoke"), 9, 7), Reply::Void),
        u("M", 42),
        (Call::Message(Identity(1), 42, bytes("MiXeD")), Reply::Void),
    ]);
    c.enter_group(&tokens(&["effectGroup", "9"]), &mut h).unwrap();
    // effectGroup's end hook is return-only: group remains 9.
    c.collection = Some(Identity(1));
    c.effect_id(&tokens(&["instance", "7", "Smoke"]), true, &mut Ascii, &mut h)
        .unwrap();
    c.message(
        &tokens(&["messageTrigger", "M", "MiXeD", "ignored"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
}
#[test]
fn ec16_through_ec24_resource_gates_order_and_retained_error_state() {
    let mut c = Commands::default();
    let mut h = Host::default();
    c.enter_resource(&tokens(&["effectsResource", "bad", "name"]), &mut h)
        .unwrap();
    c.exit_resource(&mut h).unwrap();
    c.resource = Some(Identity(9));
    assert_eq!(
        c.enter_resource(&tokens(&["effectsResource"]), &mut h),
        Err(Error::Parse("Wrong number of arguments".into()))
    );
    c.resource = None;
    c.resource_enabled = true;
    c.resource_manager = Some(Identity(1));
    let mut h = Host::returning(vec![
        u("7", 7),
        (
            Call::CreateResource(Identity(1), key(7)),
            Reply::StatusObject(false, None),
        ),
    ]);
    c.enter_resource(&tokens(&["effectsResource", "7", "Name"]), &mut h)
        .unwrap();
    assert_eq!(c.resource_key, [0; 3]);
    h.done();
    c.manager = Some(Identity(2));
    c.collection = Some(Identity(3));
    let mut h = Host::returning(vec![
        u("7", 7),
        (
            Call::CreateResource(Identity(1), key(7)),
            Reply::StatusObject(true, Some(Identity(4))),
        ),
        (Call::NameResource(Identity(4), bytes("Name")), Reply::Void),
        (Call::QueryCollection(Identity(4)), Reply::Object(Some(Identity(5)))),
        (Call::Retain(Identity(5)), Reply::Void),
        (Call::Release(Identity(3)), Reply::Void),
        (Call::Release(Identity(5)), Reply::Void),
        (
            Call::Segment(Identity(1), DIRECTORY_GROUP),
            Reply::StatusObject(true, Some(Identity(6))),
        ),
        (
            Call::SaveResource(Identity(1), key(7), Some(Identity(6))),
            Reply::Bool(true),
        ),
        (Call::QueryCollection(Identity(2)), Reply::Object(Some(Identity(7)))),
        (Call::Retain(Identity(7)), Reply::Void),
        (Call::Release(Identity(5)), Reply::Void),
        (Call::Release(Identity(7)), Reply::Void),
        (Call::Release(Identity(4)), Reply::Void),
        (Call::Release(Identity(6)), Reply::Void),
    ]);
    c.enter_resource(&tokens(&["effectsResource", "7", "Name"]), &mut h)
        .unwrap();
    c.exit_resource(&mut h).unwrap();
    h.done();
    assert_eq!(c.collection, Some(Identity(7)));
    assert_eq!(c.resource, None);
    assert_eq!(c.resource_key, key(7));
    c.resource = Some(Identity(4));
    for save in [false, true] {
        let mut calls = vec![(
            Call::Segment(Identity(1), DIRECTORY_GROUP),
            Reply::StatusObject(save, Some(Identity(6))),
        )];
        if save {
            calls.push((
                Call::SaveResource(Identity(1), key(7), Some(Identity(6))),
                Reply::Bool(false),
            ));
        }
        let mut h = Host::returning(calls);
        assert!(c.exit_resource(&mut h).is_err());
        h.done();
        assert_eq!(c.resource, Some(Identity(4)));
        assert_eq!(c.collection, Some(Identity(7)));
    }
    c.resource_manager = None;
    let mut h = Host::returning(vec![u("7", 7)]);
    c.load_resource(&tokens(&["loadResource", "7", "extra"]), &mut Ascii, &mut h)
        .unwrap();
    h.done();
    c.resource_manager = Some(Identity(1));
    for status in [false, true] {
        let mut calls = vec![
            u("7", 7),
            (
                Call::LoadResource(Identity(1), key(7)),
                Reply::StatusObject(status, Some(Identity(4))),
            ),
        ];
        if status {
            calls.extend([
                (Call::ImportResource(Identity(4), Some(Identity(7))), Reply::Void),
                (Call::Release(Identity(4)), Reply::Void),
            ]);
        }
        let mut h = Host::returning(calls);
        assert_eq!(
            c.load_resource(&tokens(&["loadResource", "7", "extra"]), &mut Ascii, &mut h)
                .is_ok(),
            status
        );
        h.done();
    }
}

#[test]
fn ec25_through_ec30_camera_expansion_switch_order_span_and_null_collection() {
    for (input, replies, expected) in [
        (
            vec!["camera", "160"],
            vec![f("160", 160.)],
            Camera {
                distances: vec![160., 80., 40., 20., 10.],
                first: 100.,
                span: 1.,
                size: 4.,
                side_swipe: 7.,
            },
        ),
        (
            vec!["camera", "160", "90", "30"],
            vec![f("160", 160.), f("90", 90.), f("30", 30.)],
            Camera {
                distances: vec![160., 90., 30.],
                first: 100.,
                span: 1.,
                size: 4.,
                side_swipe: 7.,
            },
        ),
        (
            vec![
                "camera",
                "160",
                "-sideSwipe",
                "2",
                "-size",
                "3",
                "-parallax",
                "10",
                "13.75",
            ],
            vec![f("160", 160.), f("10", 10.), f("13.75", 13.75), f("3", 3.), f("2", 2.)],
            Camera {
                distances: vec![160., 80., 40., 20., 10.],
                first: 10.,
                span: 3.,
                size: 3.,
                side_swipe: 2.,
            },
        ),
        (
            vec!["camera", "160", "-parallax", "10", "9.5"],
            vec![f("160", 160.), f("10", 10.), f("9.5", 9.5)],
            Camera {
                distances: vec![160., 80., 40., 20., 10.],
                first: 10.,
                span: 0.,
                size: 4.,
                side_swipe: 7.,
            },
        ),
    ] {
        let mut c = Commands {
            collection: Some(Identity(1)),
            ..Commands::default()
        };
        let mut calls = replies;
        calls.push((Call::Camera(Identity(1), expected), Reply::Void));
        let mut h = Host::returning(calls);
        c.camera(&mut tokens(&input), &mut Ascii, &mut h).unwrap();
        h.done();
    }
    let mut c = Commands::default();
    let mut h = Host::returning(vec![f("160", 160.)]);
    let mut t = tokens(&["camera", "160", "-parallax", "10"]);
    assert!(c.camera(&mut t, &mut Ascii, &mut h).is_err());
    assert_eq!(t[2].bytes, b"+parallax");
    h.done();
    let mut h = Host::returning(vec![f("160", 160.)]);
    c.camera(&mut tokens(&["camera", "160"]), &mut Ascii, &mut h).unwrap();
    h.done();
}

#[test]
fn ec31_through_ec42_priorities_visual_reset_flags_and_failure_prefixes() {
    let mut c = Commands::default();
    let mut h = Host::returning(vec![i("-4", -4), i("9", 9), i("9", 9)]);
    for input in [
        vec!["setPriority", "Smoke", "-4"],
        vec!["setPriority", "Smoke", "9", "Other"],
        vec!["setPriority", "smoke", "9", "Other", "surplus"],
    ] {
        c.priority(&tokens(&input), &mut Ascii, &mut h).unwrap();
    }
    h.done();
    assert_eq!(
        c.pending_priorities,
        vec![
            Priority {
                name: bytes("Smoke"),
                priority: -4,
                rename: vec![],
                used: false
            },
            Priority {
                name: bytes("smoke"),
                priority: 9,
                rename: bytes("Other"),
                used: false
            }
        ]
    );
    h.expected
        .push_back((Call::Int(bytes("bad")), Err(Error::Foreign("int".into()))));
    assert!(c
        .priority(&tokens(&["setPriority", "New", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.pending_priorities.len(), 2);
    c.name = bytes("Outer");
    assert!(c.enter_visual(&mut tokens(&["effect"]), &mut Ascii, &mut h).is_err());
    assert_eq!(c.name, b"Outer");
    c.name.clear();
    c.visual.description.flags = 0xffff;
    c.word_4a8 = 77;
    c.enter_visual(&mut tokens(&["effect", "Name", "ignored"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.name, b"Name");
    assert_eq!(c.visual, Visual::default());
    assert_eq!(c.word_4a8, 77);
    c.name.clear();
    let mut h = Host::returning(vec![u("10", 10), u("20", 20), u("30", 30)]);
    c.enter_visual(
        &mut tokens(&[
            "effect",
            "Name",
            "-manualRestart",
            "-applyCursor",
            "-viewRelative",
            "-startMessage",
            "10",
            "20",
            "30",
            "40",
        ]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.visual.description.flags, 0x121);
    assert_eq!(c.visual.description.extra, [10, 20, 30]);
    assert_eq!(c.visual.message_known, [true; 3]);
    h.done();
    c.name.clear();
    let mut h = Host::returning(vec![u("10", 10)]);
    h.expected
        .push_back((Call::Uint(bytes("bad")), Err(Error::Foreign("uint".into()))));
    let mut t = tokens(&[
        "effect",
        "Name",
        "-rigid",
        "-startMessage",
        "10",
        "bad",
        "-manualRestart",
    ]);
    c.word_4a0 = 10;
    assert!(c.enter_visual(&mut t, &mut Ascii, &mut h).is_err());
    assert_eq!(c.name, b"Name");
    assert_eq!(c.visual.description.flags, 8);
    assert_eq!(c.visual.description.extra[0], 10);
    assert_eq!(c.visual.message_known, [true, false, false]);
    assert_eq!(c.word_4a0, 10);
    assert_eq!(t[6].bytes, b"-manualRestart");
    c.name.clear();
    let mut h = Host::returning(vec![i("0", 0)]);
    assert!(c
        .enter_visual(&mut tokens(&["effect", "Name", "-priority", "0"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.visual.description.priority, 1);
    c.name.clear();
    c.pending_priorities.push(Priority {
        name: bytes("Name"),
        priority: 0,
        rename: bytes("Renamed"),
        used: false,
    });
    let mut t = tokens(&["effect", "Name", "-priority", "bad"]);
    c.enter_visual(&mut t, &mut Ascii, &mut h).unwrap();
    assert_eq!(c.visual.description.priority, 0);
    assert_eq!(c.name, b"Renamed");
    assert_eq!(t[2].bytes, b"-priority");
    assert!(c.pending_priorities.last().unwrap().used);
    c.collection = Some(Identity(1));
    let saved = c.visual.clone();
    h.expected.push_back((
        Call::PublishVisual(Identity(1), bytes("Renamed"), saved.clone()),
        Err(Error::Foreign("publication".into())),
    ));
    assert!(c.exit_visual(&mut h).is_err());
    assert_eq!(c.name, b"Renamed");
    h.expected.push_back((
        Call::PublishVisual(Identity(1), bytes("Renamed"), saved.clone()),
        Ok(Reply::Void),
    ));
    c.exit_visual(&mut h).unwrap();
    assert!(c.name.is_empty());
    assert_eq!(c.visual, saved);
    c.name = bytes("Name");
    c.collection = None;
    c.exit_visual(&mut h).unwrap();
    assert!(c.name.is_empty());
    h.done();
}

fn preview_setup(source: Transform, instance: Transform) -> Vec<(Call, Reply)> {
    vec![
        (Call::Exists(Identity(1), bytes("Name")), Reply::Bool(true)),
        (
            Call::CreatePreview(Identity(1), bytes("Name")),
            Reply::StatusObject(true, Some(Identity(2))),
        ),
        (Call::Retain(Identity(2)), Reply::Void),
        (Call::SourceTransform(Identity(2), source), Reply::Void),
        (Call::InstanceTransform(Identity(2), instance), Reply::Void),
    ]
}
#[test]
fn ec43_ec44_ec45_ec46_preview_defaults_positions_and_ordered_setters() {
    for (input, early, position) in [
        (vec!["testEffect", "Name"], vec![], [512., 100., 512.]),
        (
            vec!["testEffect", "Name", "10,20,30", "ignored"],
            vec![
                (Call::CountArguments(bytes("10,20,30")), Reply::Uint(3)),
                (Call::Vector3(bytes("10,20,30")), Reply::Vector3([10., 20., 30.])),
            ],
            [10., 20., 30.],
        ),
        (
            vec!["testEffect", "Name", "10,30"],
            vec![
                (Call::CountArguments(bytes("10,30")), Reply::Uint(2)),
                (Call::Vector2(bytes("10,30")), Reply::Vector2([10., 30.])),
                (Call::Terrain(10., 30.), Reply::Float(7.)),
            ],
            [10., 11., 30.],
        ),
    ] {
        let mut c = Commands {
            manager: Some(Identity(1)),
            ..Commands::default()
        };
        let mut instance = Transform::new([0.; 3], 1.);
        instance.change = 5;
        let mut calls = early;
        calls.extend(preview_setup(Transform::new(position, 1.), instance));
        calls.push((Call::Release(Identity(2)), Reply::Void));
        let mut h = Host::returning(calls);
        c.preview(&mut tokens(&input), &mut Ascii, &mut h).unwrap();
        assert_eq!(c.previews, [Identity(2)]);
        assert_eq!(c.hard, [0]);
        h.done();
    }
    let mut c = Commands {
        manager: Some(Identity(1)),
        ..Commands::default()
    };
    let mut calls = vec![
        f("2", 2.),
        f("3", 3.),
        (Call::Vector3(bytes("4,5,6")), Reply::Vector3([4., 5., 6.])),
    ];
    let mut instance = Transform::new([4., 5., 6.], 3.);
    instance.change = 5;
    calls.extend(preview_setup(Transform::new([512., 100., 512.], 2.), instance));
    calls.extend([
        f("7", 7.),
        (Call::Speed(Identity(2), 7.), Reply::Void),
        (Call::Vector3(bytes("8,9,10")), Reply::Vector3([8., 9., 10.])),
        (Call::Target(Identity(2), [8., 9., 10.]), Reply::Void),
        (Call::Release(Identity(2)), Reply::Void),
    ]);
    let mut h = Host::returning(calls);
    c.preview(
        &mut tokens(&[
            "testEffect",
            "Name",
            "-sourceScale",
            "2",
            "-scale",
            "3",
            "-trans",
            "4,5,6",
            "-speed",
            "7",
            "-target",
            "8,9,10",
            "-hard",
        ]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.hard, [1]);
    h.done();
}
#[test]
fn ec47_ec48_ec49_ec50_preview_failure_gates_and_late_error_prefix() {
    let mut c = Commands {
        manager: Some(Identity(1)),
        ..Commands::default()
    };
    let mut h = Host::default();
    h.expected
        .push_back((Call::Vector3(bytes("bad")), Err(Error::Foreign("vector".into()))));
    let mut t = tokens(&["testEffect", "Name", "-trans", "bad"]);
    assert!(c.preview(&mut t, &mut Ascii, &mut h).is_err());
    assert_eq!(t[2].bytes, b"+trans");
    h.expected
        .push_back((Call::Exists(Identity(1), bytes("Name")), Ok(Reply::Bool(false))));
    assert_eq!(
        c.preview(&mut tokens(&["testEffect", "Name"]), &mut Ascii, &mut h),
        Err(Error::Parse("No such effect".into()))
    );
    h.done();
    let mut h = Host::returning(vec![
        (Call::Exists(Identity(1), bytes("Name")), Reply::Bool(true)),
        (
            Call::CreatePreview(Identity(1), bytes("Name")),
            Reply::StatusObject(false, Some(Identity(2))),
        ),
        (Call::Release(Identity(2)), Reply::Void),
    ]);
    c.preview(
        &mut tokens(&["testEffect", "Name", "-speed", "bad"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert!(c.previews.is_empty());
    assert!(c.hard.is_empty());
    h.done();
    let mut instance = Transform::new([0.; 3], 1.);
    instance.change = 5;
    let mut h = Host::returning(preview_setup(Transform::new([512., 100., 512.], 1.), instance));
    h.expected
        .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
    let mut t = tokens(&["testEffect", "Name", "-speed", "bad", "-hard"]);
    assert!(c.preview(&mut t, &mut Ascii, &mut h).is_err());
    assert_eq!(c.previews, [Identity(2)]);
    assert!(c.hard.is_empty());
    assert_eq!(t[2].bytes, b"+speed");
    assert_eq!(t[4].bytes, b"-hard");
    h.done();
}
