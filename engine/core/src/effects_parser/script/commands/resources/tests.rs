// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::{bytes, Host};
use super::*;
use crate::effects_parser::script::tests::tokens;
fn fresh() -> Commands {
    let mut c = Commands::default();
    for kind in [Decal, Particles] {
        let d = c.definitions.get_mut(kind);
        d.name = bytes("P");
        d.description = Description::reset(kind, &Description::default());
    }
    c
}
fn floats(values: &[(&str, f64)]) -> Host {
    Host::returning(
        values
            .iter()
            .map(|(s, v)| (Call::Float(bytes(s)), Reply::Float(*v)))
            .collect(),
    )
}
fn fail(h: &mut Host, s: &str) {
    h.expected
        .push_back((Call::Float(bytes(s)), Err(Error::Foreign("conversion".into()))));
}

#[test]
fn ec162_through_ec175_decal_curves_and_life() {
    let mut c = fresh();
    c.definitions.get_mut(Decal).name.clear();
    c.decal_appearance(
        Appearance::Size,
        &mut tokens(&["size", "2", "-3"]),
        &mut Ascii,
        &mut floats(&[("2", 2.), ("-3", -3.)]),
    )
    .unwrap();
    assert_eq!(c.decal_desc().floats[&0x1c], vec![2., -3.]);
    assert!(c
        .decal_appearance(
            Appearance::Size,
            &mut tokens(&["size", "2", "-vary", "2", "-cityScale"]),
            &mut Ascii,
            &mut floats(&[("2", 2.), ("2", 2.)])
        )
        .is_err());
    assert_eq!(c.decal_desc().floats[&0x1c], vec![2.]);
    assert_eq!(c.decal_desc().words[&0], 0);
    c.decal_appearance(
        Appearance::Size,
        &mut tokens(&["size", "2", "-vary", "nan", "-cityScale"]),
        &mut Ascii,
        &mut floats(&[("2", 2.), ("nan", f64::NAN)]),
    )
    .unwrap();
    assert!(c.decal_desc().get_scalar(0x50).unwrap().is_nan());
    assert_eq!(c.decal_desc().words[&0], 0x10);
    for (kind, words, calls, at, list) in [
        (
            Appearance::Aspect,
            vec!["aspect", "2", "3", "-vary", "bad"],
            vec![("2", 2.), ("3", 3.)],
            0x40,
            vec![2., 3.],
        ),
        (
            Appearance::Rotate,
            vec!["rotate", "90", "-90", "-vary", ".25", "-offset", "bad"],
            vec![("90", 90.), ("-90", -90.), (".25", 0.25)],
            0x10,
            vec![90., -90.],
        ),
    ] {
        let mut t = tokens(&words);
        c.decal_appearance(kind, &mut t, &mut Ascii, &mut floats(&calls))
            .unwrap();
        assert_eq!(c.decal_desc().floats[&at], list);
        assert!(check_switches(&t, &mut Ascii).is_err());
    }
    assert_eq!(c.decal_desc().get_scalar(0x54), Some(0.25));
    for name in ["color255", "colour"] {
        let mut h = Host::returning(vec![(Call::Color(bytes("v")), Reply::Vector3([255., 128., -255.]))]);
        let mut t = tokens(&[name, "v", "-vary", "bad"]);
        c.decal_appearance(Appearance::Color, &mut t, &mut Ascii, &mut h)
            .unwrap();
        assert_eq!(
            c.decal_desc().vectors[&0x34][0],
            if name.ends_with("255") {
                [1., f32::from_bits(0x3f008081), -1.]
            } else {
                [255., 128., -255.]
            }
        );
        assert!(check_switches(&t, &mut Ascii).is_err());
    }
    c.decal_appearance(
        Appearance::Alpha,
        &mut tokens(&["alpha255", "255", "128", "-vary", ".5"]),
        &mut Ascii,
        &mut floats(&[("255", 255.), ("128", 128.), (".5", 0.5)]),
    )
    .unwrap();
    assert_eq!(c.decal_desc().floats[&0x28], vec![1., f32::from_bits(0x3f008081)]);
    assert_eq!(c.decal_desc().get_scalar(0x4c), Some(0.5));
    assert!(c
        .decal_appearance(
            Appearance::Alpha,
            &mut tokens(&["alpha255", "255", "128", "-vary", "2"]),
            &mut Ascii,
            &mut floats(&[("255", 255.), ("128", 128.), ("2", 2.)])
        )
        .is_err());
    assert_eq!(c.decal_desc().floats[&0x28], vec![255., 128.]);
    let mut h = floats(&[("255", 255.)]);
    fail(&mut h, "bad");
    assert!(c
        .decal_appearance(
            Appearance::Alpha,
            &mut tokens(&["alpha255", "255", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.decal_desc().floats[&0x28], vec![255.]);
    let old = c.decal_desc().clone();
    assert!(c
        .decal_appearance(
            Appearance::Alpha,
            &mut tokens(&["alpha"]),
            &mut Ascii,
            &mut Host::default()
        )
        .is_err());
    assert_eq!(*c.decal_desc(), old);
    c.decal_desc().scalar(0xc, 7.);
    c.decal_desc().words.insert(0, 0);
    c.decal_life(
        &mut tokens(&["life", "-sustain", "-single", "-loop", "-static"]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(c.decal_desc().get_scalar(0xc), Some(7.));
    assert_eq!(c.decal_desc().words[&0], 0x40);
    assert_eq!(c.decal_desc().bytes[&9], 3);
    c.decal_life(
        &mut tokens(&["life", "-2", "99"]),
        &mut Ascii,
        &mut floats(&[("-2", -2.)]),
    )
    .unwrap();
    assert_eq!(c.decal_desc().get_scalar(0xc), Some(-2.));
    let old = c.decal_desc().clone();
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .decal_life(&mut tokens(&["life", "bad", "-static"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(*c.decal_desc(), old);
}

#[test]
fn ec176_through_ec183_decal_texture_flags_selection_and_failures() {
    let mut c = fresh();
    c.decal_desc().words.insert(4, 7);
    let mut h = Host::returning(vec![(Call::Enum(bytes("mode"), DECAL_DRAW), Reply::Uint(258))]);
    c.decal_texture(
        &mut tokens(&["texture", "-draw", "mode", "-light", "-water", "-ring"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.decal_desc().bytes[&8], 2);
    assert_eq!(c.decal_desc().words[&0], 0x26);
    c.decal_desc().words.insert(0, 0);
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .decal_texture(&mut tokens(&["texture", "-repeat", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.decal_desc().words[&0], 8);
    assert_eq!(c.decal_desc().get_scalar(0x58), Some(1.));
    let mut h = Host::default();
    h.expected
        .push_back((Call::Vector2(bytes("bad")), Err(Error::Foreign("vector".into()))));
    assert!(c
        .decal_texture(
            &mut tokens(&["texture", "-light", "-offset", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.decal_desc().words[&0] & 2, 2);
    assert_eq!(c.decal_desc().get_scalar(0x5c), Some(0.));
    c.decal_desc().words.insert(4, 0);
    assert!(c
        .decal_texture(&mut tokens(&["texture", "-light"]), &mut Ascii, &mut Host::default())
        .is_err());
    assert_eq!(c.decal_desc().words[&0] & 2, 2);
    c.decal_desc().words.insert(4, 7);
    for eax in [0, -1] {
        c.texture_selected(Decal, 4, &tokens(&["texture", "Missing"]), eax, &[1], &mut Ascii)
            .unwrap();
        assert_eq!(c.decal_desc().words[&4], 7);
    }
    c.id_maps[0].insert(bytes("tex"), 0);
    let t = tokens(&["texture", "TeX", "Missing"]);
    c.texture_selected(Decal, 4, &t, 1, &[1, 2], &mut Ascii).unwrap();
    assert_eq!(c.decal_desc().words[&4], 0);
    assert_eq!(t[1].bytes, b"TeX");
    c.decal_desc().words.insert(4, 7);
    assert_eq!(
        c.decal_texture(
            &mut tokens(&["texture", "MiSs", "-light"]),
            &mut Ascii,
            &mut Host::default()
        ),
        Err(Error::Parse("No such texture: 'MiSs'".into()))
    );
    assert_eq!(c.decal_desc().words[&4], 7);
    assert!(!c.id_maps[0].contains_key(b"miss".as_slice()));
    c.decal_desc().words.insert(0, 1);
    let mut h = floats(&[("-3", -3.)]);
    h.expected
        .push_back((Call::Vector2(bytes("v")), Ok(Reply::Vector2([2., -4.]))));
    c.decal_texture(
        &mut tokens(&["texture", "-repeat", "-3", "-offset", "v"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.decal_desc().words[&0], 9);
    assert_eq!(c.decal_desc().get_scalar(0x58), Some(-3.));
    assert_eq!(c.decal_desc().get_scalar(0x60), Some(-4.));
}

#[test]
fn ec184_through_ec201_particle_resources() {
    let mut c = fresh();
    let mut h = Host::returning(vec![
        (Call::Enum(bytes("mode"), PARTICLE_DRAW), Reply::Uint(261)),
        (Call::Float(bytes("-2")), Reply::Float(-2.)),
    ]);
    c.particle_draw(
        &mut tokens(&["texture", "-draw", "mode", "-light", "-noCull", "-sortOffset", "-2"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.particle_desc().bytes[&0xd4], 5);
    assert_eq!(c.particle_desc().words[&0], 0x11);
    assert_eq!(c.particle_desc().get_scalar(0xd8), Some(-2.));
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .particle_draw(
            &mut tokens(&["texture", "-light", "-noCull", "-sortOffset", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().get_scalar(0xd8), Some(-2.));
    c.definitions.get_mut(Particles).name.clear();
    c.particle_desc().words.insert(0xd0, 7);
    c.particle_texture(
        &mut tokens(&["texture", "-vflip", "-hflip"]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0] & 0x60000000, 0x60000000);
    assert_eq!(c.particle_desc().words[&0xd0], 7);
    c.particle_desc().words.insert(0xd0, 0);
    assert!(c
        .particle_texture(&mut tokens(&["texture", "-light"]), &mut Ascii, &mut Host::default())
        .is_err());
    c.id_maps[0].insert(bytes("tex"), 9);
    c.particle_texture(
        &mut tokens(&["texture", "TeX", "Missing"]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0xd0], 9);
    assert_eq!(
        c.particle_texture(
            &mut tokens(&["texture", "MiSs", "-hflip"]),
            &mut Ascii,
            &mut Host::default()
        ),
        Err(Error::Parse("No such texture: 'MiSs'".into()))
    );
    assert_eq!(c.particle_desc().words[&0xd0], 9);
    let old = c.particle_desc().clone();
    assert!(c
        .particle_model(
            &mut tokens(&["model", "-draw", "bad"]),
            &mut Ascii,
            &mut Host::default()
        )
        .is_err());
    assert_eq!(*c.particle_desc(), old);
    c.definitions.get_mut(Particles).name = bytes("P");
    c.particle_desc().words.insert(8, 0);
    c.particle_desc().words.insert(0xd0, 7);
    c.particle_desc().lists.insert(0x1e0, vec![1, 2]);
    c.particle_model(&mut tokens(&["model"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.particle_desc().words[&8], 1);
    assert_eq!(c.particle_desc().bytes[&0xd4], 3);
    assert_eq!(c.particle_desc().lists[&0x1e0], vec![1, 2]);
    let mut h = Host::returning(vec![(Call::Enum(bytes("mode"), PARTICLE_DRAW), Reply::Uint(7))]);
    c.particle_model(&mut tokens(&["model", "-draw", "mode"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.particle_desc().bytes[&0xd4], 7);
    let mut t = tokens(&["model", "-slaveApplyAlpha", "-applyAlpha"]);
    c.particle_model(&mut t, &mut Ascii, &mut Host::default()).unwrap();
    assert_eq!(c.particle_desc().words[&8] & 0x11, 0x11);
    assert!(check_switches(&t, &mut Ascii).is_err());
    c.particle_model(
        &mut tokens(&["model", "-modelSpeed", "2", "-modelSpeedStatic", "-3"]),
        &mut Ascii,
        &mut floats(&[("2", 2.), ("-3", -3.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().get_scalar(0x1dc), Some(-3.));
    assert_ne!(c.particle_desc().words[&8] & 0x40, 0);
    c.particle_desc().words.insert(8, 0);
    c.p_set(0x1dc, 7.);
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .particle_model(&mut tokens(&["model", "-modelSpeed", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.particle_desc().words[&8], 1);
    assert_eq!(c.particle_desc().get_scalar(0x1dc), Some(7.));
    let mut h = floats(&[("2", 2.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_model(
            &mut tokens(&["model", "-modelSpeed", "2", "-modelSpeedStatic", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().words[&8], 0x41);
    assert_eq!(c.particle_desc().get_scalar(0x1dc), Some(2.));
    c.particle_desc().words.insert(8, 0);
    c.particle_model(
        &mut tokens(&[
            "model",
            "-fakePerspective",
            "-moveEntireSlave",
            "-sustain",
            "-applyLighting",
            "-noCullFaces",
        ]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&8], 0x42f);
    c.id_maps[1].insert(bytes("a"), 9);
    c.id_maps[1].insert(bytes("b"), 0);
    c.particle_model(&mut tokens(&["model", "A"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.particle_desc().words[&0xd0], 9);
    assert!(c.particle_desc().lists[&0x1e0].is_empty());
    c.particle_desc().words.insert(0xd0, 7);
    c.particle_model(&mut tokens(&["model", "A", "B", "a"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.particle_desc().words[&0xd0], 7);
    assert_eq!(c.particle_desc().lists[&0x1e0], vec![9, 0, 9]);
    c.particle_desc().words.insert(8, 0);
    assert!(c
        .particle_model(
            &mut tokens(&["model", "A", "Missing", "-fakePerspective"]),
            &mut Ascii,
            &mut Host::default()
        )
        .is_err());
    assert_eq!(c.particle_desc().words[&8], 3);
    assert_eq!(c.particle_desc().lists[&0x1e0], vec![9, 0, 9]);
    c.particle_model_selected(&tokens(&["model"]), 1, &[], &mut Ascii)
        .unwrap();
    assert!(c.particle_desc().lists[&0x1e0].is_empty());
    assert_eq!(c.particle_desc().words[&0xd0], 7);
}

#[test]
fn ec202_through_ec208_timed_effect_insertion_and_unordered_times() {
    let mut c = fresh();
    c.definitions.get_mut(Particles).name.clear();
    c.particle_desc().timed_effects = vec![(bytes("A"), 1.), (bytes("B"), 3.), (bytes("C"), 3.)];
    let old = c.particle_desc().timed_effects.clone();
    assert!(c
        .particle_timed(&tokens(&["timedEffect", "X"]), &mut Ascii, &mut Host::default())
        .is_err());
    assert_eq!(c.particle_desc().timed_effects, old);
    assert_eq!(c.particle_desc().words[&0], 0x80000000);
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .particle_timed(&tokens(&["timedEffect", "MiXeD", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.particle_desc().timed_effects, old);
    c.particle_timed(
        &tokens(&["timedEffect", "MiXeD", "3", "99"]),
        &mut Ascii,
        &mut floats(&[("3", 3.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().timed_effects.last(), Some(&(bytes("MiXeD"), 3.)));
    c.particle_timed(
        &tokens(&["timedEffect", "X", "-2"]),
        &mut Ascii,
        &mut floats(&[("-2", -2.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().timed_effects[0], (bytes("X"), -2.));
    c.particle_desc().timed_effects = vec![(bytes("A"), 1.), (bytes("N"), f32::NAN), (bytes("B"), 3.)];
    c.particle_timed(
        &tokens(&["timedEffect", "X", "2"]),
        &mut Ascii,
        &mut floats(&[("2", 2.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().timed_effects[2], (bytes("X"), 2.));
    c.particle_timed(
        &tokens(&["timedEffect", "N", "nan"]),
        &mut Ascii,
        &mut floats(&[("nan", f64::NAN)]),
    )
    .unwrap();
    assert!(c.particle_desc().timed_effects.last().unwrap().1.is_nan());
    let t = tokens(&["timedEffect", "X", "2", "-unknown"]);
    c.particle_timed(&t, &mut Ascii, &mut floats(&[("2", 2.)])).unwrap();
    assert!(check_switches(&t, &mut Ascii).is_err());
    assert_eq!(c.particle_desc().timed_effects[3], (bytes("X"), 2.));
}
