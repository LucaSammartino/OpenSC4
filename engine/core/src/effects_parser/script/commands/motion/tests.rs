// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::tests::{bytes, Host};
use super::*;
use crate::effects_parser::script::tests::tokens;
use definitions::DefinitionKind::Particles;
fn fresh() -> Commands {
    let mut c = Commands {
        scalar_498: 1.,
        ..Commands::default()
    };
    c.definitions.get_mut(Particles).description = Description::reset(Particles, &Description::default());
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
fn v(s: &str, x: [f32; 3]) -> (Call, Reply) {
    (Call::Vector3(bytes(s)), Reply::Vector3(x))
}
fn f(s: &str, x: f64) -> (Call, Reply) {
    (Call::Float(bytes(s)), Reply::Float(x))
}
fn fail(h: &mut Host, s: &str) {
    h.expected
        .push_back((Call::Float(bytes(s)), Err(Error::Foreign("conversion".into()))));
}
fn scalar(c: &mut Commands, at: u32) -> f32 {
    c.particle_desc().get_scalar(at).unwrap()
}

#[test]
fn ec209_through_ec219_warp_order_atomic_append_and_uv_boundary() {
    let mut c = fresh();
    c.particle_warp(
        &mut tokens(&["warp", "-screw", "50"]),
        &mut Ascii,
        &mut floats(&[("50", 50.)]),
    )
    .unwrap();
    assert_eq!(scalar(&mut c, 0x104), 1.);
    c.p_set(0x104, 7.);
    let mut h = Host::default();
    fail(&mut h, "bad");
    assert!(c
        .particle_warp(
            &mut tokens(&["warp", "-screw", "bad", "-wiggleVerts"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(scalar(&mut c, 0x104), 7.);
    assert_eq!(c.particle_desc().words[&8], 0);
    let old = [9.; 7];
    c.particle_desc().wiggles.push(old);
    let mut h = Host::returning(vec![
        f("2", 2.),
        f("3", 3.),
        f("4", 4.),
        f("0", 0.),
        (Call::Cos(0.), Reply::Float(1.)),
        (Call::Sin(0.), Reply::Float(0.)),
    ]);
    c.particle_warp(
        &mut tokens(&["warp", "-wiggle", "2", "3", "4", "0"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.particle_desc().wiggles[0], old);
    assert_eq!(c.particle_desc().wiggles[1], [3., 0., 2., 0., 4., 0., -0.]);
    assert_eq!(c.particle_desc().wiggles[1][6].to_bits(), (-0f32).to_bits());
    let before = c.particle_desc().wiggles.clone();
    assert!(c
        .particle_warp(
            &mut tokens(&["warp", "-wiggle", "2", "3", "4", "2"]),
            &mut Ascii,
            &mut floats(&[("2", 2.), ("3", 3.), ("4", 4.), ("2", 2.)])
        )
        .is_err());
    assert_eq!(c.particle_desc().wiggles, before);
    for third in [false, true] {
        let mut words = vec!["warp", "-wiggleDir", "3", "v"];
        let mut calls = vec![f("3", 3.), v("v", [1., 2., 3.])];
        if third {
            words.extend(["u", "ignored"]);
            calls.push(v("u", [4., 5., 6.]));
        }
        let mut h = Host::returning(calls);
        c.particle_warp(&mut tokens(&words), &mut Ascii, &mut h).unwrap();
        h.done();
        assert_eq!(
            *c.particle_desc().wiggles.last().unwrap(),
            if third {
                [3., 4., 5., 6., 1., 2., 3.]
            } else {
                [3., 0., 0., 0., 1., 2., 3.]
            }
        );
    }
    let mut h = Host::returning(vec![
        f("2", 2.),
        f("3", 3.),
        f("4", 4.),
        f("0", 0.),
        (Call::Cos(0.), Reply::Float(1.)),
        (Call::Sin(0.), Reply::Float(0.)),
        f("3", 3.),
        v("v", [1., 2., 3.]),
    ]);
    h.expected
        .push_back((Call::Vector3(bytes("bad")), Err(Error::Foreign("vector".into()))));
    let before = c.particle_desc().wiggles.len();
    assert!(c
        .particle_warp(
            &mut tokens(&["warp", "-wiggle", "2", "3", "4", "0", "-wiggleDir", "3", "v", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().wiggles.len(), before + 1);
    c.particle_desc().words.insert(8, 2);
    c.particle_warp(&mut tokens(&["warp", "-wiggleVerts"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.particle_desc().words[&8], 0x102);
    c.particle_desc().words.insert(8, 0);
    let mut h = Host::returning(vec![f("2", 2.), (Call::Vector2(bytes("v")), Reply::Vector2([3., -4.]))]);
    c.particle_warp(&mut tokens(&["warp", "-uv", "2", "v"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(
        [scalar(&mut c, 0x114), scalar(&mut c, 0x118), scalar(&mut c, 0x11c)],
        [2., 3., -4.]
    );
    assert_eq!(c.particle_desc().words[&8], 0x100);
    c.particle_desc().words.insert(8, 0);
    c.p_set(0x118, 7.);
    c.p_set(0x11c, 8.);
    let mut h = floats(&[("2", 2.)]);
    h.expected
        .push_back((Call::Vector2(bytes("bad")), Err(Error::Foreign("vector".into()))));
    assert!(c
        .particle_warp(&mut tokens(&["warp", "-uv", "2", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(
        [scalar(&mut c, 0x114), scalar(&mut c, 0x118), scalar(&mut c, 0x11c)],
        [2., 7., 8.]
    );
    assert_eq!(c.particle_desc().words[&8], 0);
    assert_eq!(
        c.particle_warp(
            &mut tokens(&["warp", "-uv", "2"]),
            &mut Ascii,
            &mut floats(&[("2", 2.)])
        ),
        Err(Error::Unsupported("unchecked uv second index"))
    );
}

#[test]
fn ec220_through_ec223_warp_alpha_prefix_and_nan() {
    let mut c = fresh();
    let mut h = Host::returning(vec![v("v", [0., 0., 2.]), f("0", 0.), f(".5", 0.5), f("1", 1.)]);
    c.particle_warp(
        &mut tokens(&["warp", "-alpha", "v", "0", ".5", "1"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.read_pv(0x120).unwrap(), [0., 0., 1.]);
    assert_eq!(c.particle_desc().floats[&0x12c], vec![0., 0.5, 1.]);
    assert_eq!(c.particle_desc().words[&8], 0x100);
    c.particle_desc().words.insert(8, 0);
    let mut h = Host::returning(vec![v("v", [0., 0., 2.]), f(".5", 0.5), f("2", 2.)]);
    assert!(c
        .particle_warp(&mut tokens(&["warp", "-alpha", "v", ".5", "2"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.particle_desc().floats[&0x12c], vec![0.5]);
    assert_eq!(c.particle_desc().words[&8], 0);
    let mut h = Host::returning(vec![v("v", [0., 0., 2.]), f("2", 2.)]);
    assert!(c
        .particle_warp(
            &mut tokens(&["warp", "-alpha", "v", "2", "-wiggleVerts"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().words[&8], 0x100);
    assert!(c.particle_desc().floats[&0x12c].is_empty());
    let mut h = Host::returning(vec![v("v", [0., 0., 2.]), f("nan", f64::NAN)]);
    c.particle_warp(&mut tokens(&["warp", "-alpha", "v", "nan"]), &mut Ascii, &mut h)
        .unwrap();
    assert!(c.particle_desc().floats[&0x12c][0].is_nan());
}

#[test]
fn ec224_through_ec233_source_shapes_and_short_circuit() {
    let mut c = fresh();
    for (words, calls, lo, hi) in [
        (vec!["source"], vec![], [-1., 0., -1.], [1., 0., 1.]),
        (
            vec!["source", "-point", "v"],
            vec![v("v", [2., 3., 7.])],
            [2., 3., 7.],
            [2., 3., 7.],
        ),
        (
            vec!["source", "-square", "2", "c"],
            vec![f("2", 2.), v("c", [10., 20., 30.])],
            [8., 20., 28.],
            [12., 20., 32.],
        ),
        (
            vec!["source", "-quad", "v", "c"],
            vec![
                (Call::Vector2(bytes("v")), Reply::Vector2([2., 5.])),
                v("c", [10., 20., 30.]),
            ],
            [8., 20., 25.],
            [12., 20., 35.],
        ),
        (
            vec!["source", "-cube", "-2"],
            vec![f("-2", -2.)],
            [2., 2., 2.],
            [-2., -2., -2.],
        ),
        (
            vec!["source", "-box", "v", "c"],
            vec![v("v", [2., 3., 5.]), v("c", [10., 20., 30.])],
            [8., 17., 25.],
            [12., 23., 35.],
        ),
        (vec!["source", "-point", "-square", "bad"], vec![], [0.; 3], [0.; 3]),
        (vec!["source", "-dice", "4"], vec![f("4", 4.)], [0.; 3], [0.; 3]),
        (
            vec!["source", "-cityWindySide", "8", "-3"],
            vec![f("8", 8.), f("-3", -3.)],
            [-1., 8., -1.],
            [1., -3., 1.],
        ),
    ] {
        let mut t = tokens(&words);
        let mut h = Host::returning(calls);
        c.particle_source(&mut t, &mut Ascii, &mut h).unwrap();
        h.done();
        assert_eq!(c.read_pv(0x50).unwrap(), lo);
        assert_eq!(c.read_pv(0x5c).unwrap(), hi);
        if words.contains(&"-square") && words.contains(&"-point") {
            assert!(check_switches(&t, &mut Ascii).is_err());
        }
    }
    assert_eq!(scalar(&mut c, 0x1d8), 4.);
    assert_eq!(c.particle_desc().words[&0] & 0x3000, 0x3000);
    c.p_or(0, 0x40);
    c.particle_source(&mut tokens(&["source", "-point"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.read_pv(0x50).unwrap(), [0.; 3]);
    assert_eq!(c.particle_desc().words[&0] & 0x40, 0x40);
}

#[test]
fn ec234_through_ec236_source_secondary_order_and_failure_prefix() {
    let mut c = fresh();
    let mut h = floats(&[("4", 4.), ("3", 3.), ("9", 9.), ("2", 2.)]);
    c.particle_source(
        &mut tokens(&[
            "source",
            "-heightRange",
            "9",
            "2",
            "-aboveHeight",
            "3",
            "-belowHeight",
            "4",
            "-lakeOnly",
            "-seaOnly",
            "-waterOnly",
            "-terrainOnly",
        ]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(scalar(&mut c, 0x158), 9.);
    assert_eq!(scalar(&mut c, 0x15c), 2.);
    assert_eq!(c.particle_desc().words[&0], 0x70000);
    let mut c = fresh();
    let old = scalar(&mut c, 0x15c);
    let mut h = floats(&[("9", 9.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_source(
            &mut tokens(&["source", "-heightRange", "9", "bad", "-killOutsideCity"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(scalar(&mut c, 0x158), 9.);
    assert_eq!(scalar(&mut c, 0x15c), old);
    assert_eq!(c.particle_desc().words[&0], 0x10000);
    assert_eq!(c.read_pv(0x50).unwrap(), [-1., 0., -1.]);
    let mut c = fresh();
    c.particle_source(
        &mut tokens(&[
            "source",
            "-scaleParticles",
            "-pinToTerrain",
            "-pinToWater",
            "-killOutsideCity",
            "-resetIncoming",
        ]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(c.particle_desc().words[&0], 0x40c800);
    assert_eq!(c.particle_desc().words[&8], 0x200);
}

#[test]
fn ec237_through_ec242_emit_rates_and_velocity() {
    let mut c = fresh();
    let old = c.particle_desc().clone();
    c.particle_emit(&mut tokens(&["emit"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(*c.particle_desc(), old);
    let mut t = tokens(&["emit", "-inject", "3", "-rate", "4", "5"]);
    c.particle_emit(&mut t, &mut Ascii, &mut floats(&[("3", 3.)])).unwrap();
    assert_eq!(c.particle_desc().floats[&0x88], vec![3.]);
    assert_eq!(c.particle_desc().words[&0], 2);
    assert!(check_switches(&t, &mut Ascii).is_err());
    let mut h = floats(&[("7", 7.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_emit(
            &mut tokens(&["emit", "-rate", "7", "bad", "-loop", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(c.particle_desc().floats[&0x88], vec![7.]);
    let mut h = Host::returning(vec![v("v", [0., 2., 0.]), f("8", 8.), f(".25", 0.25)]);
    c.particle_emit(&mut tokens(&["emit", "-velocity", "v", "8", ".25"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.read_pv(0x30).unwrap(), [-0.25, 0.75, -0.25]);
    assert_eq!(c.read_pv(0x3c).unwrap(), [0.25, 1.25, 0.25]);
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [6., 10.]);
    let mut h = Host::returning(vec![v("v", [0.; 3])]);
    c.particle_emit(&mut tokens(&["emit", "-velocity", "v"]), &mut Ascii, &mut h)
        .unwrap();
    assert!(c.read_pv(0x30).unwrap().iter().all(|x| x.is_nan()));
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [1., 1.]);
    let mut h = Host::returning(vec![
        v("v", [0., 1., 0.]),
        f("4", 4.),
        f(".5", 0.5),
        f("10", 10.),
        f("2", 2.),
    ]);
    c.particle_emit(
        &mut tokens(&["emit", "-velocity", "v", "4", ".5", "-speed", "10", "2"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.read_pv(0x30).unwrap(), [-0.5, 0.5, -0.5]);
    assert_eq!(c.read_pv(0x3c).unwrap(), [0.5, 1.5, 0.5]);
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [8., 12.]);
}

#[test]
fn ec243_through_ec249_emit_direction_shapes_and_overrides() {
    let mut c = fresh();
    let mut h = Host::returning(vec![
        v("v", [0.; 3]),
        (Call::SplitArguments(bytes("r")), Reply::Arguments(vec![bytes("2")])),
        f("2", 2.),
    ]);
    c.particle_emit(&mut tokens(&["emit", "-dir", "v", "r"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.read_pv(0x30).unwrap(), [-2.; 3]);
    assert_eq!(c.read_pv(0x3c).unwrap(), [2.; 3]);
    let mut h = Host::returning(vec![
        v("v", [1., 0., 0.]),
        (
            Call::SplitArguments(bytes("r")),
            Reply::Arguments(vec![bytes("2"), bytes("3"), bytes("4")]),
        ),
        f("4", 4.),
        f("3", 3.),
        f("2", 2.),
    ]);
    c.particle_emit(&mut tokens(&["emit", "-dir", "v", "r"]), &mut Ascii, &mut h)
        .unwrap();
    h.done();
    assert_eq!(c.read_pv(0x30).unwrap(), [-1., -3., -4.]);
    assert_eq!(c.read_pv(0x3c).unwrap(), [3., 3., 4.]);
    let mut h = Host::returning(vec![
        v("v", [0., 1., 0.]),
        v("v", [1., 0., 0.]),
        (
            Call::SplitArguments(bytes("r")),
            Reply::Arguments(vec![bytes("2"), bytes("3")]),
        ),
    ]);
    assert_eq!(
        c.particle_emit(
            &mut tokens(&["emit", "-velocity", "v", "-dir", "v", "r", "-vel", "bad", "-base"]),
            &mut Ascii,
            &mut h
        ),
        Err(Error::Parse(
            "Expecting float or vector3 for second -dir argument".into()
        ))
    );
    assert_eq!(c.read_pv(0x30).unwrap(), [0., 1., 0.]);
    let mut h = Host::returning(vec![f("9", 9.), v("d", [0., 0., 1.]), v("v", [3., 4., 0.])]);
    c.particle_emit(
        &mut tokens(&["emit", "-speed", "9", "-dir", "d", "-vel", "v"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    h.done();
    assert_eq!(c.read_pv(0x30).unwrap(), [0.6, 0.8, 0.]);
    assert_eq!(c.read_pv(0x3c).unwrap(), [0.6, 0.8, 0.]);
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [5., 5.]);
    let mut h = Host::returning(vec![v("v", [0.; 3])]);
    c.particle_emit(&mut tokens(&["emit", "-vel", "v", "-base"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.read_pv(0x30).unwrap(), [0.; 3]);
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [0., 0.]);
    assert_eq!(c.particle_desc().words[&0] & 0x20, 0x20);
    c.particle_emit(
        &mut tokens(&["emit", "-speed", "-2", "-4"]),
        &mut Ascii,
        &mut floats(&[("-2", -2.), ("-4", -4.)]),
    )
    .unwrap();
    assert_eq!([scalar(&mut c, 0x48), scalar(&mut c, 0x4c)], [2., -6.]);
    let mut h = Host::returning(vec![f("2", 2.), (Call::Uint(bytes("5")), Reply::Uint(5))]);
    c.particle_emit(&mut tokens(&["emit", "-loop", "2", "5", "-single"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(c.particle_desc().words[&0x14], 0x3a83126f);
    assert_eq!(c.particle_desc().words[&0x18], 1);
}

#[test]
fn ec250_through_ec258_force_acceleration_and_attraction_failure_prefix() {
    let mut c = fresh();
    let old = c.particle_desc().clone();
    c.particle_force(&mut tokens(&["force"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(*c.particle_desc(), old);
    c.pv(0xe0, [3., 4., 5.]);
    c.particle_desc().tractors.push([1.; 8]);
    c.particle_desc().floats.insert(0x1a4, vec![8.]);
    c.word_494 = 9f32.to_bits();
    c.scalar_498 = 2.;
    c.particle_force(&mut tokens(&["force", "-reset"]), &mut Ascii, &mut Host::default())
        .unwrap();
    assert_eq!(c.read_pv(0xe0).unwrap(), [0.; 3]);
    assert!(c.particle_desc().tractors.is_empty());
    assert_eq!(c.particle_desc().floats[&0x1a4], vec![8.]);
    assert_eq!(f32::from_bits(c.word_494), 9.);
    assert_eq!(c.scalar_498, 2.);
    c.pv(0xe0, [1., 2., 3.]);
    let mut h = Host::returning(vec![f("4", 4.), v("v", [0., 3., 0.]), f("6", 6.)]);
    c.particle_force(
        &mut tokens(&["force", "-gravity", "4", "-wind", "v", "6"]),
        &mut Ascii,
        &mut h,
    )
    .unwrap();
    assert_eq!(c.read_pv(0xe0).unwrap(), [1., 4., 3.]);
    let mut h = Host::returning(vec![v("v", [0.; 3])]);
    c.particle_force(&mut tokens(&["force", "-reset", "-wind", "v"]), &mut Ascii, &mut h)
        .unwrap();
    assert!(c.read_pv(0xe0).unwrap().iter().all(|x| x.is_nan()));
    c.particle_force(
        &mut tokens(&["force", "-screw", "50", "-global_wind", "-2", "-drag", "-3"]),
        &mut Ascii,
        &mut floats(&[("-2", -2.), ("50", 50.), ("-3", -3.)]),
    )
    .unwrap();
    assert_eq!(
        [scalar(&mut c, 0x104), scalar(&mut c, 0xec), scalar(&mut c, 0x100)],
        [1., -2., -3.]
    );
    let mut h = floats(&[("7", 7.)]);
    h.expected
        .push_back((Call::Vector3(bytes("bad")), Err(Error::Foreign("vector".into()))));
    assert!(c
        .particle_force(
            &mut tokens(&["force", "-bomb", "7", "bad", "-drag", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(scalar(&mut c, 0xf0), 7.);
    let mut t = tokens(&["force", "-motherDuck", "2", "3", "4", "-attractor", "bad"]);
    c.particle_force(&mut t, &mut Ascii, &mut floats(&[("2", 2.), ("3", 3.), ("4", 4.)]))
        .unwrap();
    assert_eq!(scalar(&mut c, 0x1b0), 2.);
    assert_eq!(c.particle_desc().floats[&0x1a4], vec![3., 4.]);
    assert_ne!(c.particle_desc().words[&0] & 0x8000000, 0);
    assert!(check_switches(&t, &mut Ascii).is_err());
    c.particle_force(
        &mut tokens(&["force", "-alphaAttractor", "5"]),
        &mut Ascii,
        &mut floats(&[("5", 5.)]),
    )
    .unwrap();
    assert_eq!(scalar(&mut c, 0x1b0), 5.);
    assert!(c.particle_desc().floats[&0x1a4].is_empty());
    assert_ne!(c.particle_desc().words[&0] & 0x4000000, 0);
    assert_ne!(c.particle_desc().words[&8] & 0x10, 0);
    let mut c = fresh();
    let mut h = floats(&[("2", 2.), ("3", 3.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_force(
            &mut tokens(&["force", "-motherDuck", "2", "3", "bad", "-tractor", "bad", "bad"]),
            &mut Ascii,
            &mut h
        )
        .is_err());
    assert_eq!(scalar(&mut c, 0x1b0), 2.);
    assert_eq!(c.particle_desc().floats[&0x1a4], vec![3.]);
    assert_eq!(c.particle_desc().words[&0], 0);
}

#[test]
fn ec259_through_ec264_force_automata_terrain_and_explosion() {
    let mut c = fresh();
    c.particle_desc().floats.insert(0x1a4, vec![8.]);
    let old = c.particle_desc().clone();
    c.particle_force(
        &mut tokens(&["force", "-automata", "bad", "bad"]),
        &mut Ascii,
        &mut Host::default(),
    )
    .unwrap();
    assert_eq!(*c.particle_desc(), old);
    c.particle_desc().floats.get_mut(&0x1a4).unwrap().clear();
    let mut h = Host::returning(vec![(Call::Uint(bytes("23")), Reply::Uint(23)), f("6", 6.)]);
    c.particle_force(&mut tokens(&["force", "-automata", "23", "6"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(scalar(&mut c, 0x1b0), 1.);
    assert_eq!(c.particle_desc().words[&0x1b4], 23);
    assert_eq!(c.particle_desc().floats[&0x1a4], vec![6.]);
    let mut h = Host::returning(vec![(Call::Uint(bytes("23")), Reply::Uint(23))]);
    fail(&mut h, "bad");
    assert!(c
        .particle_force(&mut tokens(&["force", "-automata", "23", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert!(c.particle_desc().floats[&0x1a4].is_empty());
    c.particle_force(
        &mut tokens(&["force", "-terrainRepel", "2", "3", "4", "5", "6"]),
        &mut Ascii,
        &mut floats(&[("2", 2.), ("3", 3.), ("4", 4.), ("5", 5.), ("6", 6.)]),
    )
    .unwrap();
    for (at, x) in [(0x13c, 2.), (0x140, 3.), (0x144, 4.), (0x148, 5.), (0x14c, 6.)] {
        assert_eq!(scalar(&mut c, at), x);
    }
    assert_eq!(c.particle_desc().words[&0] & 0x400, 0x400);
    c.particle_desc().words.insert(0, 0);
    let mut h = floats(&[("2", 2.)]);
    fail(&mut h, "bad");
    assert!(c
        .particle_force(&mut tokens(&["force", "-terrainRepel", "2", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(scalar(&mut c, 0x13c), 2.);
    assert_eq!(c.particle_desc().words[&0], 0);
    c.particle_force(
        &mut tokens(&["force", "-explosion", "7", "2", "3", "-explosionFront", "4", "5"]),
        &mut Ascii,
        &mut floats(&[("7", 7.), ("2", 2.), ("3", 3.), ("4", 4.), ("5", 5.)]),
    )
    .unwrap();
    assert_eq!(scalar(&mut c, 0x1ec), 7.);
    assert_eq!(c.particle_desc().floats[&0x1f4], vec![2., 3.]);
    assert_eq!([scalar(&mut c, 0x200), scalar(&mut c, 0x1f0)], [4., 5.]);
}

#[test]
fn ec265_through_ec271_force_tractor_geometry_time_and_compounding() {
    let mut c = fresh();
    c.pv(0x50, [0.; 3]);
    c.pv(0x5c, [0.; 3]);
    c.scalar_498 = 2.;
    c.word_494 = 3f32.to_bits();
    let mut h = Host::returning(vec![
        v("p", [0., 4., 0.]),
        (Call::CountArguments(bytes("6")), Reply::Uint(1)),
        f("6", 6.),
        f("5", 5.),
    ]);
    c.particle_force(&mut tokens(&["force", "-tractor", "p", "6", "5"]), &mut Ascii, &mut h)
        .unwrap();
    h.done();
    assert_eq!(c.particle_desc().tractors[0], [0., 4., 0., 0., 3., 0., 3., 13.]);
    assert_eq!(f32::from_bits(c.word_494), 13.);
    assert_eq!(c.particle_desc().words[&0] & 0x10000000, 0x10000000);
    c.scalar_498 = 1.;
    c.particle_desc().tractors = vec![[10., 0., 0., 0., 0., 0., 0., 0.]];
    let mut h = Host::returning(vec![
        v("p", [2., 0., 0.]),
        (Call::CountArguments(bytes("4")), Reply::Int(1)),
        f("4", 4.),
    ]);
    let mut t = tokens(&["force", "-tractorRel", "p", "4", "-tractor", "bad", "bad"]);
    c.particle_force(&mut t, &mut Ascii, &mut h).unwrap();
    assert_eq!(
        *c.particle_desc().tractors.last().unwrap(),
        [12., 0., 0., -4., 0., 0., 13., 0.]
    );
    assert!(check_switches(&t, &mut Ascii).is_err());
    c.scalar_498 = 2.;
    c.word_494 = 7f32.to_bits();
    let mut h = Host::returning(vec![
        v("p", [1., 2., 3.]),
        (Call::CountArguments(bytes("v")), Reply::Uint(3)),
        v("v", [4., 6., 8.]),
    ]);
    c.particle_force(&mut tokens(&["force", "-tractor", "p", "v"]), &mut Ascii, &mut h)
        .unwrap();
    assert_eq!(
        *c.particle_desc().tractors.last().unwrap(),
        [1., 2., 3., 2., 3., 4., 7., 0.]
    );
    assert_eq!(f32::from_bits(c.word_494), 7.);
    c.word_494 = 3f32.to_bits();
    c.p_set(0xc, 4.);
    c.p_set(0x10, 6.);
    c.particle_desc().tractors = vec![[9., 8., 7., 2., 4., 6., 1., 3.]];
    for (time, life, tangent, times) in [
        (6., [8., 12.], [1., 2., 3.], [2., 6.]),
        (12., [16., 24.], [0.5, 1., 1.5], [4., 12.]),
    ] {
        c.particle_force(
            &mut tokens(&["force", "-tractorTimeScale", "2"]),
            &mut Ascii,
            &mut floats(&[("2", 2.)]),
        )
        .unwrap();
        assert_eq!(c.scalar_498, 2.);
        assert_eq!(f32::from_bits(c.word_494), time);
        assert_eq!([scalar(&mut c, 0xc), scalar(&mut c, 0x10)], life);
        let r = c.particle_desc().tractors[0];
        assert_eq!(&r[0..3], &[9., 8., 7.]);
        assert_eq!(&r[3..6], &tangent);
        assert_eq!(&r[6..8], &times);
    }
    c.word_494 = 7f32.to_bits();
    c.particle_desc().words.insert(0, 0);
    let old = c.particle_desc().tractors.clone();
    let mut h = Host::returning(vec![
        v("p", [1., 2., 3.]),
        (Call::CountArguments(bytes("v")), Reply::Uint(3)),
        v("v", [4., 6., 8.]),
    ]);
    fail(&mut h, "bad");
    assert!(c
        .particle_force(&mut tokens(&["force", "-tractor", "p", "v", "bad"]), &mut Ascii, &mut h)
        .is_err());
    assert_eq!(c.particle_desc().tractors, old);
    assert_eq!(f32::from_bits(c.word_494), 7.);
    assert_eq!(c.particle_desc().words[&0], 0);
}
