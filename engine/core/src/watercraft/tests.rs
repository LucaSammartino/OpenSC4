// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent vectors from watercraft §§2-7,9,11, not captured native output.
#![allow(clippy::approx_constant)] // The specification publishes rounded decimal constants.
use super::*;
use crate::automata::{AutomatonBase, State};
use crate::stream::{ByteReader, ByteWriter};
use std::sync::Arc;

#[test]
fn wc9_literal_simultaneous_steering_pending_spec_resolution() {
    let mut b = boat();
    let mut command = DriveCommand { left: 100.0, right: 100.0, ..DriveCommand::default() };
    b.translate_drive_commands(10.0, &mut command, &mut CommandEffects::default());
    assert!((b.turn + 0.1).abs() < 1e-6);
    assert_eq!(command.left, 100.0);
}

#[test]
fn wc6_05_through_09_render_tween_and_shared_channel_boundaries() {
    let mut b = boat();
    let mut command = DriveCommand::default();
    let mut effects = CommandEffects::default();
    b.turn = 3.0;
    b.heading = [1.0, 0.0, 0.0];
    b.complete_drive_commands(0.0, &mut command, &mut effects);
    assert_eq!(b.turn, 2.0);
    assert_eq!(command.rudder, [-0.25, 0.0, std::f32::consts::TAU]);
    assert!((command.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    b.heading = [-1.0, 0.0, 0.0];
    b.complete_drive_commands(0.0, &mut command, &mut effects);
    assert!((command.heading - 3.0 * std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    b.heading = [0.0, 0.0, 1.0];
    b.complete_drive_commands(0.0, &mut command, &mut effects);
    assert_eq!(command.heading, 0.0);
    // Start chosen so the existing linear tween samples 0.15 at 5/2000.
    b.player.speed_tween = SpeedTween { start: 0.150_125_31, target: 0.1, elapsed: 5.0, duration: 2000.0 };
    b.speed = -0.2;
    b.complete_drive_commands(0.0, &mut command, &mut effects);
    let tween = b.player.speed_tween;
    assert!((tween.start - 0.15).abs() < 1e-6);
    assert_eq!((tween.target, tween.elapsed, tween.duration), (0.2, 0.0, 0.0));
    b.complete_drive_commands(0.0, &mut command, &mut effects);
    assert_eq!(b.player.speed_tween, tween);
    command.throttle = 0.0;
    command.shared[0] = CommandChannel { previous: 10.0, duration: 2, limit: 1, timer: 95.0, interval: 100.0, ..CommandChannel::default() };
    command.shared_bookkeeping(10.0, &mut effects);
    assert!(command.shared[0].edge);
    assert!(!command.shared[0].armed);
    assert_eq!(command.shared[0].timer, 105.0);
    command.shared_bookkeeping(10.0, &mut effects);
    assert!(!command.shared[0].edge);
    assert!(command.shared[0].armed);
    assert_eq!(command.shared[0].timer, 0.0);
    command.shared[0].previous = 250.0;
    command.shared_bookkeeping(10.0, &mut effects);
    assert!(!command.shared[0].edge);
}

#[test]
fn wc6_09_shared_events_exact_sentinel_and_signed_duration() {
    #[derive(Default)]
    struct Events(Vec<(usize, u32)>);
    impl PlayerEffects for Events {
        fn play_fire(&mut self, _: &'static str) -> bool { false }
        fn fire_model_hook(&mut self) {}
        fn has_waterski(&self) -> bool { false }
        fn waterski_transform(&mut self, _: &Transform) {}
        fn waterski(&mut self, _: bool) {}
        fn command_event(&mut self, channel: usize, event: u32) { self.0.push((channel, event)); }
    }
    let mut command = DriveCommand { throttle: 0.0, reverse: -2.0, ..DriveCommand::default() };
    command.shared[0] = CommandChannel { duration: 2, limit: 2, armed: true, ..CommandChannel::default() };
    let mut effects = Events::default();
    command.shared_bookkeeping(10.0, &mut effects);
    assert!(!command.shared[0].armed);
    assert_eq!(effects.0, [(0, 0x14), (1, 0x14)]);
    command.shared[0].duration = -1;
    effects.0.clear();
    command.shared_bookkeeping(10.0, &mut effects);
    assert!(command.shared[0].armed);
    assert_eq!(effects.0, [(1, 0x14)]);
}

fn group() -> Arc<Group> {
    Arc::new(Group {
        id: 0,
        parameters: None,
        aems_value: Some(1.0),
        parent: None,
    })
}

#[derive(Default)]
struct CommandEffects {
    fire_succeeds: bool,
    calls: Vec<String>,
}

impl PlayerEffects for CommandEffects {
    fn fire_model_hook(&mut self) {}
    fn has_waterski(&self) -> bool { true }
    fn waterski_transform(&mut self, _: &Transform) { self.calls.push("transform".into()); }
    fn command_event(&mut self, _: usize, _: u32) {}
    fn play_fire(&mut self, name: &'static str) -> bool {
        self.calls.push(name.into());
        self.fire_succeeds
    }

    fn waterski(&mut self, running: bool) {
        self.calls.push(format!("skier:{running}"));
    }
}

#[test]
fn wc8_01_through_05_nonlinear_speed_channels_and_crossing_zero() {
    let mut b = boat();
    let mut effects = CommandEffects::default();
    let mut command = DriveCommand {
        throttle: 3000.0,
        ..DriveCommand::default()
    };
    assert_eq!(
        b.translate_drive_commands(1000.0, &mut command, &mut effects),
        Some(Gear::Forward)
    );
    assert_eq!(b.speed, 0.3);
    assert_eq!(b.player.gear, Gear::Forward);
    command.throttle = 1.0;
    b.translate_drive_commands(100.0, &mut command, &mut effects);
    assert!((b.speed - 0.3).abs() < 1e-6);
    assert!((command.throttle - 3000.0).abs() < 0.01);
    command.throttle = -1.0;
    command.reverse = 2000.0;
    b.speed = 0.005;
    b.translate_drive_commands(100.0, &mut command, &mut effects);
    assert!((b.speed + 0.005).abs() < 1e-6);
    assert_eq!(command.reverse, 150.0);
    assert_eq!(b.player.gear, Gear::Reverse);
    command.reverse = -1.0;
    command.throttle = 3000.0;
    b.translate_drive_commands(100.0, &mut command, &mut effects);
    assert!((b.speed - 0.005).abs() < 1e-6);
    assert_eq!(command.throttle, 0.0);
    command.throttle = -1.0;
    b.speed = 0.1;
    b.translate_drive_commands(10_000.0, &mut command, &mut effects);
    assert_eq!(b.speed, 0.1);
    assert_eq!(b.player.gear, Gear::Neutral);
    command.throttle = 3000.0;
    command.reverse = 150.0;
    b.speed = 0.0;
    b.translate_drive_commands(100.0, &mut command, &mut effects);
    assert_eq!(b.player.gear, Gear::Forward);
    assert!((b.speed - 0.1).abs() < 1e-6);
}

#[test]
fn tow_ahead_cancels_only_for_positive_heading_dot_product() {
    let mut f = fleet();
    f.insert(2, boat()).unwrap();
    f.push_state(2, State::new(state::TOWED), &mut None).unwrap();
    f.get_mut(1).unwrap().towed = Some(2);
    f.get_mut(2).unwrap().tug = Some(1);
    f.get_mut(2).unwrap().position = [0.0, 0.0, 10.0];
    f.release_tow_ahead(1, &mut None).unwrap();
    assert_eq!(f.get(1).unwrap().towed, Some(2)); // zero dot is not ahead
    f.get_mut(2).unwrap().position[0] = 1.0;
    f.release_tow_ahead(1, &mut None).unwrap();
    assert_eq!(f.get(1).unwrap().towed, None);
    assert_eq!(f.get(2).unwrap().tug, None);
    assert_eq!(f.get(2).unwrap().top_state(), None);
}

#[test]
fn following_target_is_radius_plus_ten_behind_tug() {
    let b = boat();
    assert_eq!(b.following_target([20.0, 3.0, 5.0], [1.0, 0.0, 0.0]), [5.0, 3.0, 5.0]);
}

#[test]
fn backwards_and_still_timers_use_strict_boundaries_and_stack_membership() {
    let mut b = boat();
    b.speed = -0.1;
    b.update_backwards_time(100.0);
    b.update_backwards_time(200.0);
    assert_eq!(b.backwards_elapsed, 300.0);
    b.speed = 0.0;
    b.update_backwards_time(100.0);
    assert_eq!(b.backwards_elapsed, 0.0);
    b.still_elapsed = 5000.0;
    assert!(b.still_time_allows_animation());
    b.still_elapsed = 5001.0;
    assert!(!b.still_time_allows_animation());
    for kind in [
        state::IDLE,
        state::PLAYER_DRIVE,
        state::QUEUE,
        state::PARKED,
        state::TOWED,
        state::UNDOCK,
    ] {
        b.record.base.states = vec![State::new(kind), State::new(state::DOCK)];
        assert!(b.still_time_allows_animation());
    }
    b.record.base.states = vec![State::new(state::DOCK)];
    assert!(!b.still_time_allows_animation());
}

#[test]
fn parent_sound_set_enables_commands_and_tow_button_runs_before_base_translation() {
    let mut f = fleet();
    f.get_mut(1).unwrap().group = Arc::new(Group {
        id: 0x4410,
        parameters: None,
        aems_value: None,
        parent: Some(group()),
    });
    let mut effects = CommandEffects::default();
    let mut command = DriveCommand {
        tow: true,
        ..DriveCommand::default()
    };
    f.player_commands(1, 0.0, &mut command, &mut effects, &mut None)
        .unwrap();
    assert!(f.get(1).unwrap().tow_armed);
    assert_eq!(
        f.get(1).unwrap().events,
        [Event::PlayEffect("tugropethrow"), Event::BaseCommandTranslation]
    );
}

#[test]
fn command_steering_countersteers_returns_and_clamps() {
    let mut b = boat();
    let mut effects = CommandEffects::default();
    let mut command = DriveCommand {
        right: 100.0,
        ..DriveCommand::default()
    };
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!((b.turn - 0.1).abs() < 1e-6);
    command.right = -1.0;
    command.left = 100.0;
    b.turn = 0.0;
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!((b.turn + 0.1).abs() < 1e-6);
    command.left = -1.0;
    b.turn = -0.2;
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!((b.turn + 0.19).abs() < 1e-6);
    command.left = 0.0;
    b.turn = -0.2;
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!((command.left - 190.0).abs() < 1e-3);
    assert!((b.turn + 0.19).abs() < 1e-6);
    command.left = -1.0;
    command.right = 0.0;
    b.turn = 0.2;
    b.translate_drive_commands(10.0, &mut command, &mut effects);
    assert!((command.right - 210.0).abs() < 1e-3);
    assert!((b.turn - 0.21).abs() < 1e-6);
}

#[test]
fn commands_require_aems_and_fire_consumes_only_successful_shots() {
    let mut b = boat();
    let mut effects = CommandEffects::default();
    let mut command = DriveCommand {
        throttle: 1.0,
        fire: 0.0,
        ..DriveCommand::default()
    };
    command.shared[7].armed = true;
    command.shared[7].timer = 42.0;
    b.group = Arc::new(Group {
        aems_value: None,
        ..(*group()).clone()
    });
    assert_eq!(b.translate_drive_commands(100.0, &mut command, &mut effects), None);
    assert_eq!(b.speed, 0.0);
    assert_eq!(command.fire, 0.0);
    assert!(effects.calls.is_empty());
    b.group = group();
    b.translate_drive_commands(0.0, &mut command, &mut effects);
    assert!(command.shared[7].armed);
    assert_eq!(command.shared[7].timer, 42.0);
    assert_eq!(b.player.shots, 0);
    effects.fire_succeeds = true;
    b.translate_drive_commands(0.0, &mut command, &mut effects);
    assert!(!command.shared[7].armed);
    assert_eq!(command.shared[7].timer, 0.0);
    assert_eq!(command.fire, 0.0);
    assert_eq!(b.player.shots, 1);
    assert_eq!(effects.calls, ["whalefire", "whalefire"]);
}

#[test]
fn skier_toggle_and_stop_and_command_translation_order() {
    let mut f = fleet();
    f.get_mut(1).unwrap().group = Arc::new(Group {
        id: 0x4409,
        ..(*group()).clone()
    });
    let mut effects = CommandEffects::default();
    let mut command = DriveCommand {
        throttle: 3000.0,
        skier_toggle: true,
        ..DriveCommand::default()
    };
    f.player_commands(1, 100.0, &mut command, &mut effects, &mut None)
        .unwrap();
    assert_eq!(effects.calls, ["transform", "skier:true", "transform"]);
    assert_eq!(
        f.get(1).unwrap().events,
        [Event::ResetSpeedTween, Event::BaseCommandTranslation]
    );
    command.skier_toggle = false;
    command.throttle = -1.0;
    f.get_mut(1).unwrap().speed = 0.0;
    f.player_commands(1, 10_000.0, &mut command, &mut effects, &mut None)
        .unwrap();
    assert_eq!(effects.calls, ["transform", "skier:true", "transform", "skier:false"]);
    assert!(!f.get(1).unwrap().player.skier_running);
    f.get_mut(1).unwrap().events.clear();
    f.player_commands(1, 100.0, &mut command, &mut effects, &mut None)
        .unwrap();
    assert_eq!(f.get(1).unwrap().events, [Event::BaseCommandTranslation]);
}

#[test]
fn geometry_dispatch_pops_completed_docking_and_undocking() {
    let mut f = fleet();
    f.get_mut(1)
        .unwrap()
        .set_station_information([station(1, 0.0), station(2, 10.0)], true);
    f.get_mut(1).unwrap().position = [9.5, 0.0, 0.0];
    f.get_mut(1).unwrap().inherited_position = [9.5, 0.0, 0.0];
    f.get_mut(1).unwrap().dock_requests = [true; 2];
    f.push_state(1, State::new(state::DOCK), &mut None).unwrap();
    let rectangle = [Cell { x: 9, z: 0 }; 4];
    f.state_tick_with_geometry(1, 0.0, rectangle, &Water, &mut None)
        .unwrap();
    assert_eq!(f.get(1).unwrap().top_state(), None);
    assert!(!f.get(1).unwrap().docking);
    f.get_mut(1).unwrap().neighbor = Some(NeighborRoute {
        occupant: Arc::new(Resource(4)),
        position: [0.0; 3],
        cells: Arc::from([]),
        next: 0,
    });
    f.get_mut(1).unwrap().position = [10.0, 0.0, 0.0];
    f.push_state(1, State::new(state::UNDOCK), &mut None).unwrap();
    f.state_tick_with_geometry(1, 1.0, rectangle, &Water, &mut None)
        .unwrap();
    assert_eq!(f.get(1).unwrap().top_state(), None);
}

pub(super) fn boat() -> Watercraft {
    let mut boat = Watercraft::new(
        WatercraftRecord {
            base: AutomatonBase::new(),
        },
        group(),
        5.0,
    );
    boat.heading = [1.0, 0.0, 0.0];
    boat.previous_heading = boat.heading;
    boat.inherited_heading = boat.heading;
    boat.travel.clear(); // Tests below exercise initialized runtime storage.
    boat
}

fn fleet() -> Fleet {
    let mut fleet = Fleet::default();
    fleet.insert(1, boat()).unwrap();
    fleet
}

fn station(id: u64, x: f32) -> Arc<Station> {
    Arc::new(Station {
        lot: Arc::new(Resource(id)),
        occupant: Arc::new(Resource(id + 10)),
        position: [x, 0.0, 0.0],
        direction: [1.0, 0.0, 0.0],
        route: Arc::from([]),
    })
}

#[derive(Default)]
struct Manager {
    berth: bool,
    waiting: bool,
    stay: bool,
    calls: Vec<String>,
}

impl DockManager for Manager {
    fn take_berth(&mut self, lot: &Resource, boat: BoatId) -> bool {
        self.calls.push(format!("berth:{}:{boat}", lot.0));
        self.berth
    }
    fn take_waiting_spot(&mut self, lot: &Resource, boat: BoatId) -> bool {
        self.calls.push(format!("wait:{}:{boat}", lot.0));
        self.waiting
    }
    fn done_docking(&mut self, lot: &Resource, boat: BoatId) {
        self.calls.push(format!("done:{}:{boat}", lot.0));
    }
    fn may_stay(&mut self, boat: BoatId) -> bool {
        self.calls.push(format!("stay:{boat}"));
        self.stay
    }
}

pub(super) struct Water;
impl Terrain for Water {
    fn contains(&self, cell: Cell) -> bool {
        (0..100).contains(&cell.x) && (0..100).contains(&cell.z)
    }
    fn cell_at(&self, position: [f32; 3]) -> Cell {
        Cell {
            x: position[0] as i32,
            z: position[2] as i32,
        }
    }
    fn terrain_height(&self, _: [f32; 3]) -> f32 {
        -10.0
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: [f32; 3]) -> f32 {
        0.0
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}

#[test]
fn absent_list_and_zero_list_have_distinct_dependent_defaults() {
    let absent = DriveParameters::load(None);
    assert_eq!(absent.0[21], 8000.0);
    assert_eq!(absent.0[36], 300.0);
    assert_eq!(absent.0[17], 1.0472);
    let mut list = [0.0; 40];
    list[9] = 3.0;
    list[7] = 0.4;
    list[18] = 0.8;
    list[20] = 0.6;
    let loaded = DriveParameters::load(Some(&list));
    assert_eq!(loaded.0[8], -3.0);
    assert_eq!(loaded.0[17], 2.0);
    assert_eq!(loaded.0[19], 0.2);
    assert_eq!(loaded.0[33], 0.0);
}

#[test]
fn zero_sign_nan_and_supplied_dependent_entries() {
    let mut list = [0.0; 40];
    list[0] = -0.0;
    list[1] = f32::NAN;
    list[8] = -4.0;
    list[17] = 9.0;
    list[19] = -2.0;
    let loaded = DriveParameters::load(Some(&list));
    assert_eq!(loaded.0[0], 0.1);
    assert!(loaded.0[1].is_nan());
    assert_eq!(loaded.0[8], -4.0);
    assert_eq!(loaded.0[17], 9.0);
    assert_eq!(loaded.0[19], -2.0);
}

#[test]
fn first_present_ancestor_list_wins_even_when_all_zero() {
    let mut parent = (*group()).clone();
    let mut values = [0.0; 40];
    values[21] = 123.0;
    parent.parameters = Some(values);
    let mut child = (*group()).clone();
    child.parent = Some(Arc::new(parent));
    assert_eq!(child.drive_parameters().0[21], 123.0);
    child.parameters = Some([0.0; 40]);
    assert_eq!(child.drive_parameters().0[21], 8000.0);
}

#[test]
fn record_only_saves_version_and_base_and_rejects_truncation() {
    let mut original = boat();
    original.record.base.states.push(State::new(state::IDLE));
    original.lifetime_elapsed = 900.0;
    original.position = [10.0, 20.0, 30.0];
    let mut writer = ByteWriter::new();
    assert!(original.record.write(&mut writer));
    let bytes = writer.bytes();
    assert_eq!(&bytes[..4], &[2, 0, 8, 0]);
    let loaded = WatercraftRecord::read(&mut ByteReader::new(bytes)).unwrap();
    assert_eq!(loaded.base.states, original.record.base.states);
    assert_eq!(loaded.base.vectors, original.record.base.vectors);
    assert_eq!(loaded.base.automata_flags, original.record.base.automata_flags);
    // OccupantBase::read applies its inherited loaded-status bit. Runtime
    // watercraft fields are absent: changing them cannot change the record.
    original.lifetime_elapsed = 1.0;
    original.position = [0.0; 3];
    let mut again = ByteWriter::new();
    assert!(original.record.write(&mut again));
    assert_eq!(bytes, again.bytes());
    for end in 0..bytes.len() {
        assert!(
            WatercraftRecord::read(&mut ByteReader::new(&bytes[..end])).is_none(),
            "truncation {end}"
        );
    }
    let mut bad = bytes.to_vec();
    bad[0] = 3;
    assert!(WatercraftRecord::read(&mut ByteReader::new(&bad)).is_none());
}

#[test]
fn state_entry_exit_and_remove_side_effect_order() {
    let mut fleet = fleet();
    let mut no_manager = None;
    fleet
        .push_state(1, State::new(state::PLAYER_DRIVE), &mut no_manager)
        .unwrap();
    let b = &fleet.boats[&1];
    assert_eq!(b.player.field_144, 15);
    assert_eq!(b.player.field_1ac, 2000.0);
    assert_eq!(b.player.turn_rate, 0.2618);
    assert_eq!(b.events, [Event::ActiveSound(false)]);
    fleet.pop_state(1, &mut no_manager).unwrap();
    fleet
        .push_state(1, State::new(state::SLOW_REMOVE), &mut no_manager)
        .unwrap();
    fleet.state_tick(1, 1.0, &mut no_manager).unwrap();
    assert_eq!(
        fleet.boats[&1].events,
        [
            Event::ActiveSound(false),
            Event::ActiveSound(true),
            Event::Remove(false),
            Event::Remove(false)
        ]
    );
}

#[test]
fn idle_is_cumulative_and_strictly_above_sixty_seconds_falls_through() {
    let mut fleet = fleet();
    let mut manager = None;
    fleet.push_state(1, State::new(state::IDLE), &mut manager).unwrap();
    fleet.state_tick(1, 60_000.0, &mut manager).unwrap();
    assert_eq!(fleet.boats[&1].top_state(), Some(state::IDLE));
    fleet.pop_state(1, &mut manager).unwrap();
    fleet.push_state(1, State::new(state::IDLE), &mut manager).unwrap();
    assert_eq!(
        fleet.state_tick(1, 1.0, &mut manager),
        Err(ContractGap::CrashSequenceWc5)
    );
    assert_eq!(fleet.boats[&1].idle_elapsed, 60_001.0);
    assert_eq!(fleet.boats[&1].crash_elapsed, 1.0);
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SLOW_REMOVE));
}

#[test]
fn ferry_public_vector_selects_berth_wait_or_remove_and_preserves_order() {
    for (berth, waiting, expected, calls) in [
        (true, true, state::DOCK, vec!["berth:2:1"]),
        (false, true, state::QUEUE, vec!["berth:2:1", "wait:2:1"]),
        // Entering removal also gives back the source berth (§4 EndDocking).
        (
            false,
            false,
            state::SLOW_REMOVE,
            vec!["berth:2:1", "wait:2:1", "done:1:1"],
        ),
    ] {
        let mut fleet = fleet();
        fleet
            .boats
            .get_mut(&1)
            .unwrap()
            .set_station_information([station(1, 0.0), station(2, 250.0)], true);
        let mut manager = Manager {
            berth,
            waiting,
            ..Manager::default()
        };
        fleet.dock_requests(1, &mut Some(&mut manager)).unwrap();
        assert_eq!(fleet.boats[&1].top_state(), Some(expected));
        assert_eq!(manager.calls, calls);
    }
}

#[test]
fn queued_occupied_berth_never_takes_another_waiting_spot() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.set_station_information([station(1, 0.0), station(2, 250.0)], true);
    let mut manager = Manager::default();
    fleet.push_state(1, State::new(state::QUEUE), &mut None).unwrap();
    fleet.dock_requests(1, &mut Some(&mut manager)).unwrap();
    assert_eq!(manager.calls, ["berth:2:1"]);
    assert_eq!(fleet.boats[&1].top_state(), Some(state::QUEUE));
}

#[test]
fn public_docked_vector_and_far_approach_gap() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.set_station_information([station(1, 0.0), station(2, 10.0)], true);
    b.position = [9.5, 0.0, 0.0];
    let angle = 0.5_f32.to_radians();
    b.heading = [angle.cos(), 0.0, angle.sin()];
    b.inherited_position = b.position;
    b.inherited_heading = b.heading;
    assert_eq!(fleet.docking_tick(1, 1.0, &Water, &mut None), Ok(false));
    fleet.boats.get_mut(&1).unwrap().position = [200.0, 0.0, 0.0];
    fleet.boats.get_mut(&1).unwrap().inherited_position = [200.0, 0.0, 0.0];
    assert_eq!(
        fleet.docking_tick(1, 1.0, &Water, &mut None),
        Err(ContractGap::AiMotionWc4)
    );
}

#[test]
fn player_collision_public_ratios_are_strict() {
    let mut b = boat();
    b.record.base.states.push(State::new(state::PLAYER_DRIVE));
    b.speed = 0.12;
    assert!(!b.boat_crash_threshold_exceeded(0.0));
    b.speed = 0.18;
    assert!(b.boat_crash_threshold_exceeded(0.0));
    b.speed = b.drive.0[7] * b.drive.0[27];
    assert!(!b.boat_crash_threshold_exceeded(0.0));
    b.speed = -0.18;
    assert!(b.boat_crash_threshold_exceeded(0.0));
}

#[test]
fn player_coasting_cannot_cross_zero_and_uses_unclamped_dt() {
    let mut b = boat();
    b.speed = 0.01;
    b.coast(10_000.0, -1.0, -1.0);
    assert_eq!(b.speed, 0.0);
    b.speed = -0.01;
    b.coast(10_000.0, -1.0, -1.0);
    assert_eq!(b.speed, 0.0);
    b.speed = 0.3;
    b.coast(100.0, 0.0, -1.0);
    assert_eq!(b.speed, 0.3);
    assert_eq!(b.clamped_frame_time(-10.0), 0.0);
    assert_eq!(b.clamped_frame_time(1000.0), 100.0);
}

#[test]
fn exact_gear_gates_and_rudder() {
    let mut b = boat();
    assert_eq!(b.select_gear(0.0, 150.0), Some((Gear::Forward, 0.0)));
    assert_eq!(b.select_gear(-1.0, 150.0), Some((Gear::Reverse, -150.0)));
    assert_eq!(b.select_gear(-1.0, 149.0), Some((Gear::Neutral, 0.0)));
    b.group = Arc::new(Group {
        aems_value: None,
        ..(*group()).clone()
    });
    assert_eq!(b.select_gear(1.0, 500.0), None);
    b.turn = 2.0;
    assert_eq!(b.rudder_angle(), -0.25);
}

#[test]
fn wakes_wait_until_strictly_above_250ms_then_restart_in_order() {
    let mut b = boat();
    b.update_wakes(250.0, 16.0);
    assert!(b.events.is_empty());
    b.update_wakes(1.0, 16.0);
    assert_eq!(b.events, WAKES.map(Event::StopWake));
    b.events.clear();
    b.speed = -0.001;
    b.update_wakes(1.0, 16.0);
    assert_eq!(b.still_elapsed, 0.0);
    assert_eq!(b.events, WAKES.map(Event::StartWake));
}

#[test]
fn debug_text_has_exact_fields_width_and_negative_zero() {
    let mut b = boat();
    b.turn = -0.0;
    b.queued = true;
    assert_eq!(b.debug_extra_text([2, 3], Cell { x: 1, z: -2 }),
        "Tow: 0, (FT,MT): (2,3), (Q,D): (1,0), Turn: -0.00, Speed: 0.00, Cell(  1, -2), Pos(0.00,0.00,0.00), Ornt<1.00,0.00,0.00>");
}

#[test]
fn direction_order_excludes_reverse_and_respects_every_mask_bit() {
    assert_eq!(candidate_directions(2, 0).collect::<Vec<_>>(), [2, 3, 1, 4, 0, 5, -1]);
    for direction in 0..8 {
        let order = candidate_directions(direction, 0).collect::<Vec<_>>();
        assert_eq!(order.len(), 7);
        assert!(!order.contains(&i32::from((direction + 4) % 8)));
        for mask in 0..=255 {
            let expected = order
                .iter()
                .copied()
                .filter(|d| mask & (1_u32.wrapping_shl((*d as u32) & 31) as u8) == 0)
                .collect::<Vec<_>>();
            assert_eq!(candidate_directions(direction, mask).collect::<Vec<_>>(), expected);
        }
    }
}

#[test]
fn lifetime_boundary_manager_order_and_retention_flags() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.lifetime_limit = 100.0;
    b.lifetime_elapsed = 100.0;
    b.record.base.automata_flags = 0x10000 | 0x400;
    let mut manager = Manager {
        stay: true,
        ..Manager::default()
    };
    assert_eq!(fleet.lifetime_tick(1, 1.0, &mut Some(&mut manager)), Ok(true));
    assert!(manager.calls.is_empty());
    assert_eq!(fleet.lifetime_tick(1, 1.0, &mut Some(&mut manager)), Ok(true));
    assert_eq!(manager.calls, ["stay:1"]);
    assert_eq!(fleet.boats[&1].lifetime_limit, 0.0);
    assert_eq!(fleet.boats[&1].record.base.automata_flags, 0x400);
    assert_eq!(fleet.boats[&1].lifetime_elapsed, 102.0);
}

#[test]
fn parked_lifetime_asks_manager_before_override_and_no_manager_removes() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.lifetime_limit = 10.0;
    b.lifetime_elapsed = 11.0;
    let mut manager = Manager {
        stay: true,
        ..Manager::default()
    };
    fleet
        .push_state(1, State::new(state::PARKED), &mut Some(&mut manager))
        .unwrap();
    assert_eq!(fleet.lifetime_tick(1, 100.0, &mut Some(&mut manager)), Ok(false));
    assert_eq!(manager.calls, ["stay:1"]);
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SLOW_REMOVE));
    assert_eq!(fleet.boats[&1].lifetime_elapsed, 11.0);
    fleet.pop_state(1, &mut None).unwrap();
    fleet.pop_state(1, &mut None).unwrap();
    fleet.lifetime_tick(1, 1.0, &mut None).unwrap();
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SLOW_REMOVE));
}

