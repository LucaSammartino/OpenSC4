// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::VecDeque;
const POLICY: NumericPolicy = NumericPolicy::NearestAtLeast53;
#[derive(Default)]
struct Host {
    calls: Vec<Call>,
    values: VecDeque<Reply>,
    payloads: Vec<Description>,
    reject: bool,
    fail_float: bool,
    fail_hook: bool,
}
impl Services for Host {
    fn call(&mut self, call: Call) -> Result<Reply> {
        self.calls.push(call.clone());
        Ok(match call {
            Call::ValidateNested(..) => Reply::Bool(!self.reject),
            Call::AllocateNested(..) => Reply::Object(Some(Identity(7))),
            Call::Retain(..) | Call::Release(..) => Reply::Void,
            Call::InsertNested(_, _, d) => {
                self.payloads.push(d);
                Reply::Uint(u32::MAX)
            }
            Call::NestedType(..) => Reply::Uint(0x12340005),
            Call::InheritedBlock => {
                if self.fail_hook {
                    return Err(Error::Foreign("hook".into()));
                }
                Reply::Void
            }
            Call::Float(..) if self.fail_float => return Err(Error::Foreign("conversion".into())),
            Call::Float(..) | Call::Uint(..) | Call::Int(..) | Call::Vector3(..) | Call::Ftoll(..) => {
                self.values.pop_front().expect("missing independent primitive reply")
            }
            _ => panic!("unexpected call {call:?}"),
        })
    }
}
fn tokens(args: &[&str]) -> Vec<ScriptToken> {
    args.iter()
        .enumerate()
        .map(|(i, s)| ScriptToken::new(i as u32, s))
        .collect()
}
fn live() -> Commands {
    Commands {
        name: b"effect".to_vec(),
        ..Commands::default()
    }
}
fn add(c: &mut Commands, weight: u16) {
    let mut record = fresh();
    record.component.select_weight = weight;
    c.visual.components.push(Component { name: vec![], record });
}
#[test]
fn ec277_through_ec282_named_context_validation_shells_and_flags() {
    for (kind, ty, slot) in [
        (Named::Particle, 0, Some(0x20)),
        (Named::Dynamic, 16, None),
        (Named::Decal, 1, Some(0x30)),
        (Named::Visual, 2, Some(0x18)),
    ] {
        let mut c = live();
        c.collection = Some(Identity(1));
        c.word_4a0 = 65538;
        c.byte_4ac = 1;
        let mut h = Host::default();
        let mut t = tokens(&["child", "MiXeD", "ignored"]);
        c.named_component(kind, &mut t, &mut Ascii, &mut h, POLICY).unwrap();
        assert_eq!(
            h.calls,
            slot.map(|s| vec![Call::ValidateNested(Identity(1), s, b"MiXeD".to_vec())])
                .unwrap_or_default()
        );
        let r = &c.visual.components[0];
        assert_eq!(r.name, b"MiXeD");
        assert_eq!(r.record.type_word, ty);
        assert_eq!(
            (
                r.record.component.select_group,
                r.record.component.lod_min,
                r.record.component.lod_max
            ),
            (2, 1, 6)
        );
        assert_eq!(
            r.record.component.flags,
            if matches!(kind, Named::Particle | Named::Dynamic) {
                2
            } else {
                0
            }
        );
        let mut c = live();
        c.collection = Some(Identity(1));
        let mut h = Host {
            reject: true,
            ..Host::default()
        };
        let result = c.named_component(kind, &mut tokens(&["child", "missing"]), &mut Ascii, &mut h, POLICY);
        assert_eq!(result.is_ok(), kind == Named::Dynamic);
        assert_eq!(c.visual.components.len(), usize::from(kind == Named::Dynamic));
    }
    for (args, values, spacing) in [
        (
            vec!["particleEffect", "A", "-shells", "65538", "3"],
            vec![Reply::Uint(65538), Reply::Float(3.)],
            3,
        ),
        (vec!["particleEffect", "A", "-shells", "2"], vec![Reply::Uint(2)], 16),
    ] {
        let mut c = live();
        let mut h = Host {
            values: values.into(),
            ..Host::default()
        };
        c.named_component(Named::Particle, &mut tokens(&args), &mut Ascii, &mut h, POLICY)
            .unwrap();
        let r = &c.visual.components[0].record.component;
        assert_eq!((r.copies, r.spacing), (2, spacing));
        assert!(h.values.is_empty());
    }
    let mut c = Commands::default();
    let mut t = tokens(&["particleEffect", "-shells"]);
    let before = t.clone();
    assert_eq!(
        c.named_component(Named::Particle, &mut t, &mut Ascii, &mut Host::default(), POLICY),
        Err(Error::Parse("Not in effects block".into()))
    );
    assert_eq!(t, before);
}
#[test]
fn ec283_through_ec287_auxiliary_order_alias_short_circuit_and_failure_prefix() {
    let mut c = live();
    let mut h = Host::default();
    for (kind, name) in [
        (AuxiliaryKind::Shake, "A"),
        (AuxiliaryKind::Flash, "B"),
        (AuxiliaryKind::Tint, "C"),
    ] {
        c.auxiliary_component(kind, &mut tokens(&["child", name]), &mut Ascii, &mut h, POLICY)
            .unwrap();
    }
    assert_eq!(
        c.visual.auxiliary.iter().map(|a| a.flags).collect::<Vec<_>>(),
        [3, 4, 8]
    );
    assert!(c.visual.components.is_empty());
    assert_eq!(
        c.visual.auxiliary[0],
        Auxiliary {
            flags: 3,
            name: b"A".to_vec(),
            value: 0,
            index: u32::MAX
        }
    );
    let mut t = tokens(&["shakeEffect", "A", "-noEpicentre", "-noEpicenter"]);
    c.auxiliary_component(AuxiliaryKind::Shake, &mut t, &mut Ascii, &mut h, POLICY)
        .unwrap();
    assert_eq!(c.visual.auxiliary.last().unwrap().flags, 1);
    assert_eq!(t[2].bytes, b"+noEpicentre");
    assert_eq!(t[3].bytes, b"-noEpicenter");
    c.auxiliary_component(
        AuxiliaryKind::Flash,
        &mut tokens(&["flashEffect", "B", "-epicenter"]),
        &mut Ascii,
        &mut h,
        POLICY,
    )
    .unwrap();
    assert_eq!(c.visual.auxiliary.last().unwrap().value, 0x447a0000);
    h.fail_float = true;
    assert!(c
        .auxiliary_component(
            AuxiliaryKind::Flash,
            &mut tokens(&["flashEffect", "B", "-epicentre", "bad"]),
            &mut Ascii,
            &mut h,
            POLICY
        )
        .is_err());
    assert_eq!(
        (
            c.visual.auxiliary.last().unwrap().flags,
            c.visual.auxiliary.last().unwrap().value
        ),
        (6, 0)
    );
    let count = c.visual.auxiliary.len();
    for name in ["MixedCase", "Other"] {
        c.chain_component(&tokens(&["chainEffect", name]), &mut Ascii).unwrap();
    }
    assert_eq!(c.visual.chain, b"Other");
    assert_eq!(c.visual.auxiliary.len(), count);
}
#[test]
fn ec288_through_ec290_sound_brush_maps_and_stored_length() {
    let mut c = live();
    let mut h = Host::default();
    assert_eq!(
        c.anonymous_component(
            Kind::Sound,
            &mut tokens(&["soundEffect", "named"]),
            &mut Ascii,
            &mut h,
            POLICY,
            0
        ),
        Err(Error::Parse("sound description unimplemented".into()))
    );
    assert!(h.calls.is_empty());
    c.collection = Some(Identity(1));
    c.id_maps[2].insert(b"beep".to_vec(), 9);
    h.values = vec![Reply::Float(4.), Reply::Float32(0x80000000)].into();
    c.anonymous_component(
        Kind::Sound,
        &mut tokens(&[
            "soundEffect",
            "-name",
            "BEEP",
            "-locationUpdateRate",
            "4",
            "-length",
            "negativezero",
        ]),
        &mut Ascii,
        &mut h,
        POLICY,
        0,
    )
    .unwrap();
    let d = &h.payloads[0];
    assert_eq!(
        (d.words[&0x10], d.words[&0x14], d.words[&0x18]),
        (9, 0x3e800000, 0x80000000)
    );
    assert_eq!(c.visual.components[0].record.component.flags, 1);
    c.id_maps[3].insert(b"oakbrush".to_vec(), 12);
    h.values = vec![Reply::Float(2.), Reply::Float(3.), Reply::Int(5)].into();
    c.anonymous_component(
        Kind::Brush,
        &mut tokens(&["brushEffect", "-name", "OAK", "-rate", "2", "-apply", "3", "-zoom", "5"]),
        &mut Ascii,
        &mut h,
        POLICY,
        0,
    )
    .unwrap();
    let d = &h.payloads[1];
    assert_eq!(
        (d.words[&0x0c], d.words[&0x10], d.bytes[&0x18], d.words[&0x1c]),
        (12, 3f32.to_bits(), 1, 4)
    );
    assert_eq!(c.visual.components[1].record.component.flags, 0);
    assert_eq!(c.visual.components[1].record.type_word, 0x12340005);
}
#[test]
fn ec291_through_ec293_scrubber_aliases_crt_and_shape_overrides() {
    for alias in ["scrubberEffect", "demolishEffect", "gameEffect", "mapEffect"] {
        let mut c = live();
        let mut h = Host::default();
        let mut t = tokens(&[alias, "-single"]);
        assert_eq!(
            c.anonymous_component(Kind::Scrubber, &mut t, &mut Ascii, &mut h, POLICY, 0xabcdef01),
            Err(Error::Parse("Need some options for anonymous scrubber effect".into()))
        );
        assert_eq!(t[1].bytes, b"-single");
        assert!(h.payloads.is_empty());
    }
    let mut c = live();
    c.collection = Some(Identity(1));
    let mut h = Host {
        values: vec![Reply::Float(2.75), Reply::Int(2), Reply::Uint(511)].into(),
        ..Host::default()
    };
    c.anonymous_component(
        Kind::Scrubber,
        &mut tokens(&[
            "scrubberEffect",
            "-demolish",
            "2.75",
            "-explode",
            "-demolishEffectID",
            "511",
        ]),
        &mut Ascii,
        &mut h,
        POLICY,
        0xabcdef01,
    )
    .unwrap();
    assert_eq!(
        (
            h.payloads[0].words[&0x14],
            h.payloads[0].words[&0x18],
            h.payloads[0].words[&0x40]
        ),
        (2, 0x13ff, 0xabcdef01)
    );
    h.values = vec![
        Reply::Float(2.),
        Reply::Float(3.),
        Reply::Int(2),
        Reply::Float(4.),
        Reply::Float(5.),
        Reply::Float(6.),
        Reply::Int(3),
        Reply::Float(7.),
        Reply::Float(8.),
        Reply::Float(9.),
        Reply::Float(10.),
    ]
    .into();
    c.anonymous_component(
        Kind::Scrubber,
        &mut tokens(&[
            "scrubberEffect",
            "-pauseSim",
            "2",
            "-pauseClock",
            "3",
            "-blob",
            "2",
            "4",
            "5",
            "6",
            "-rect",
            "3",
            "7",
            "8",
            "9",
            "10",
        ]),
        &mut Ascii,
        &mut h,
        POLICY,
        0,
    )
    .unwrap();
    let d = &h.payloads[1];
    assert_eq!(d.words[&0x0c], 0x50);
    assert_eq!(d.words[&0x38], 3);
    for (at, x) in [(0x3c, 7f32), (0x40, 8.), (0x44, 9.), (0x48, 10.), (0x4c, 3.)] {
        assert_eq!(d.words[&at], x.to_bits());
    }
    assert!(h.values.is_empty());
}
#[test]
fn ec294_through_ec297_ec305_automata_camera_and_anonymous_insertion_prefix() {
    let mut c = live();
    c.collection = Some(Identity(1));
    let mut h = Host::default();
    let mut t = tokens(&["automataEffect", "-name", "MiXeD", "-group", "Other"]);
    c.anonymous_component(Kind::Automata, &mut t, &mut Ascii, &mut h, POLICY, 0)
        .unwrap();
    assert_eq!(h.payloads[0].strings[&0x0c], b"MiXeD");
    assert_eq!(h.payloads[0].bytes[&0x18], 0);
    assert_eq!(t[3].bytes, b"-group");
    h.values = vec![Reply::Int(5), Reply::Int(2)].into();
    c.anonymous_component(
        Kind::Camera,
        &mut tokens(&["cameraEffect", "-target", "-zoom", "5", "-rotation", "2"]),
        &mut Ascii,
        &mut h,
        POLICY,
        0,
    )
    .unwrap();
    let d = &h.payloads[1];
    assert_eq!((d.words[&0x0c], d.bytes[&0x10], d.bytes[&0x11]), (7, 2, 0));
    c.anonymous_component(
        Kind::Camera,
        &mut tokens(&["cameraEffect", "-slave", "-target"]),
        &mut Ascii,
        &mut h,
        POLICY,
        0,
    )
    .unwrap();
    assert_eq!(h.payloads[2].words[&0x0c], 12);
    let mut c = live();
    let mut h = Host::default();
    let mut t = tokens(&["cameraEffect", "-lod", "1"]);
    c.anonymous_component(Kind::Camera, &mut t, &mut Ascii, &mut h, POLICY, 0)
        .unwrap();
    assert_eq!(
        h.calls,
        [
            Call::AllocateNested(Kind::Camera),
            Call::Retain(Identity(7)),
            Call::Release(Identity(7))
        ]
    );
    assert_eq!(t[1].bytes, b"-lod");
    assert!(c.visual.components.is_empty());
    c.collection = Some(Identity(1));
    c.word_4a0 = 1;
    h.calls.clear();
    assert_eq!(
        c.anonymous_component(
            Kind::Camera,
            &mut tokens(&["cameraEffect", "-lod", "1"]),
            &mut Ascii,
            &mut h,
            POLICY,
            0
        ),
        Err(Error::Parse("Can't specify LOD in a select".into()))
    );
    assert_eq!(h.payloads.len(), 1);
    assert!(c.visual.components.is_empty());
    assert_eq!(h.calls.last(), Some(&Call::NestedType(Identity(7))));
}
#[test]
fn ec298_through_ec304_select_counts_tail_wrap_nesting_and_hooks() {
    let mut c = live();
    for _ in 0..3 {
        add(&mut c, 0);
    }
    c.word_4a4 = u32::MAX;
    let mut h = Host::default();
    c.enter_select(&mut h).unwrap();
    assert_eq!((c.word_4a4, c.word_4a0, c.word_4a8), (0, 0, 3));
    for (input, output) in [
        (vec![0, 0], vec![32768, 32767]),
        (vec![100, 200], vec![100, 197]),
        (vec![0], vec![65535]),
    ] {
        let mut c = live();
        c.enter_select(&mut h).unwrap();
        for x in input {
            add(&mut c, x);
        }
        c.exit_select();
        assert_eq!(
            c.visual
                .components
                .iter()
                .map(|r| r.record.component.select_weight)
                .collect::<Vec<_>>(),
            output
        );
        assert_eq!(c.word_4a0, 0);
    }
    let mut c = live();
    add(&mut c, 9);
    c.enter_select(&mut h).unwrap();
    c.exit_select();
    assert_eq!(c.visual.components[0].record.component.select_weight, 8);
    let mut c = live();
    c.enter_select(&mut h).unwrap();
    c.exit_select();
    assert!(c.visual.components.is_empty());
    c.enter_select(&mut h).unwrap();
    add(&mut c, 100);
    c.enter_select(&mut h).unwrap();
    add(&mut c, 200);
    c.exit_select();
    assert_eq!(c.visual.components[1].record.component.select_weight, 198);
    c.exit_select();
    assert_eq!(c.visual.components[1].record.component.select_weight, 196);
    assert_eq!(c.word_4a8, 1);
    c.enter_particle_sequence(&mut h).unwrap();
    c.enter_particle_sequence(&mut h).unwrap();
    c.exit_particle_sequence();
    c.named_component(
        Named::Particle,
        &mut tokens(&["particleEffect", "A"]),
        &mut Ascii,
        &mut h,
        POLICY,
    )
    .unwrap();
    assert_eq!(c.visual.components.last().unwrap().record.component.flags, 0);
    h.fail_hook = true;
    assert!(c.enter_select(&mut h).is_err());
    assert_ne!(c.word_4a0, 0);
    assert!(c.enter_particle_sequence(&mut h).is_err());
    assert_eq!(c.byte_4ac, 1);
}

