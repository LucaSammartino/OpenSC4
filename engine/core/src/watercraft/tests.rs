// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent vectors from watercraft §§2-7,9,11, not captured native output.
use super::*;
use crate::automata::{AutomatonBase, State};
use crate::stream::{ByteReader, ByteWriter};
use std::sync::Arc;

fn group() -> Arc<Group> {
    Arc::new(Group {
        id: 0,
        parameters: None,
        aems_value: Some(1.0),
        parent: None,
    })
}

fn boat() -> Watercraft {
    let mut boat = Watercraft::new(WatercraftRecord { base: AutomatonBase::new() }, group(), 5.0);
    boat.heading = [1.0, 0.0, 0.0];
    boat.previous_heading = boat.heading;
    boat
}

fn fleet() -> Fleet {
    let mut fleet = Fleet::default();
    fleet.boats.insert(1, boat());
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

struct Water;
impl Terrain for Water {
    fn contains(&self, cell: Cell) -> bool { (0..100).contains(&cell.x) && (0..100).contains(&cell.z) }
    fn cell_at(&self, position: [f32; 3]) -> Cell { Cell { x: position[0] as i32, z: position[2] as i32 } }
    fn terrain_height(&self, _: [f32; 3]) -> f32 { -10.0 }
    fn sea_level(&self) -> f32 { 0.0 }
    fn water_height(&self, _: [f32; 3]) -> f32 { 0.0 }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool { true }
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
    assert_eq!(WatercraftRecord::read(&mut ByteReader::new(bytes)).unwrap(), original.record);
    for end in 0..bytes.len() {
        assert!(WatercraftRecord::read(&mut ByteReader::new(&bytes[..end])).is_none(), "truncation {end}");
    }
    let mut bad = bytes.to_vec();
    bad[0] = 3;
    assert!(WatercraftRecord::read(&mut ByteReader::new(&bad)).is_none());
}

#[test]
fn state_entry_exit_and_remove_side_effect_order() {
    let mut fleet = fleet();
    let mut no_manager = None;
    fleet.push_state(1, State::new(state::PLAYER_DRIVE), &mut no_manager).unwrap();
    let b = &fleet.boats[&1];
    assert_eq!(b.player.field_144, 15);
    assert_eq!(b.player.field_1ac, 2000.0);
    assert_eq!(b.player.turn_rate, 0.2618);
    assert_eq!(b.events, [Event::ActiveSound(false)]);
    fleet.pop_state(1, &mut no_manager).unwrap();
    fleet.push_state(1, State::new(state::SLOW_REMOVE), &mut no_manager).unwrap();
    fleet.state_tick(1, 1.0, &mut no_manager).unwrap();
    assert_eq!(fleet.boats[&1].events, [Event::ActiveSound(false), Event::ActiveSound(true), Event::Remove(false), Event::Remove(false)]);
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
    assert_eq!(fleet.state_tick(1, 1.0, &mut manager), Err(ContractGap::CrashSequenceWc5));
    assert_eq!(fleet.boats[&1].idle_elapsed, 60_001.0);
    assert_eq!(fleet.boats[&1].crash_elapsed, 1.0);
    assert_eq!(fleet.boats[&1].top_state(), Some(state::SLOW_REMOVE));
}

#[test]
fn ferry_public_vector_selects_berth_wait_or_remove_and_preserves_order() {
    for (berth, waiting, expected, calls) in [
        (true, true, state::DOCK, vec!["berth:2:1"]),
        (false, true, state::QUEUE, vec!["berth:2:1", "wait:2:1"]),
        (false, false, state::SLOW_REMOVE, vec!["berth:2:1", "wait:2:1"]),
    ] {
        let mut fleet = fleet();
        fleet.boats.get_mut(&1).unwrap().set_station_information([station(1, 0.0), station(2, 250.0)], true);
        let mut manager = Manager { berth, waiting, ..Manager::default() };
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
    assert_eq!(fleet.docking_tick(1, 1.0, &Water, &mut None), Ok(false));
    fleet.boats.get_mut(&1).unwrap().position = [200.0, 0.0, 0.0];
    assert_eq!(fleet.docking_tick(1, 1.0, &Water, &mut None), Err(ContractGap::AiMotionWc4));
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
    b.group = Arc::new(Group { aems_value: None, ..(*group()).clone() });
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