#[test]
fn animation_early_return_and_two_idle_accumulations() {
    let mut fleet = fleet();
    fleet.boats.get_mut(&1).unwrap().count_idle_always = true;
    fleet.push_state(1, State::new(state::IDLE), &mut None).unwrap();
    assert_eq!(fleet.animation_budget(1, 0.000009, 20.0, &mut None), Ok(false));
    assert_eq!(fleet.boats[&1].idle_elapsed, 0.0);
    assert_eq!(fleet.animation_budget(1, 0.00001, 20.0, &mut None), Ok(true));
    fleet.state_tick(1, 30.0, &mut None).unwrap();
    assert_eq!(fleet.boats[&1].idle_elapsed, 50.0);
    assert_eq!(fleet.lifetime_tick(1, 1000.0, &mut None), Ok(false));
    assert_eq!(fleet.boats[&1].lifetime_elapsed, 0.0);
}

#[test]
fn lifetime_stack_exclusions_are_not_just_top_state_checks() {
    for excluded in [16, 74, 75, 77, 78] {
        let mut fleet = fleet();
        let b = fleet.boats.get_mut(&1).unwrap();
        b.record.base.states = vec![State::new(excluded), State::new(3)];
        b.lifetime_limit = 1.0;
        b.lifetime_elapsed = 2.0;
        assert_eq!(fleet.lifetime_tick(1, 100.0, &mut None), Ok(false));
        assert_eq!(fleet.boats[&1].lifetime_elapsed, 2.0);
        assert_eq!(fleet.boats[&1].top_state(), Some(3));
    }
}