#[test]
fn anonymous_failure_prefix_ranges_missing_names_and_nonfinite_lengths() {
    let mut c = live();
    let mut h = Host::default();
    c.collection = Some(Identity(1));
    c.id_maps[3].insert(b"oak".to_vec(), 9);
    h.values = vec![Reply::Float(2.), Reply::RawFloat(RawFloat::Unsupported)].into();
    assert!(c
        .anonymous_component(
            Kind::Brush,
            &mut tokens(&["brushEffect", "-name", "oak", "-strength", "2", "unqualified"]),
            &mut Ascii,
            &mut h,
            POLICY,
            0
        )
        .is_err());
    let d = &c.pending_nested.as_ref().unwrap().2;
    assert_eq!(d.words[&0x20], 2f32.to_bits());
    assert_eq!(d.words[&0x24], 1f32.to_bits());
    assert!(h.payloads.is_empty());
    assert!(!h.calls.iter().any(|call| matches!(call, Call::Release(..))));
    let mut c = live();
    let mut h = Host {
        values: vec![Reply::Int(0)].into(),
        ..Host::default()
    };
    assert!(c
        .anonymous_component(
            Kind::Camera,
            &mut tokens(&["cameraEffect", "-slave", "-zoom", "0"]),
            &mut Ascii,
            &mut h,
            POLICY,
            0
        )
        .is_err());
    let d = &c.pending_nested.as_ref().unwrap().2;
    assert_eq!(d.words[&0x0c], 8);
    assert_eq!(d.bytes[&0x10], 0);
    for kind in [Kind::Sound, Kind::Brush, Kind::Automata] {
        let mut c = live();
        let mut h = Host::default();
        assert!(c
            .anonymous_component(kind, &mut tokens(&["child"]), &mut Ascii, &mut h, POLICY, 0)
            .is_err());
        assert!(h.payloads.is_empty());
    }
    for (length, flag) in [(0x00000000, 1), (0x80000000, 1), (0x7fc00000, 0), (0x3f800000, 0)] {
        let mut c = live();
        c.collection = Some(Identity(1));
        c.id_maps[2].insert(b"beep".to_vec(), 9);
        let mut h = Host {
            values: vec![Reply::Float(-1.), Reply::Float32(length)].into(),
            ..Host::default()
        };
        c.anonymous_component(
            Kind::Sound,
            &mut tokens(&[
                "soundEffect",
                "-name",
                "beep",
                "-locationUpdateRate",
                "negative",
                "-length",
                "raw",
            ]),
            &mut Ascii,
            &mut h,
            POLICY,
            0,
        )
        .unwrap();
        assert_eq!(h.payloads[0].words[&0x14], 0);
        assert_eq!(c.visual.components[0].record.component.flags, flag);
    }
    // No float occurs here, so an unqualified numeric service cannot reject it.
    live()
        .anonymous_component(
            Kind::Camera,
            &mut tokens(&["cameraEffect", "-slave"]),
            &mut Ascii,
            &mut Host::default(),
            NumericPolicy::Unqualified,
            0,
        )
        .unwrap();
}

