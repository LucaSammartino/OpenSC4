// SPDX-License-Identifier: GPL-3.0-or-later
//! Command vectors WC8-06–WC8-10, WC8-12, WC8-13, WC9-01 and WC6-01–WC6-04
//! (§§6.2.1–6.2.3).
use super::tests::boat;
use super::*;
use std::sync::Arc;

#[derive(Default)]
struct Effects {
    fire_succeeds: bool,
    calls: Vec<String>,
}
impl PlayerEffects for Effects {
    fn play_fire(&mut self, name: &'static str) -> bool {
        self.calls.push(name.into());
        self.fire_succeeds
    }
    fn fire_model_hook(&mut self) {}
    fn has_waterski(&self) -> bool {
        true
    }
    fn waterski_transform(&mut self, transform: &Transform) {
        self.calls.push(format!("transform {:?}", transform.translation));
    }
    fn waterski(&mut self, running: bool) {
        self.calls.push(format!("skier:{running}"));
    }
    fn command_event(&mut self, _: usize, _: u32) {}
}

fn steer(left: f32, right: f32, steering: f32, dt: f32) -> (f32, DriveCommand) {
    let mut b = boat();
    let mut command = DriveCommand {
        throttle: -1.0,
        reverse: -1.0,
        fire: -1.0,
        left,
        right,
        steering,
        ..DriveCommand::default()
    };
    assert!(b
        .translate_drive_commands(dt, &mut command, &mut Effects::default())
        .is_some());
    (b.turn, command)
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn wc8_06_wc8_07_single_channels_respond_in_opposite_directions() {
    near(steer(100.0, -1.0, 0.0, 10.0).0, -0.1);
    near(steer(-1.0, 100.0, 0.0, 10.0).0, 0.1);
}

#[test]
fn wc9_01_wc8_08_simultaneous_channels_follow_response_order() {
    let (turn, command) = steer(100.0, 100.0, 0.0, 10.0);
    near(turn, -0.1);
    assert_eq!((command.left, command.right), (100.0, 100.0));
}

#[test]
fn wc8_09_inactive_channels_return_toward_zero() {
    near(steer(-1.0, -1.0, -0.2, 10.0).0, -0.19);
}

#[test]
fn wc8_10_zero_channel_rebases_timer_with_opposite_signs() {
    let (turn, command) = steer(0.0, -1.0, -0.2, 10.0);
    near(command.left, 190.0);
    near(turn, -0.19);
    let (turn, command) = steer(-1.0, 0.0, 0.2, 10.0);
    near(command.right, 210.0);
    near(turn, 0.21);
}

fn skier_group() -> Option<Arc<Group>> {
    Some(Arc::new(Group {
        id: 1,
        parameters: None,
        aems_value: Some(1.0),
        parent: Some(Arc::new(Group {
            id: 0x4409,
            parameters: None,
            aems_value: None,
            parent: None,
        })),
    }))
}

#[test]
fn wc8_12_running_skier_stops_at_zero_speed_without_toggle_write() {
    let mut b = boat();
    b.group = skier_group();
    b.player.skier_running = true;
    let mut command = DriveCommand::default();
    let mut effects = Effects::default();
    b.translate_drive_commands(0.0, &mut command, &mut effects);
    assert_eq!(effects.calls, ["skier:false"]);
    assert!(!b.player.skier_running);
    assert!(!command.skier_toggle);
}

#[test]
fn wc8_13_missing_group_or_aems_returns_false_without_command_effects() {
    for group in [
        None,
        Some(Arc::new(Group {
            id: 1,
            parameters: None,
            aems_value: None,
            parent: Some(Arc::new(Group {
                id: 2,
                parameters: None,
                aems_value: None,
                parent: None,
            })),
        })),
    ] {
        let mut b = boat();
        b.group = group;
        b.speed = 0.2;
        b.turn = 0.05;
        let mut command = DriveCommand {
            throttle: 3000.0,
            left: 100.0,
            steering: 0.05,
            ..DriveCommand::default()
        };
        let before = command.clone();
        let mut effects = Effects::default();
        assert_eq!(b.translate_drive_commands(10.0, &mut command, &mut effects), None);
        assert_eq!((b.speed, b.turn), (0.2, 0.05));
        assert_eq!(command, before);
        assert!(effects.calls.is_empty());
    }
}

#[test]
fn wc6_01_wc6_02_fire_requires_armed_channel_and_successful_start() {
    for succeeds in [true, false] {
        let mut b = boat();
        b.player.shots = 3;
        let mut command = DriveCommand {
            throttle: -1.0,
            reverse: -1.0,
            fire: 0.0,
            ..DriveCommand::default()
        };
        command.shared[7].armed = true;
        command.shared[7].timer = 9.0;
        let mut effects = Effects {
            fire_succeeds: succeeds,
            ..Effects::default()
        };
        b.translate_drive_commands(10.0, &mut command, &mut effects);
        assert_eq!(effects.calls, ["whalefire"]);
        if succeeds {
            assert_eq!(b.player.shots, 4);
            assert_eq!(command.shared[7].timer, 0.0);
            assert!(!command.shared[7].armed);
        } else {
            assert_eq!(b.player.shots, 3);
            assert_eq!(command.shared[7].timer, 9.0);
            assert!(command.shared[7].armed);
        }
    }
    let mut b = boat();
    let mut command = DriveCommand {
        fire: -1.0,
        ..DriveCommand::default()
    };
    command.shared[7].armed = true;
    let mut effects = Effects {
        fire_succeeds: true,
        ..Effects::default()
    };
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!(effects.calls.is_empty());
}

#[test]
fn wc6_03_skier_start_uses_boat_transform_then_refreshes_while_running() {
    let mut b = boat();
    b.group = skier_group();
    b.speed = 0.1;
    b.transform.translation = [1.0, 2.0, 3.0];
    let mut command = DriveCommand {
        throttle: -1.0,
        reverse: -1.0,
        fire: -1.0,
        skier_toggle: true,
        ..DriveCommand::default()
    };
    let mut effects = Effects::default();
    b.translate_drive_commands(0.0, &mut command, &mut effects);
    let m = "transform [1.0, 2.0, 3.0]".to_string();
    assert_eq!(effects.calls, [m.clone(), "skier:true".into(), m]);
    assert!(b.player.skier_running);
}

#[test]
fn wc6_04_tug_button_arms_without_rope_effect() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.group = Some(Arc::new(Group {
        id: 0x4410,
        parameters: None,
        aems_value: Some(1.0),
        parent: None,
    }));
    f.insert(1, b).unwrap();
    let mut command = DriveCommand {
        throttle: -1.0,
        reverse: -1.0,
        fire: -1.0,
        tow: true,
        ..DriveCommand::default()
    };
    let mut effects = Effects::default();
    assert_eq!(f.player_commands(1, 10.0, &mut command, &mut effects, &mut None), Ok(()));
    let b = f.get(1).unwrap();
    assert!(b.tow_armed);
    assert!(b.events.contains(&Event::PlayEffect("tugropethrow")));
}