#[test]
fn tow_message_uses_first_geometric_match_then_group_and_releases_both_links() {
    let mut fleet = fleet();
    let mut other = boat();
    other.group = Arc::new(Group {
        id: 0x4417,
        ..(*group()).clone()
    });
    other.position = [100.0, 0.0, 0.0];
    fleet.insert(2, other).unwrap();
    fleet.boats.get_mut(&1).unwrap().tow_armed = true;
    let mut manager = Manager::default();
    assert_eq!(
        fleet.do_message(1, Message::TowLanding(Some([100.0, 0.0, 0.0])), &mut Some(&mut manager)),
        Ok(MessageDisposition::Handled)
    );
    assert_eq!(fleet.boats[&1].towed, Some(2));
    assert_eq!(fleet.boats[&2].tug, Some(1));
    assert_eq!(fleet.boats[&2].top_state(), Some(state::TOWED));
    assert!(!fleet.boats[&1].tow_armed);
    fleet.release_tow_ahead(1, &mut Some(&mut manager)).unwrap();
    assert_eq!(fleet.boats[&1].towed, None);
    assert_eq!(fleet.boats[&2].tug, None);
    assert_eq!(fleet.boats[&2].top_state(), None);
}

#[test]
fn invalid_tow_pick_disarms_and_unarmed_messages_forward() {
    let mut fleet = fleet();
    fleet.boats.get_mut(&1).unwrap().tow_armed = true;
    assert_eq!(
        fleet.do_message(1, Message::TowLanding(None), &mut None),
        Ok(MessageDisposition::Handled)
    );
    assert!(!fleet.boats[&1].tow_armed);
    assert_eq!(
        fleet.do_message(1, Message::TowLanding(None), &mut None),
        Ok(MessageDisposition::ForwardToAutomaton)
    );
    assert_eq!(
        fleet.do_message(1, Message::Other(42), &mut None),
        Ok(MessageDisposition::ForwardToAutomaton)
    );
}

