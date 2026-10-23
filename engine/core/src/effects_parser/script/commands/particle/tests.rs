// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::{bytes, Host};
use super::*;
use crate::effects_parser::script::tests::tokens;

fn fresh() -> Commands {
    let mut c = Commands::default();
    let d = c.definitions.get_mut(Particles);
    d.name = bytes("P");
    d.description = Description::reset(Particles, &Description::default());
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
fn scalar(c: &Commands, at: u32) -> f32 {
    c.definitions.get(Particles).description.get_scalar(at).unwrap()
}
fn fail(h: &mut Host, s: &str) {
    h.expected
        .push_back((Call::Float(bytes(s)), Err(Error::Foreign("conversion".into()))));
}
fn life(c: &mut Commands, words: &[&str], h: &mut Host) -> Result<()> {
    c.particle_life(&mut tokens(words), &mut Ascii, h)
}

#[test]
fn ec72_through_ec78_raw_scalar_and_life() {
    let mut c = fresh();
    c.definitions.get_mut(Particles).name.clear();
    let old = c.particle_desc().clone();
    assert!(c
        .particle_zoom_stretch(false, &tokens(&["zoom"]), &mut Host::default())
        .is_err());
    let mut h = Host::returning(vec![(Call::Uint(bytes("9")), Reply::Uint(9))]);
    c.particle_zoom_stretch(false, &tokens(&["zoom", "9", "ignored"]), &mut h)
        .unwrap();
    h.done();
    assert_eq!(*c.particle_desc(), old);
    c.particle_zoom_stretch(
        true,
        &tokens(&["stretch", "-2", "ignored"]),
        &mut floats(&[("-2", -2.)]),
    )
    .unwrap();
    assert_eq!(scalar(&c, 0xdc), -2.);
    for (r, expected, extra) in [(2., [3., 7., 5.], false), (-2., [7., 3., 9.], true)] {
        let rtext = if r > 0. { "2" } else { "-2" };
        let mut words = vec!["life", "5", rtext, "ignored"];
        let mut h = floats(&[("5", 5.), (rtext, r)]);
        if extra {
            words.extend(["-preroll", "9"]);
            h.expected.push_back((Call::Float(bytes("9")), Ok(Reply::Float(9.))));
        }
        life(&mut c, &words, &mut h).unwrap();
        h.done();
        assert_eq!([scalar(&c, 0xc), scalar(&c, 0x10), scalar(&c, 0x1c)], expected);
    }
    c.p_set(0x1c, 99.);
    let mut h = floats(&[("5", 5.), ("2", 2.)]);
    fail(&mut h, "bad");
    assert!(life(&mut c, &["life", "5", "2", "-preroll", "bad"], &mut h).is_err());
    h.done();
    assert_eq!(scalar(&c, 0x1c), 99.);
    assert_eq!(scalar(&c, 0xc), 3.);
    life(
        &mut c,
        &["life", "m", "0"],
        &mut floats(&[("m", 1. + 2f64.powi(-24)), ("0", 0.)]),
    )
    .unwrap();
    assert_eq!(scalar(&c, 0xc), 1.);
    assert_eq!(scalar(&c, 0x1c), 1.);
}

#[test]
fn ec79_through_ec86_rate_prefix_modes_and_options() {
    let mut c = fresh();
    c.particle_desc().floats.insert(0x88, vec![9., 8., 7.]);
    let mut h = floats(&[("2", 2.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_rate(0, &mut tokens(&["rate", "2", "bad", "4"]), &mut Ascii, &mut h)
        .is_err());
    h.done();
    assert_eq!(c.particle_desc().floats[&0x88], vec![2.]);
    c.particle_rate(
        0,
        &mut tokens(&["rate", "100", "-inject", "6", "7"]),
        &mut Ascii,
        &mut floats(&[("6", 6.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().floats[&0x88], vec![6.]);
    assert_eq!(c.particle_desc().words[&0] & 2, 2);
    c.particle_desc().floats.insert(0x88, vec![9., 8., 7.]);
    c.particle_desc().words.insert(0, 0);
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .particle_rate(2, &mut tokens(&["inject", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.particle_desc().floats[&0x88], vec![9.]);
    assert_eq!(c.particle_desc().words[&0], 0);
    c.particle_desc().words.insert(0, 2);
    c.particle_rate(
        4,
        &mut tokens(&["maintain", "8", "99", "-delay", "7", "3"]),
        &mut Ascii,
        &mut floats(&[("8", 8.), ("7", 7.), ("3", 3.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0], 6);
    assert_eq!([scalar(&c, 0x20), scalar(&c, 0x24)], [7., 3.]);
    let mut h = floats(&[("4", 4.), ("8", 8.)]);
    h.expected.push_back((Call::Uint(bytes("6")), Ok(Reply::Uint(6))));
    c.particle_desc().words.insert(0, 0);
    c.particle_rate(
        2,
        &mut tokens(&[
            "inject",
            "4",
            "-volumeScale",
            "-loop",
            "8",
            "6",
            "-single",
            "-sustain",
            "-scale",
            "-areaScale",
        ]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.particle_desc().words[&0], 0x38000a);
    assert_eq!(scalar(&c, 0x14).to_bits(), 0x3a83126f);
    assert_eq!(c.particle_desc().words[&0x18], 1);
    c.particle_desc().words.insert(0x18, 7);
    let mut h = floats(&[("4", 4.), ("8", 8.)]);
    h.expected
        .push_back((Call::Uint(bytes("bad")), Err(Error::Foreign("uint".into()))));
    let mut t = tokens(&["rate", "4", "-loop", "8", "bad", "-single"]);
    assert!(c.particle_rate(0, &mut t, &mut Ascii, &mut h).is_err());
    h.done();
    assert_eq!(scalar(&c, 0x14), 8.);
    assert_eq!(c.particle_desc().words[&0x18], 7);
    assert!(check_switches(&t, &mut Ascii).is_err());
    c.particle_rate(
        2,
        &mut tokens(&["inject", "4", "-loop", "8"]),
        &mut Ascii,
        &mut floats(&[("4", 4.), ("8", 8.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0x18], 7);
    let mut t = tokens(&["inject", "4", "-delay", "6", "-trigger", "9", "2", "-retrigger", "8"]);
    c.particle_rate(
        2,
        &mut t,
        &mut Ascii,
        &mut floats(&[("4", 4.), ("6", 6.), ("9", 9.), ("2", 2.)]),
    )
    .unwrap();
    assert_eq!(
        [scalar(&c, 0x20), scalar(&c, 0x24), scalar(&c, 0x28), scalar(&c, 0x2c)],
        [6., 6., 9., 2.]
    );
    assert!(check_switches(&t, &mut Ascii).is_err());
    let old = c.particle_desc().floats[&0x88].clone();
    c.particle_rate_selected(0, &mut tokens(&["rate"]), 0, &[], &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.particle_desc().floats[&0x88], old);
}

#[test]
fn ec87_through_ec90_alignment_guard_and_aliases() {
    let mut c = fresh();
    c.definitions.get_mut(Particles).name.clear();
    assert_eq!(
        c.particle_align(&mut tokens(&["align"]), &mut Ascii, &mut Host::default()),
        Err(Error::Parse("Not in particles block".into()))
    );
    c.definitions.get_mut(Particles).name = bytes("P");
    let mut h = Host::returning(vec![(Call::Enum(bytes("dirZ"), ALIGN), Reply::Uint(4))]);
    c.particle_align(&mut tokens(&["align", "dirZ", "ignored"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.particle_desc().bytes[&0xd5], 4);
    h.done();
    let mut h = Host::returning(vec![(Call::Enum(bytes("ground"), ALIGN), Reply::Uint(1))]);
    h.expected.extend(floats(&[("2", 2.), ("3", 3.), ("4", 4.)]).expected);
    let mut t = tokens(&[
        "align",
        "ground",
        "-damp",
        "2",
        "-bank",
        "3",
        "4",
        "-windBank",
        "8",
        "9",
    ]);
    c.particle_align(&mut t, &mut Ascii, &mut h).unwrap();
    h.done();
    assert_eq!([scalar(&c, 0x198), scalar(&c, 0x19c), scalar(&c, 0x1a0)], [2., 3., 4.]);
    assert_eq!(c.particle_desc().words[&8] & 0x80, 0);
    assert!(check_switches(&t, &mut Ascii).is_err());
    let mut h = Host::returning(vec![(Call::Enum(bytes("ground"), ALIGN), Reply::Uint(1))]);
    h.expected.extend(floats(&[("3", 3.)]).expected);
    fail(&mut h, "bad");
    assert!(c
        .particle_align(
            &mut tokens(&["align", "ground", "-windBank", "3", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().words[&8] & 0x80, 0x80);
    assert_eq!(scalar(&c, 0x1a0), 4.);
}

#[test]
fn ec91_through_ec104_collision_terrain_and_walk() {
    let mut c = fresh();
    c.definitions.get_mut(Particles).name.clear();
    assert!(c
        .particle_collision(
            &mut tokens(&["collide", "-bounce", "bad"]),
            &mut Ascii,
            &mut Host::default()
        )
        .is_err());
    assert_eq!(c.particle_desc().words[&0], 0);
    c.definitions.get_mut(Particles).name = bytes("P");
    c.p_set(0x138, -0.);
    c.particle_collision(&mut tokens(&["collide"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(scalar(&c, 0x138).to_bits(), 0x3e99999a);
    c.p_set(0x138, f32::NAN);
    c.particle_collision(&mut tokens(&["collide"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert!(scalar(&c, 0x138).is_nan());
    c.particle_collision(
        &mut tokens(&[
            "collide",
            "-effect",
            "9",
            "-death",
            ".25",
            "-deathByWater",
            "1",
            "-sticky",
            "-killOutsideCity",
            "-destroyBuildings",
        ]),
        &mut Ascii,
        &mut floats(&[("9", 9.), (".25", 0.25), ("1", 1.)]),
    )
    .unwrap();
    assert_eq!(scalar(&c, 0x150), 0.25);
    assert_eq!(scalar(&c, 0x154), 1.);
    assert_eq!(c.particle_desc().words[&0], 0xb00);
    assert_eq!(c.particle_desc().words[&4], 1);
    assert!(c
        .particle_collision(
            &mut tokens(&["collide", "-effect", "9", "-death", "1.25"]),
            &mut Ascii,
            &mut floats(&[("9", 9.), ("1.25", 1.25)])
        )
        .is_err());
    assert_eq!(scalar(&c, 0x150), 9.);
    assert!(bounded(&mut floats(&[("nan", f64::NAN)]), b"nan", 0., 1.)
        .unwrap()
        .is_nan());
    assert!(bounded(&mut floats(&[("x", 1. + 2f64.powi(-30))]), b"x", 0., 1.).is_err());
    c.definitions.get_mut(Particles).name.clear();
    assert!(c
        .particle_terrain(&mut tokens(&["terrainRepel", "2"]), &mut Ascii, &mut Host::default())
        .is_err());
    assert_ne!(c.particle_desc().words[&0] & 0x400, 0);
    let old = scalar(&c, 0x140);
    let mut h = floats(&[("2", 2.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_terrain(&mut tokens(&["terrainRepel", "2", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(scalar(&c, 0x13c), 2.);
    assert_eq!(scalar(&c, 0x140), old);
    c.particle_terrain(
        &mut tokens(&[
            "terrainRepel",
            "-2",
            "4",
            "99",
            "-scout",
            "5",
            "-vertical",
            "6",
            "-killHeight",
            "7",
        ]),
        &mut Ascii,
        &mut floats(&[("-2", -2.), ("4", 4.), ("5", 5.), ("6", 6.), ("7", 7.)]),
    )
    .unwrap();
    assert_eq!(
        [
            scalar(&c, 0x13c),
            scalar(&c, 0x140),
            scalar(&c, 0x144),
            scalar(&c, 0x148),
            scalar(&c, 0x14c)
        ],
        [-2., 4., 5., 6., 7.]
    );
    let old = c.particle_desc().clone();
    c.particle_walk(&mut tokens(&["randomWalk"]), &mut Ascii, &mut Host::default())
        .unwrap();
    let mut expected = old;
    *expected.words.get_mut(&0).unwrap() |= 0x800000;
    assert_eq!(*c.particle_desc(), expected);
    c.particle_walk(
        &mut tokens(&["randomWalk", "-delay", "5", "2", "-strength", "5", "-2", "-turn", ".75"]),
        &mut Ascii,
        &mut floats(&[("5", 5.), ("2", 2.), ("5", 5.), ("-2", -2.), (".75", 0.75)]),
    )
    .unwrap();
    assert_eq!(
        [
            scalar(&c, 0x174),
            scalar(&c, 0x178),
            scalar(&c, 0x17c),
            scalar(&c, 0x180),
            scalar(&c, 0x184),
            scalar(&c, 0x188)
        ],
        [3., 7., 7., 3., 0.75, 0.]
    );
    assert!(c
        .particle_walk(
            &mut tokens(&["randomWalk", "-turn", ".25", "2", "-wait"]),
            &mut Ascii,
            &mut floats(&[(".25", 0.25), ("2", 2.)])
        )
        .is_err());
    assert_eq!(scalar(&c, 0x184), 0.25);
    assert_eq!(scalar(&c, 0x188), 0.);
    let mut h = Host::returning(vec![(Call::Vector3(bytes("v")), Reply::Vector3([0., 3., 4.]))]);
    c.particle_walk(
        &mut tokens(&["randomWalk", "-wait", "-preferSea", "-preferDir", "v"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0] & 0x3800000, 0x3800000);
    assert_eq!([scalar(&c, 0x18c), scalar(&c, 0x190), scalar(&c, 0x194)], [0., 3., 4.]);
}

#[test]
fn ec105_through_ec119_appearance_precision_prefix_and_suffix() {
    let mut c = fresh();
    c.definitions.get_mut(Particles).name.clear();
    for (kind, word, args, values, expected) in [
        (
            Appearance::Size,
            "size",
            vec!["2", "-1"],
            vec![("2", 2.), ("-1", -1.)],
            vec![100., -50.],
        ),
        (
            Appearance::Size,
            "size",
            vec!["x"],
            vec![("x", 1. + 2f64.powi(-24))],
            vec![f32::from_bits(0x42480001)],
        ),
        (
            Appearance::Alpha,
            "alpha255",
            vec!["255", "128", "-255"],
            vec![("255", 255.), ("128", 128.), ("-255", -255.)],
            vec![1., f32::from_bits(0x3f008081), -1.],
        ),
        (
            Appearance::Alpha,
            "alpha",
            vec!["255", "128"],
            vec![("255", 255.), ("128", 128.)],
            vec![255., 128.],
        ),
        (
            Appearance::Alpha,
            "alpha255",
            vec!["256"],
            vec![("256", 256.)],
            vec![f32::from_bits(0x3f808081)],
        ),
    ] {
        let mut words = vec![word];
        words.extend(args);
        let mut h = floats(&values);
        c.particle_appearance(kind, &mut tokens(&words), &mut Ascii, &mut h)
            .unwrap();
        h.done();
        let at = if kind == Appearance::Size { 0xac } else { 0xa0 };
        assert_eq!(c.particle_desc().floats[&at], expected);
    }
    let mut h = floats(&[("2", 2.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_appearance(
            Appearance::Aspect,
            &mut tokens(&["aspect", "2", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().floats[&0xb8], vec![2.]);
    c.particle_appearance(
        Appearance::Rotate,
        &mut tokens(&["rotate", ".25", "2", "-vary", ".5", "-offset", "-3"]),
        &mut Ascii,
        &mut floats(&[(".25", 0.25), ("2", 2.), (".5", 0.5), ("-3", -3.)]),
    )
    .unwrap();
    assert_eq!(c.particle_desc().floats[&0xc4], vec![0.25, 2.]);
    assert_eq!(scalar(&c, 0x70), 0.5);
    assert_eq!(scalar(&c, 0x74), -3.);
    assert!(c
        .particle_appearance(
            Appearance::Rotate,
            &mut tokens(&["rotate", "2", "-vary", "2", "-offset", "7"]),
            &mut Ascii,
            &mut floats(&[("2", 2.), ("2", 2.)])
        )
        .is_err());
    assert_eq!(scalar(&c, 0x70), 0.5);
    assert_eq!(scalar(&c, 0x74), -3.);
    c.particle_appearance(
        Appearance::Aspect,
        &mut tokens(&["aspect", "2", "-vary", "nan"]),
        &mut Ascii,
        &mut floats(&[("2", 2.), ("nan", f64::NAN)]),
    )
    .unwrap();
    assert!(scalar(&c, 0x6c).is_nan());
    c.particle_appearance(
        Appearance::Alpha,
        &mut tokens(&["alpha255", "255", "128", "-vary", ".5"]),
        &mut Ascii,
        &mut floats(&[("255", 255.), ("128", 128.), (".5", 0.5)]),
    )
    .unwrap();
    assert_eq!(scalar(&c, 0x78), 0.5);
    assert!(c
        .particle_appearance(
            Appearance::Alpha,
            &mut tokens(&["alpha255", "255", "128", "-vary", "2"]),
            &mut Ascii,
            &mut floats(&[("255", 255.), ("128", 128.), ("2", 2.)])
        )
        .is_err());
    assert_eq!(c.particle_desc().floats[&0xa0], vec![255., 128.]);
    let mut h = floats(&[("255", 255.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_appearance(
            Appearance::Alpha,
            &mut tokens(&["alpha255", "255", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().floats[&0xa0], vec![255.]);
    for (name, vary, ok) in [
        ("color255", [1., 0.5, 0.], true),
        ("colour", [1., 0.5, 0.], true),
        ("colour255", [1., 0.5, 0.], true),
        ("color255", [0.5, 2., 0.], false),
        ("color", [f32::NAN, 0.5, 1.], true),
    ] {
        let old = [scalar(&c, 0x7c), scalar(&c, 0x80), scalar(&c, 0x84)];
        let mut h = Host::returning(vec![
            (Call::Color(bytes("v")), Reply::Vector3([255., 128., 0.])),
            (Call::Vector3(bytes("w")), Reply::Vector3(vary)),
        ]);
        assert_eq!(
            c.particle_appearance(
                Appearance::Color,
                &mut tokens(&[name, "v", "-vary", "w"]),
                &mut Ascii,
                &mut h
            )
            .is_ok(),
            ok
        );
        h.done();
        if !ok {
            assert_eq!([scalar(&c, 0x7c), scalar(&c, 0x80), scalar(&c, 0x84)], old);
            assert_eq!(c.particle_desc().vectors[&0x94], vec![[255., 128., 0.]]);
        } else {
            let actual = [scalar(&c, 0x7c), scalar(&c, 0x80), scalar(&c, 0x84)];
            for i in 0..3 {
                assert_eq!(actual[i].to_bits(), vary[i].to_bits());
            }
            assert_eq!(
                c.particle_desc().vectors[&0x94][0][0],
                if name.ends_with("255") { 1. } else { 255. }
            );
        }
    }
}