#[test]
fn named_common_options_keep_store_order_and_zero_shell_comparison() {
    let mut c = live();
    let mut h = Host {
        values: vec![
            Reply::RawFloat(RawFloat::Extended(Extended::new(0x3fff, 0x8000004000000000))),
            Reply::Uint(511),
            Reply::Float(2.),
            Reply::Float(3.),
        ]
        .into(),
        ..Host::default()
    };
    c.named_component(
        Named::Dynamic,
        &mut tokens(&[
            "dynamicParticleEffect",
            "A",
            "-scale",
            "raw",
            "-lod",
            "511",
            "-emitScale",
            "2",
            "3",
        ]),
        &mut Ascii,
        &mut h,
        POLICY,
    )
    .unwrap();
    let r = &c.visual.components[0].record.component;
    assert_eq!(u32::from_le_bytes(r.placement[52..56].try_into().unwrap()), 0x3f800000);
    assert_eq!(r.placement[0], 1);
    assert_eq!(r.placement[1] & 2, 2);
    assert_eq!((r.lod_min, r.lod_max), (255, 0));
    assert_eq!(&r.render[..2], &[2., 3.]);
    let mut c = live();
    let mut h = Host {
        values: vec![Reply::Uint(2), Reply::Float32(0x80000000)].into(),
        ..Host::default()
    };
    let policy = NumericPolicy::X87(numeric::Control {
        precision: numeric::Precision::P64,
        rounding: numeric::Rounding::Up,
    });
    c.named_component(
        Named::Particle,
        &mut tokens(&["particleEffect", "A", "-shells", "2", "zero"]),
        &mut Ascii,
        &mut h,
        policy,
    )
    .unwrap();
    assert_eq!(c.visual.components[0].record.component.spacing, 0);
    let mut c = live();
    c.word_4a4 = u32::MAX;
    c.enter_select(&mut Host::default()).unwrap();
    let mut h = Host::default();
    assert_eq!(
        c.named_component(
            Named::Particle,
            &mut tokens(&["particleEffect", "A", "-prob", "x"]),
            &mut Ascii,
            &mut h,
            POLICY
        ),
        Err(Error::Parse("-prob only valid inside a select group".into()))
    );
    assert!(h.calls.is_empty());
    assert!(c.visual.components.is_empty());
}