#[test]
fn ferry_reader_skips_used_cells_only_once_and_neighbor_exhaustion_removes() {
    let mut fleet = fleet();
    let route = Arc::<[Cell]>::from([
        Cell { x: 0, z: 0 },
        Cell { x: 1, z: 0 },
        Cell { x: 2, z: 0 },
        Cell { x: 3, z: 0 },
    ]);
    let mut s = (*station(1, 0.0)).clone();
    s.route = route.clone();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.ferry = true;
    b.initialize_current_route(Arc::new(s));
    assert_eq!(fleet.next_route_cell(1, &mut None), Ok(Some(Cell { x: 2, z: 0 })));
    assert_eq!(fleet.next_route_cell(1, &mut None), Ok(Some(Cell { x: 3, z: 0 })));
    assert_eq!(fleet.boats[&1].used_cells, 4);
    assert_eq!(fleet.next_route_cell(1, &mut None), Ok(None));
    fleet.boats.get_mut(&1).unwrap().set_neighbor_route(
        NeighborRoute {
            occupant: Arc::new(Resource(8)),
            position: [0.0; 3],
            cells: route,
            next: 0,
        },
        |c| [c.x as f32, 0.0, c.z as f32],
    );
    assert_eq!(fleet.next_route_cell(1, &mut None), Ok(None));
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SLOW_REMOVE));
}

