// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::{bytes, Host};
use super::*;
use crate::effects_parser::script::tests::tokens;
fn f(s: &str, v: f64) -> (Call, Reply) {
    (Call::Float(bytes(s)), Reply::Float(v))
}
fn prepared() -> Commands {
    let mut c = Commands::default();
    for kind in [DynamicParticle, Sequence, Shake, Light] {
        let d = c.definitions.get_mut(kind);
        d.name = bytes("Name");
        d.description = Description::reset(kind, &d.description);
    }
    c.definitions.get_mut(Sequence).object = Some(Identity(1));
    c
}
#[test]
fn ec121_through_ec130_dynamic_guard_stores_and_model_transaction_boundary() {
    let mut c = Commands::default();
    let mut h = Host::default();
    assert!(c.dynamic_base(&tokens(&["effectBase"]), &mut Ascii).is_err());
    assert!(c.dynamic_mass(&tokens(&["mass"]), &mut Ascii, &mut h).is_err());
    assert!(c
        .dynamic_friction(&mut tokens(&["friction"]), &mut Ascii, &mut h)
        .is_err());
    assert!(c
        .dynamic_model_selected(&tokens(&["model"]), 1, &[], &mut Ascii)
        .is_err());
    c = prepared();
    c.dynamic_base(&tokens(&["effectBase", "MiXeD", "extra"]), &mut Ascii)
        .unwrap();
    assert_eq!(c.definitions.get(DynamicParticle).description.strings[&0x0c], b"MiXeD");
    let mut h = Host::returning(vec![f("-2", -2.)]);
    c.dynamic_mass(&tokens(&["mass", "-2", "99"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(
        c.definitions.get(DynamicParticle).description.get_scalar(0x18),
        Some(-2.)
    );
    let mut h = Host::returning(vec![f("2", 2.)]);
    h.expected
        .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
    assert!(c
        .dynamic_friction(
            &mut tokens(&["friction", "2", "bad", "-angular", "8"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    let d = &c.definitions.get(DynamicParticle).description;
    assert_eq!(d.get_scalar(0x20), Some(2.));
    assert_eq!(d.get_scalar(0x24), Some(0.));
    assert_eq!(d.get_scalar(0x28), Some(0.));
    let mut h = Host::returning(vec![f("2", 2.), f("3", 3.), f("-4", -4.)]);
    c.dynamic_friction(
        &mut tokens(&["friction", "2", "3", "99", "-angular", "-4"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(
        c.definitions.get(DynamicParticle).description.get_scalar(0x28),
        Some(-4.)
    );
    h.done();
    let old = c.definitions.get(DynamicParticle).description.clone();
    for eax in [0, -1] {
        c.dynamic_model_selected(&tokens(&["model", "missing"]), eax, &[1], &mut Ascii)
            .unwrap();
        assert_eq!(c.definitions.get(DynamicParticle).description, old);
    }
    c.id_maps[1].insert(bytes("house"), 7);
    let d = &mut c.definitions.get_mut(DynamicParticle).description;
    d.words.insert(0x30, 9);
    d.lists.insert(0x34, vec![1, 2]);
    let t = tokens(&["model", "HOUSE"]);
    c.dynamic_model_selected(&t, 1, &[1], &mut Ascii).unwrap();
    assert_eq!(t[1].bytes, b"HOUSE");
    assert_eq!(c.definitions.get(DynamicParticle).description.words[&0x30], 7);
    assert!(c.definitions.get(DynamicParticle).description.lists[&0x34].is_empty());
    c.id_maps[1].insert(bytes("a"), 7);
    c.id_maps[1].insert(bytes("b"), 0);
    c.definitions.get_mut(DynamicParticle).description.words.insert(0x30, 9);
    c.dynamic_model_selected(&tokens(&["model", "A", "B", "a"]), 1, &[1, 2, 3], &mut Ascii)
        .unwrap();
    assert_eq!(c.definitions.get(DynamicParticle).description.words[&0x30], 9);
    assert_eq!(c.definitions.get(DynamicParticle).description.lists[&0x34], [7, 0, 7]);
    let old = c.definitions.get(DynamicParticle).description.clone();
    assert!(c
        .dynamic_model_selected(&tokens(&["model", "A", "Missing"]), 1, &[1, 2], &mut Ascii)
        .is_err());
    assert_eq!(c.definitions.get(DynamicParticle).description, old);
    assert!(!c.id_maps[1].contains_key(b"missing".as_slice()));
    c.dynamic_model_selected(&tokens(&["model"]), 1, &[], &mut Ascii)
        .unwrap();
    assert!(c.definitions.get(DynamicParticle).description.lists[&0x34].is_empty());
    assert_eq!(c.definitions.get(DynamicParticle).description.words[&0x30], 9);
}
#[test]
fn ec132_through_ec140_sequence_endpoint_appends_and_failure_atomicity() {
    let mut c = prepared();
    let original = SequenceElement {
        first: 9.,
        second: 9.,
        name: bytes("E"),
    };
    c.definitions.get_mut(Sequence).sequence_elements.push(original.clone());
    for (play, input, replies, expected) in [
        (
            false,
            vec!["wait", "3"],
            vec![f("3", 3.)],
            SequenceElement {
                first: 3.,
                second: 3.,
                name: vec![],
            },
        ),
        (
            false,
            vec!["wait", "4", "-2", "99"],
            vec![f("4", 4.), f("-2", -2.)],
            SequenceElement {
                first: 4.,
                second: -2.,
                name: vec![],
            },
        ),
        (
            true,
            vec!["play", "MiXeD", "2"],
            vec![f("2", 2.)],
            SequenceElement {
                first: 2.,
                second: 2.,
                name: bytes("MiXeD"),
            },
        ),
        (
            true,
            vec!["play", "Missing", "-3", "7", "99"],
            vec![f("-3", -3.), f("7", 7.)],
            SequenceElement {
                first: -3.,
                second: 7.,
                name: bytes("Missing"),
            },
        ),
    ] {
        let mut h = Host::returning(replies);
        c.sequence_element(play, &tokens(&input), &mut Ascii, &mut h).unwrap();
        assert_eq!(c.definitions.get(Sequence).sequence_elements.last(), Some(&expected));
        assert_eq!(c.definitions.get(Sequence).sequence_elements[0], original);
        h.done();
    }
    for play in [false, true] {
        let old = c.definitions.get(Sequence).sequence_elements.clone();
        let mut h = Host::returning(vec![f("1", 1.)]);
        h.expected
            .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
        let t = if play {
            tokens(&["play", "Name", "1", "bad"])
        } else {
            tokens(&["wait", "1", "bad"])
        };
        assert!(c.sequence_element(play, &t, &mut Ascii, &mut h).is_err());
        assert_eq!(c.definitions.get(Sequence).sequence_elements, old);
    }
    let mut h = Host::returning(vec![(Call::Float(bytes("-0")), Reply::Float32(0x80000000))]);
    c.sequence_element(false, &tokens(&["wait", "-0"]), &mut Ascii, &mut h)
        .unwrap();
    let last = c.definitions.get(Sequence).sequence_elements.last().unwrap();
    assert_eq!(last.first.to_bits(), 0x80000000);
    assert_eq!(last.second.to_bits(), 0x80000000);
    let mut h = Host::returning(vec![
        (Call::Float(bytes("nan")), Reply::Float32(f32::NAN.to_bits())),
        (Call::Float(bytes("inf")), Reply::Float32(f32::INFINITY.to_bits())),
    ]);
    c.sequence_element(true, &tokens(&["play", "Name", "nan", "inf"]), &mut Ascii, &mut h)
        .unwrap();
    let last = c.definitions.get(Sequence).sequence_elements.last().unwrap();
    assert!(last.first.is_nan());
    assert!(last.second.is_infinite());
    c.definitions.get_mut(Sequence).name.clear();
    let t = tokens(&["wait", "1", "-unknown"]);
    let mut h = Host::returning(vec![f("1", 1.)]);
    c.sequence_element(false, &t, &mut Ascii, &mut h).unwrap();
    assert!(check_switches(&t, &mut Ascii).is_err());
    c.definitions.get_mut(Sequence).object = None;
    let mut h = Host::returning(vec![f("1", 1.)]);
    assert_eq!(
        c.sequence_element(false, &tokens(&["wait", "1"]), &mut Ascii, &mut h),
        Err(Error::Unsupported("installed sequence required"))
    );
}
#[test]
fn ec142_through_ec152_shake_scalar_nan_clamps_and_light_cross_write() {
    for (length, fade, expected) in [
        (3., 4., 3.),
        (-2., -1., -2.),
        (f32::NAN, 4., 4.),
        (3., f32::NAN, f32::NAN),
    ] {
        let mut c = prepared();
        let mut h = Host::returning(vec![
            (Call::Float(bytes("length")), Reply::Float32(length.to_bits())),
            (Call::Float(bytes("fade")), Reply::Float32(fade.to_bits())),
        ]);
        c.shake_light_length(
            false,
            &mut tokens(&["length", "length", "-fade", "fade"]),
            &mut Ascii,
            &mut h,
        )
        .unwrap();
        let result = c.definitions.get(Shake).description.get_scalar(4).unwrap();
        assert!(result == expected || (result.is_nan() && expected.is_nan()));
    }
    let mut c = prepared();
    c.definitions.get_mut(Shake).description.scalar(4, 4.);
    let mut h = Host::returning(vec![f("9", 9.)]);
    h.expected
        .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
    assert!(c
        .shake_light_length(false, &mut tokens(&["length", "9", "-fade", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.definitions.get(Shake).description.get_scalar(0), Some(9.));
    assert_eq!(c.definitions.get(Shake).description.get_scalar(4), Some(4.));
    let mut h = Host::default();
    assert!(c
        .shake_light_length(false, &mut tokens(&["length"]), &mut Ascii, &mut h)
        .is_err());
    assert!(c.shake_aspect(&tokens(&["shakeAspect"]), &mut h).is_err());
    assert!(c.shake_table(&tokens(&["table"]), &mut h).is_err());
    let mut h = Host::returning(vec![
        f("-5", -5.),
        (Call::Enum(bytes("sineY"), SHAKE_TABLE), Reply::Uint(1)),
    ]);
    c.shake_aspect(&tokens(&["shakeAspect", "-5", "extra", "-unknown"]), &mut h)
        .unwrap();
    c.shake_table(&tokens(&["table", "sineY"]), &mut h).unwrap();
    assert_eq!(c.definitions.get(Shake).description.get_scalar(0x20), Some(-5.));
    assert_eq!(c.definitions.get(Shake).description.words[&0x24], 1);
    h.expected.push_back((
        Call::Enum(bytes("bad"), SHAKE_TABLE),
        Err(Error::Foreign("enum".into())),
    ));
    assert!(c.shake_table(&tokens(&["table", "bad"]), &mut h).is_err());
    assert_eq!(c.definitions.get(Shake).description.words[&0x24], 1);
    c.definitions.get_mut(Shake).description.scalar(0, 5.);
    c.definitions.get_mut(Shake).description.scalar(4, 1.);
    let mut h = Host::returning(vec![f("100", 100.), f("9", 9.)]);
    c.shake_light_length(true, &mut tokens(&["length", "100", "-fade", "9"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.definitions.get(Light).description.get_scalar(0x18), Some(100.));
    assert_eq!(c.definitions.get(Shake).description.get_scalar(4), Some(5.));
    c.definitions.get_mut(Shake).description.scalar(4, 9.);
    let mut h = Host::returning(vec![f("2", 2.)]);
    c.shake_light_length(true, &mut tokens(&["length", "2"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.definitions.get(Shake).description.get_scalar(4), Some(5.));
    c.definitions.get_mut(Shake).description.scalar(4, 1.);
    let mut h = Host::returning(vec![f("2", 2.)]);
    h.expected
        .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
    assert!(c
        .shake_light_length(true, &mut tokens(&["length", "2", "-fade", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.definitions.get(Light).description.get_scalar(0x18), Some(2.));
    assert_eq!(c.definitions.get(Shake).description.get_scalar(4), Some(1.));
}
#[test]
fn ec153_through_ec160_curves_color_aliases_and_unscaled_failure_prefix() {
    let mut c = prepared();
    let mut h = Host::returning(vec![f("2", 2.), f("-3", -3.), f("2", 2.)]);
    c.shake_light_curve(Shake, 8, &tokens(&["amplitude", "2", "-3", "2"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.definitions.get(Shake).description.floats[&8], [2., -3., 2.]);
    let mut h = Host::returning(vec![f("30", 30.)]);
    h.expected
        .push_back((Call::Float(bytes("bad")), Err(Error::Foreign("float".into()))));
    assert!(c
        .shake_light_curve(Shake, 0x14, &tokens(&["frequency", "30", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.definitions.get(Shake).description.floats[&0x14], [30.]);
    c.definitions.get_mut(Light).description.floats.insert(0x0c, vec![7.]);
    assert!(c
        .shake_light_curve(Light, 0x0c, &tokens(&["strength"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.definitions.get(Light).description.floats[&0x0c], [7.]);
    let mut h = Host::returning(vec![
        f("-1", -1.),
        (Call::Float(bytes("nan")), Reply::Float32(f32::NAN.to_bits())),
    ]);
    c.shake_light_curve(Light, 0x0c, &tokens(&["strength", "-1", "nan"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.definitions.get(Light).description.floats[&0x0c][0], -1.);
    assert!(c.definitions.get(Light).description.floats[&0x0c][1].is_nan());
    for (name, expected) in [
        ("color255", [1., f32::from_bits(0x3f008081), -1.]),
        ("colour", [255., 128., -255.]),
    ] {
        let mut h = Host::returning(vec![(Call::Color(bytes("color")), Reply::Vector3([255., 128., -255.]))]);
        c.light_color(&tokens(&[name, "color"]), &mut Ascii, &mut h).unwrap();
        assert_eq!(c.definitions.get(Light).description.vectors[&0], [expected]);
    }
    let mut h = Host::returning(vec![(Call::Color(bytes("first")), Reply::Vector3([255., 128., -255.]))]);
    h.expected
        .push_back((Call::Color(bytes("bad")), Err(Error::Foreign("color".into()))));
    assert!(c
        .light_color(&tokens(&["colour255", "first", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.definitions.get(Light).description.vectors[&0], [[255., 128., -255.]]);
    let mut h = Host::returning(vec![(Call::Color(bytes("red")), Reply::Vector3([255., 0., 0.]))]);
    let t = tokens(&["color255", "red", "-vary", "1"]);
    c.light_color(&t, &mut Ascii, &mut h).unwrap();
    assert!(check_switches(&t, &mut Ascii).is_err());
    assert_eq!(c.definitions.get(Light).description.vectors[&0], [[1., 0., 0.]]);
}
