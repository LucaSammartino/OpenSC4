// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects::transform::ParentTransform;

#[test]
fn identity_parent_uses_local_translation_directly() {
    let local = [-0., 3., 7.];
    let position = ParentTransform::default().world_position(local);
    assert_eq!(position.map(f32::to_bits), local.map(f32::to_bits));
}

#[test]
fn semantic_transform_rotates_scales_then_translates_both_owners() {
    let parent = ParentTransform {
        rotation: [[0., 0., 1.], [0., 1., 0.], [-1., 0., 0.]],
        scale: 2.,
        translation: [100., 30., 200.],
    };
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 2,
    };
    let mut rng = crate::rng::RzRandom::new(123, &seeds);
    let mut brush = BrushEffect::new(&BRUSHES[0], &mut rng);
    let mut game = GameEffect::new(&GameRecord::default());
    brush.set_transform([8., 4., 12.], &parent);
    game.set_transform([8., 4., 12.], &parent);
    assert_eq!(parent.world_position([8., 4., 12.]), [124., 38., 184.]);
    assert_eq!(brush.cur, (124., 184.));
    assert_eq!(game.cell, (7, 11));
    game.pending = false;
    game.set_transform([8.5, 4., 12.5], &parent);
    assert!(!game.pending);
    game.set_transform([8., 4., 20.], &parent);
    assert_eq!(game.cell, (8, 11));
    assert!(game.pending);

    assert_eq!(parent.world_position([0.; 3]), parent.translation);
    let identity = ParentTransform::default();
    assert_eq!(identity.world_position([2., 3., 4.]), [2., 3., 4.]);
    game.set_transform([-0.25, 0., -16.25], &identity);
    assert_eq!(game.cell, (-1, -2));
}

#[test]
fn game_map_order_move_expiry_and_host_identity() {
    let record = GameRecord {
        flags: game_flag::PAUSE_SIM | game_flag::PAUSE_SIM_HIDDEN | game_flag::PAUSE_CLOCK,
        burn_radius: 1,
        toxic: 2,
        demolish_radius: 1,
        extinguish: 3,
        message_type: 7,
        map_kind: 4,
        duration: 2.,
        ..GameRecord::default()
    };
    let mut game = GameEffect::new(&record);
    let mut host = Host::new();
    game.set_parameter(13, 0x1234_5678_abcd_ef01);
    game.set_parameter(12, 9);
    assert_eq!(game.data, 0xabcd_ef01);
    game.set_position(40., 56.);
    game.start(&mut host);
    game.start(&mut host);
    assert_eq!(host.events, [Ev::Pause, Ev::HiddenPause, Ev::Clock(true)]);
    game.update(1., &mut host);
    assert_eq!(
        &host.events[3..],
        &[
            Ev::Ignite(2, 3, 0, 1),
            Ev::Cause(4),
            Ev::Toxic(2, 3),
            Ev::Demolish(
                EffectRect {
                    x0: 1,
                    z0: 2,
                    x1: 3,
                    z1: 4
                },
                1,
                false
            ),
            Ev::Extinguish(2, 3, 3),
            Ev::Post(EffectPost {
                msg_type: 7,
                data1: 0xabcd_ef01,
                data2: 2,
                data3: 3
            }),
            Ev::Map(4, (2, 3), 109),
        ]
    );
    let first = game.map_token;
    game.set_position(56., 56.);
    game.update(1., &mut host);
    assert!(game.is_active()); // equality at duration does not stop
    assert_eq!(game.count, 2);
    let second = game.map_token;
    assert_ne!(first, second);
    assert_eq!(
        &host.events[host.events.len() - 2..],
        &[Ev::RemoveMap(first), Ev::Map(4, (3, 3), second)]
    );
    game.update(0.25, &mut host);
    assert!(!game.is_active());
    assert_eq!(
        &host.events[host.events.len() - 4..],
        &[Ev::Resume, Ev::HiddenResume, Ev::Clock(false), Ev::RemoveMap(second)]
    );
    let events = host.events.clone();
    game.stop(&mut host);
    assert_eq!(events, host.events);
}

#[test]
fn game_single_does_not_remove_map_or_resume_and_absent_simulator_does_not_start() {
    let record = GameRecord {
        flags: game_flag::SINGLE | game_flag::PAUSE_SIM,
        map_kind: 1,
        ..GameRecord::default()
    };
    let mut host = Host::new();
    let mut game = GameEffect::new(&record);
    host.dilation = None;
    game.start(&mut host);
    assert!(!game.started);
    assert!(host.events.is_empty());
    host.dilation = Some(1.);
    game.start(&mut host);
    game.update(9., &mut host);
    game.stop(&mut host);
    assert_eq!(host.events, [Ev::Pause, Ev::Map(1, (-1, -1), 101)]);
    assert_eq!(
        (game.timer, game.count, game.pending, game.map_applied),
        (0., 0, true, true)
    );
}

#[test]
fn brush_draws_twice_even_for_constant_ranges_and_start_resets_stroke() {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 2,
    };
    let mut actual = crate::rng::RzRandom::new(123, &seeds);
    let mut expected = actual.clone();
    let record = BRUSHES[0];
    expected.range_f64(f64::from(record.strength.0), f64::from(record.strength.1));
    expected.range_f64(f64::from(record.radius.0), f64::from(record.radius.1));
    let mut brush = BrushEffect::new(&record, &mut actual);
    assert_eq!(
        (actual.lcg_state(), actual.mul_state()),
        (expected.lcg_state(), expected.mul_state())
    );
    brush.set_position(20., 40.);
    brush.start();
    brush.update(0.25, &mut Host::new());
    brush.start();
    assert_eq!(
        (brush.prev, brush.acc, brush.t, brush.active, brush.long_op),
        ((20., 40.), 0., 0., true, false)
    );
}