#[test]
fn initialization_acquires_once_shutdown_stops_and_drops_only_owned_resources() {
    struct Resources {
        names: Vec<&'static str>,
        created: Vec<Arc<Resource>>,
    }
    impl EffectResources for Resources {
        fn create(&mut self, name: &'static str) -> Option<Arc<Resource>> {
            self.names.push(name);
            let resource = Arc::new(Resource(self.names.len() as u64));
            self.created.push(resource.clone());
            Some(resource)
        }
    }
    let mut resources = Resources {
        names: vec![],
        created: vec![],
    };
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    assert_eq!(
        b.initialize_runtime([1.0, 2.0, 3.0], [0.0, 0.0, 2.0], &mut resources),
        Ok(true)
    );
    assert_eq!(b.heading, [0.0, 0.0, 1.0]);
    assert_eq!(b.initialize_runtime([0.0; 3], [0.0; 3], &mut resources), Ok(true));
    assert_eq!(resources.names, EFFECTS);
    assert!(resources.created.iter().all(|r| Arc::strong_count(r) == 2));
    fleet.shutdown(1, &mut None).unwrap();
    // Shutdown transfers the acquired references into stop commands. A
    // foreign resource adapter stops each effect before dropping its command.
    assert!(resources.created.iter().all(|r| Arc::strong_count(r) == 2));
    assert_eq!(fleet.shutdown(1, &mut None), Ok(false));
    assert_eq!(
        &fleet.boats[&1].events[2..],
        [
            Event::Unregister(TOW_MESSAGE),
            Event::StopEffect(Arc::new(Resource(1))),
            Event::StopEffect(Arc::new(Resource(2))),
            Event::StopEffect(Arc::new(Resource(3))),
            Event::StopEffect(Arc::new(Resource(4))),
            Event::StopEffect(Arc::new(Resource(5))),
            Event::StopEffect(Arc::new(Resource(6))),
            Event::BaseShutdown
        ]
    );
    fleet.boats.get_mut(&1).unwrap().events.clear();
    assert!(resources.created.iter().all(|r| Arc::strong_count(r) == 1));
}

#[test]
fn absent_effect_is_tolerated_and_shutdown_releases_partial_resources() {
    struct Missing;
    impl EffectResources for Missing {
        fn create(&mut self, name: &'static str) -> Option<Arc<Resource>> {
            (name != "boat_grounding").then(|| Arc::new(Resource(1)))
        }
    }
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    assert_eq!(b.initialize_runtime([0.0; 3], [1.0, 0.0, 0.0], &mut Missing), Ok(true));
    assert_eq!(b.events, [Event::Register(TOW_MESSAGE), Event::ActiveSound(true)]);
    assert!(b.initialized);
    fleet.shutdown(1, &mut None).unwrap();
    assert!(fleet.boats[&1].effects.iter().all(Option::is_none));
}

#[test]
fn crash_case_angle_and_ai_override_do_not_reap_expired_crashes() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    assert_eq!(b.determine_crash_case([true; 4], [0.0; 3]), Ok(CrashCase::SlowRemove));
    b.record.base.states.push(State::new(state::PLAYER_DRIVE));
    assert_eq!(b.determine_crash_case([false; 4], [0.0; 3]), Ok(CrashCase::Sink));
    assert_eq!(
        b.determine_crash_case([true; 4], [0.0, 1.0, 0.0]),
        Ok(CrashCase::RunAground)
    );
    fleet
        .set_state_internal(1, CrashCase::CollisionSink, &mut None)
        .unwrap();
    assert!(fleet.boats[&1].sunk_by_collision);
    assert_eq!(
        fleet.boats[&1].record.base.states.last().unwrap().base.countdown_ms,
        8000.0
    );
    assert_eq!(
        fleet.state_tick(1, 10_000.0, &mut None),
        Err(ContractGap::CrashSequenceWc5)
    );
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SINK));
    assert!(!fleet.boats[&1].events.iter().any(|e| matches!(e, Event::Remove(_))));
}

#[test]
fn docking_final_approach_is_bounded_and_completion_uses_strict_distance() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.set_station_information([station(1, 0.0), station(2, 10.0)], true);
    b.position = [8.0, 0.0, 0.0];
    b.heading = [0.0, 0.0, 1.0];
    b.inherited_position = b.position;
    b.inherited_heading = b.heading;
    assert_eq!(fleet.docking_tick(1, 0.5, &Water, &mut None), Ok(true));
    let b = &fleet.boats[&1];
    assert_eq!(b.position, [8.5, 0.0, 0.0]);
    assert!((b.heading[0] - 0.5_f32.to_radians().sin()).abs() < 0.000001);
    assert!((b.heading[2] - 0.5_f32.to_radians().cos()).abs() < 0.000001);
    let b = fleet.boats.get_mut(&1).unwrap();
    b.heading = [1.0, 0.0, 0.0];
    b.position = [9.0, 0.0, 0.0];
    b.inherited_position = b.position;
    b.inherited_heading = b.heading;
    assert_eq!(fleet.docking_tick(1, 0.0, &Water, &mut None), Ok(true));
    assert_eq!(fleet.docking_tick(1, 0.5, &Water, &mut None), Ok(true));
    assert_eq!(fleet.docking_tick(1, 0.0, &Water, &mut None), Ok(false));
}

#[test]
fn undocking_moves_horizontally_and_does_not_overshoot() {
    let mut fleet = fleet();
    let b = fleet.boats.get_mut(&1).unwrap();
    b.position = [1.0, 4.0, 0.0];
    b.neighbor = Some(NeighborRoute {
        occupant: Arc::new(Resource(1)),
        position: [0.0, -2.0, 0.0],
        cells: Arc::from([]),
        next: 0,
    });
    let rectangle = [Cell { x: 1, z: 1 }; 4];
    assert_eq!(fleet.undocking_tick(1, 0.5, rectangle, &Water, &mut None), Ok(true));
    assert_eq!(fleet.boats[&1].position, [1.5, 0.0, 0.0]);
    assert_eq!(fleet.undocking_tick(1, 2.0, rectangle, &Water, &mut None), Ok(true));
    assert_eq!(fleet.boats[&1].position, [3.5, 0.0, 0.0]);
    assert_eq!(fleet.undocking_tick(1, 100.0, rectangle, &Water, &mut None), Ok(true));
    assert_eq!(fleet.boats[&1].position, [10.0, 0.0, 0.0]);
    assert_eq!(fleet.undocking_tick(1, 0.0, rectangle, &Water, &mut None), Ok(false));
}

#[test]
fn movement_flag_precedes_bounds_then_bounds_precede_crash_state() {
    let mut fleet = fleet();
    fleet.boats.get_mut(&1).unwrap().record.base.automata_flags = 1;
    let outside = [Cell { x: -1, z: 0 }; 4];
    assert_eq!(fleet.movement_possible(1, outside, &Water, &mut None), Ok(false));
    assert!(fleet.boats[&1].events.is_empty());
    fleet.boats.get_mut(&1).unwrap().record.base.automata_flags = 0;
    fleet.push_state(1, State::new(state::SINK), &mut None).unwrap();
    assert_eq!(fleet.movement_possible(1, outside, &Water, &mut None), Ok(false));
    assert_eq!(fleet.boats[&1].top_state(), Some(state::IMMEDIATE_REMOVE));
}

#[test]
fn station_selection_is_per_axis_and_route_restart_preserves_used_count() {
    let mut b = boat();
    let a = station(1, 0.0);
    let other = station(2, 100.0);
    b.set_station_information([a, other.clone()], true);
    b.route_start = [0.00009; 3];
    assert_eq!(b.other_station().unwrap().lot.0, 2);
    b.route_start[1] = 0.001;
    assert_eq!(b.other_station().unwrap().lot.0, 1);
    b.used_cells = 7;
    b.route_started = true;
    b.initialize_current_route(other);
    assert_eq!(b.used_cells, 7);
    assert!(!b.route_started);
}

#[test]
fn final_state_pop_preserves_crash_and_adds_disappear() {
    let mut fleet = fleet();
    fleet.set_state_internal(1, CrashCase::Sink, &mut None).unwrap();
    assert_eq!(fleet.pop_state(1, &mut None), Ok(true));
    assert_eq!(fleet.boats[&1].top_state(), Some(1));
    assert_eq!(fleet.boats[&1].record.base.states[0].state_type, state::SINK);
    assert_eq!(fleet.boats[&1].record.base.states[1].base.stack_index, 1);
}
