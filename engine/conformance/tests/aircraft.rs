// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 tests 1–3, automata.md §§6.4,6.6,14 test 15.
//! Test 3 covers the source's property rule through a foreign station boundary.

use fc4_core::automata::*;
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefValue};

struct Station {
    building: bool,
    units: Option<u32>,
    available: Option<u32>,
}

impl AirStationProperties for Station {
    fn is_building_occupant(&self) -> bool {
        self.building
    }
    fn property_u32(&self, id: u32) -> Option<u32> {
        assert_eq!(id, 0x0C0AFDD7);
        self.units
    }
    fn set_property_u32(&mut self, id: u32, value: u32) {
        assert_eq!(id, 0xCC0AFDDB);
        self.available = Some(value);
    }
}

#[test]
fn fire_plane_station_with_three_units_sets_available_property_to_seven() {
    // aircraft §8 test 3 checks the actual property write, not a fake
    // dispatch manager computing an expected bit mask for itself.
    let mut station = Station {
        building: true,
        units: Some(3),
        available: None,
    };
    assert_eq!(initialize_fire_plane_station_units(&mut station), Ok(Some(3)));
    assert_eq!(station.available, Some(7));
    for count in [0, 32, 33] {
        station.units = Some(count);
        station.available = None;
        let result = initialize_fire_plane_station_units(&mut station);
        match count {
            0 => {
                assert_eq!(result, Ok(None));
                assert_eq!(station.available, None);
            }
            32 => {
                assert_eq!(result, Ok(Some(32)));
                assert_eq!(station.available, Some(u32::MAX));
            }
            _ => {
                assert_eq!(result, Err(AircraftUnsupported::StationUnitCount));
                assert_eq!(station.available, None);
            }
        }
    }
    station.units = None;
    assert_eq!(initialize_fire_plane_station_units(&mut station), Ok(None));
    station.building = false;
    station.units = Some(3);
    assert_eq!(initialize_fire_plane_station_units(&mut station), Ok(None));
    assert_eq!(station.available, None);
}

/// Fail during the nested automaton header. Any attempted home float write
/// after this failure is a violation of automata §6.4's short-circuit rule.
#[derive(Default)]
struct HeaderFailure {
    words: usize,
    floats: usize,
    failed: bool,
}

impl OutStream for HeaderFailure {
    fn put_u8(&mut self, _: u8) -> bool {
        !self.failed
    }
    fn put_u16(&mut self, _: u16) -> bool {
        self.words += 1;
        if self.words == 3 {
            self.failed = true;
        }
        !self.failed
    }
    fn put_u32(&mut self, _: u32) -> bool {
        !self.failed
    }
    fn put_u64(&mut self, _: u64) -> bool {
        !self.failed
    }
    fn put_f32(&mut self, _: f32) -> bool {
        self.floats += 1;
        !self.failed
    }
    fn put_f64(&mut self, _: f64) -> bool {
        !self.failed
    }
    fn put_bytes(&mut self, _: &[u8]) -> bool {
        !self.failed
    }
    fn put_str(&mut self, _: &[u8]) -> bool {
        !self.failed
    }
    fn put_ref(&mut self, _: &RefValue) -> bool {
        !self.failed
    }
    fn error(&self) -> i32 {
        i32::from(self.failed)
    }
}

#[test]
fn failed_aircraft_write_never_attempts_home_vector() {
    let mut failure = HeaderFailure::default();
    assert!(!Airplane::default().write(&mut failure));
    assert!(failure.failed);
    assert_eq!(failure.floats, 0);
    let mut failure = HeaderFailure::default();
    assert!(!Helicopter::default().write(&mut failure));
    assert_eq!(failure.floats, 0);
    let mut failure = HeaderFailure {
        failed: true,
        ..HeaderFailure::default()
    };
    assert!(!Airplane::default().write(&mut failure));
    assert_eq!(failure.words, 0);
}

struct Exemplar<'a> {
    list: Option<&'a [f32]>,
    parent: Option<&'a dyn DriveParameterSource>,
}

impl DriveParameterSource for Exemplar<'_> {
    fn drive_parameters(&self) -> Option<&[f32]> {
        self.list
    }
    fn parent(&self) -> Option<&dyn DriveParameterSource> {
        self.parent
    }
}

#[test]
fn airplane_default_and_explicit_speed_parameters() {
    // aircraft §8 test 1. No list uses the printed defaults, not ×1000.
    let defaults = AirplaneDriveParameters::load(None).0;
    assert_eq!(
        defaults,
        [
            100.0, 1.0, 25.0, 150.0, 0.2618, 0.06545, 0.7854, 0.3927, 0.5236, 0.04363, 10000.0, 10000.0, 0.4, 0.04,
            0.5236, 0.08, 0.68, 0.0
        ]
    );
    let mut list = [0.0; 18];
    list[2] = 3.0;
    list[3] = 0.5;
    let values = AirplaneDriveParameters::from_list(Some(&list)).0;
    assert_eq!(&values[..4], &[100.0, 1.0, 3000.0, 500.0]);
    list[2] = 0.0;
    list[3] = 0.0;
    assert_eq!(&AirplaneDriveParameters::from_list(Some(&list)).0[2..4], &[25.0, 150.0]);
}

#[test]
fn airplane_derived_defaults_and_explicit_overrides() {
    // aircraft §8 test 2, extended to every derived pair.
    let mut list = [0.0; 18];
    list[4] = 0.4;
    list[6] = 0.6;
    list[8] = 1.2;
    list[10] = 800.0;
    list[12] = 0.8;
    let values = AirplaneDriveParameters::from_list(Some(&list)).0;
    for (index, expected) in [(5, 0.1), (7, 0.3), (9, 0.1), (11, 800.0), (13, 0.08)] {
        assert!((values[index] - expected).abs() < 1e-6);
        list[index] = 2.5;
    }
    let values = AirplaneDriveParameters::from_list(Some(&list)).0;
    for index in [5, 7, 9, 11, 13] {
        assert_eq!(values[index], 2.5);
    }
}

#[test]
fn nearest_parameter_list_stops_parent_inheritance() {
    let parent_list = [2.0; 18];
    let root = Exemplar {
        list: Some(&parent_list),
        parent: None,
    };
    let middle = Exemplar {
        list: None,
        parent: Some(&root),
    };
    let child = Exemplar {
        list: None,
        parent: Some(&middle),
    };
    assert_eq!(AirplaneDriveParameters::load(Some(&child)).0[2], 2000.0);
    let own_list = [0.0; 18];
    let child = Exemplar {
        list: Some(&own_list),
        parent: Some(&root),
    };
    assert_eq!(AirplaneDriveParameters::load(Some(&child)).0[2], 25.0);
    let empty = Exemplar {
        list: Some(&[]),
        parent: Some(&root),
    };
    assert_eq!(AirplaneDriveParameters::load(Some(&empty)).0[0], 100.0);
}

#[test]
fn init_loads_independent_ai_and_player_lists() {
    let ai_list = [2.0; 18];
    let player_list = [3.0; 18];
    let ai = Exemplar {
        list: Some(&ai_list),
        parent: None,
    };
    let player = Exemplar {
        list: Some(&player_list),
        parent: None,
    };
    let mut plane = Airplane::default();
    plane.init_data(Some(&ai), Some(&player));
    assert_eq!(plane.drive_parameters.0[2], 2000.0);
    assert_eq!(plane.player_parameters.0[2], 3000.0);
}

fn saved_placement() -> Vec<u8> {
    let mut plane = Airplane::default();
    plane.base_mut().vectors[0] = [12.0, 70.0, 30.0];
    plane.base_mut().vectors[1] = [45.0, 100.0, 90.0];
    plane.base_mut().vectors[2] = [0.0, 0.0, 1.0];
    plane.init_data(None, None);
    // Save in flight: home must stay the placement, not the current position.
    plane.base_mut().vectors[0] = [400.0, 900.0, 500.0];
    plane.base_mut().push_state(51, SetStateArgs::default());
    let mut out = ByteWriter::new();
    assert!(plane.write(&mut out));
    out.into_bytes()
}

#[test]
fn aircraft_records_nest_base_and_home_without_traffic() {
    // automata §14 test 15; independent assembly of the expected layout.
    let bytes = saved_placement();
    assert_eq!(&bytes[..4], &[1, 0, 1, 0]);
    let mut inp = ByteReader::new(&bytes[4..]);
    let base = AutomatonBase::read(&mut inp).unwrap();
    let base_end = 4 + inp.position();
    assert_eq!(bytes.len(), base_end + 12);
    assert_eq!(&bytes[base_end..base_end + 4], &12.0f32.to_le_bytes());
    assert_eq!(&bytes[base_end + 4..base_end + 8], &70.0f32.to_le_bytes());
    assert_eq!(&bytes[base_end + 8..], &30.0f32.to_le_bytes());
    assert_eq!(base.states.last().unwrap().state_type, 51);

    let mut plane = Airplane::default();
    assert!(plane.read(&mut ByteReader::new(&bytes)));
    assert!(plane.record.position_loaded);
    assert_eq!(plane.base().vectors[0], [400.0, 900.0, 500.0]);
    let mut out = ByteWriter::new();
    assert!(plane.write(&mut out));
    assert_eq!(out.bytes(), bytes);
    let mut heli = Helicopter::default();
    assert!(heli.read(&mut ByteReader::new(&bytes)));
    let mut out = ByteWriter::new();
    assert!(heli.write(&mut out));
    assert_eq!(out.bytes(), bytes);
    assert_eq!(plane.class_id(), 0xCBEB4F62);
    assert_eq!(heli.class_id(), 0x8BA98EDA);
}

#[test]
fn concrete_versions_accept_zero_but_aircraft_base_requires_one() {
    let original = saved_placement();
    for concrete in [0u16, 1, 2, u16::MAX] {
        for base in [0u16, 1, 2, u16::MAX] {
            let mut bytes = original.clone();
            bytes[..2].copy_from_slice(&concrete.to_le_bytes());
            bytes[2..4].copy_from_slice(&base.to_le_bytes());
            let expected = concrete <= 1 && base == 1;
            let mut plane = Airplane::default();
            let mut heli = Helicopter::default();
            assert_eq!(plane.read(&mut ByteReader::new(&bytes)), expected);
            assert_eq!(heli.read(&mut ByteReader::new(&bytes)), expected);
            if expected {
                let mut out = ByteWriter::new();
                assert!(plane.write(&mut out));
                assert_eq!(&out.bytes()[..4], &[1, 0, 1, 0]);
            }
        }
    }
}

#[test]
fn truncated_owner_reads_are_atomic_at_every_byte_boundary() {
    let bytes = saved_placement();
    let mut plane = Airplane::default();
    plane.base_mut().vectors[0] = [-1.0, -2.0, -3.0];
    plane.set_airplane_state(FlightPhase::Phase1);
    let mut heli = Helicopter::default();
    heli.base_mut().vectors[0] = [-4.0, -5.0, -6.0];
    heli.set_helicopter_state(FlightPhase::Phase3);
    let before_plane = plane.record.clone();
    let before_heli = heli.record.clone();
    for end in 0..bytes.len() {
        assert!(!plane.read(&mut ByteReader::new(&bytes[..end])), "plane prefix {end}");
        assert!(!heli.read(&mut ByteReader::new(&bytes[..end])), "heli prefix {end}");
        assert_eq!(plane.record, before_plane);
        assert_eq!(heli.record, before_heli);
        assert_eq!(plane.ammunition(), Some(3));
        assert_eq!(heli.phase(), Some(FlightPhase::Phase3));
    }
}

#[test]
fn first_init_restores_loaded_home_and_never_recaptures_it() {
    let bytes = saved_placement();
    let mut plane = Airplane::default();
    assert!(plane.read(&mut ByteReader::new(&bytes)));
    plane.init_data(None, None);
    assert_eq!(plane.base().vectors[0], [12.0, 70.0, 30.0]);
    plane.base_mut().vectors[0] = [200.0, 300.0, 400.0];
    plane.init_data(None, None);
    assert_eq!(plane.base().vectors[0], [200.0, 300.0, 400.0]);
    assert_eq!(plane.record.home_position, [12.0, 70.0, 30.0]);
    let mut heli = Helicopter::default();
    heli.base_mut().vectors[0] = [4.0, 5.0, 6.0];
    heli.record.init_placement();
    heli.base_mut().vectors[0] = [40.0, 50.0, 60.0];
    heli.record.init_placement();
    assert_eq!(heli.record.home_position, [4.0, 5.0, 6.0]);
    assert_eq!(heli.base().vectors[0], [40.0, 50.0, 60.0]);
}

struct World;
impl AircraftWorld for World {
    fn bounds(&self) -> AircraftBounds {
        AircraftBounds {
            min_x: 0.0,
            min_z: 0.0,
            max_x: 1024.0,
            max_z: 2048.0,
        }
    }
    fn terrain_height(&self, _: f32, _: f32) -> f32 {
        5.0
    }
    fn water_height(&self, x: f32, _: f32) -> Option<f32> {
        (x > 0.0).then_some(10.0)
    }
    fn building_height(&self, x: f32, _: f32) -> Option<f32> {
        (x > 1.0).then_some(30.0)
    }
    fn prop_height(&self, x: f32, _: f32) -> Option<f32> {
        (x > 2.0).then_some(50.0)
    }
}

#[test]
fn altitude_uses_highest_surface_and_clip_keeps_outward_margin() {
    for (x, expected) in [(0.0, 5.0), (1.0, 10.0), (2.0, 30.0), (3.0, 50.0)] {
        assert_eq!(aircraft_altitude(&World, x, 0.0), expected);
    }
    assert_eq!(
        clip_aircraft_to_world([-2000.0, 99.0, 4000.0], World.bounds()),
        [-1000.0, 99.0, 3048.0]
    );
    assert_eq!(
        clip_aircraft_to_world([3000.0, -12.0, -2000.0], World.bounds()),
        [2024.0, -12.0, -1000.0]
    );
    let allowed = [-999.0, 100.0, 2500.0];
    assert_eq!(clip_aircraft_to_world(allowed, World.bounds()), allowed);
}

#[test]
fn phase_and_loiter_ammo_are_independent_of_goal_stack() {
    let mut plane = Airplane::default();
    assert!(plane.set_airplane_state(FlightPhase::Phase1).is_some());
    assert_eq!(plane.ammunition(), Some(3));
    plane.base_mut().push_state(50, SetStateArgs::default());
    plane.begin_loiter();
    assert_eq!(plane.base().states.last().unwrap().base.countdown_ms, -1.0);
    assert_eq!(plane.tick_loiter(2000.0), Ok(AircraftGoalOutcome::Waiting));
    assert_eq!(plane.tick_loiter(1.0), Ok(AircraftGoalOutcome::Popped));
    assert_eq!(plane.ammunition(), Some(2));
    assert!(plane.set_airplane_state(FlightPhase::Phase1).is_none());
    assert_eq!(plane.ammunition(), Some(2));
    assert_eq!(plane.base().states.last().unwrap().state_type, 50);
    plane.set_airplane_state(FlightPhase::Phase3);
    plane.set_airplane_state(FlightPhase::Phase1);
    assert_eq!(plane.ammunition(), Some(3));
    let mut heli = Helicopter::default();
    heli.set_helicopter_state(FlightPhase::Phase1);
    assert!(heli.set_helicopter_state(FlightPhase::Phase1).is_none());
    assert!(heli.base().states.is_empty());
    assert!(heli.push_mission_loiter());
    assert_eq!(heli.base().states.last().unwrap().base.countdown_ms, 6000.0);
}

struct Targets(Option<ChaseTarget>);
impl AircraftTargets for Targets {
    fn chase_target(&self, _: &RefValue) -> Option<ChaseTarget> {
        self.0
    }
}

fn push_chase(base: &mut AutomatonBase, target: RefValue) {
    base.push_state(state_type::CHASE, SetStateArgs::default());
    base.states.last_mut().unwrap().extra = StateExtra::Chase(target);
}

#[test]
fn chase_rejection_obeys_shared_final_state_and_stack_indices() {
    let mut plane = Airplane::default();
    plane.base_mut().push_state(50, SetStateArgs::default());
    push_chase(plane.base_mut(), RefValue::Null);
    assert_eq!(plane.pursue_goal(), Ok(AircraftGoalOutcome::Popped));
    assert_eq!(plane.base().states.len(), 1);
    push_chase(plane.base_mut(), RefValue::Null);
    plane.base_mut().states.last_mut().unwrap().base.is_final = true;
    assert_eq!(plane.pursue_goal(), Ok(AircraftGoalOutcome::Popped));
    assert_eq!(plane.base().states.last().unwrap().state_type, 1);
    assert_eq!(
        plane
            .base()
            .states
            .iter()
            .map(|s| s.base.stack_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn helicopter_chase_pops_invalid_target_but_reports_missing_controls_for_valid_target() {
    let mut heli = Helicopter::default();
    push_chase(heli.base_mut(), RefValue::Null);
    assert_eq!(heli.pursue_goal(&Targets(None)), Ok(AircraftGoalOutcome::Popped));
    let reference = RefValue::Host {
        token: 1,
        class_id: CLSID_VEHICLE,
    };
    push_chase(heli.base_mut(), reference.clone());
    let target = ChaseTarget {
        position: [100.0, 200.0, 300.0],
        orientation: [1.0, 0.0, 0.0],
        speed: 0.25,
        top_state: Some(0),
    };
    assert_eq!(
        heli.pursue_goal(&Targets(Some(target))),
        Ok(AircraftGoalOutcome::Popped)
    );
    push_chase(heli.base_mut(), reference);
    let target = ChaseTarget {
        top_state: Some(state_type::PLAYER_DRIVE),
        ..target
    };
    let before = heli.record.clone();
    assert_eq!(
        heli.pursue_goal(&Targets(Some(target))),
        Err(AircraftUnsupported::GoalControl)
    );
    assert_eq!(heli.record, before);
    let geometry = HelicopterChaseGeometry::for_target(target, 2.0);
    assert_eq!(geometry.lead_position, [600.0, 200.0, 300.0]);
    assert_eq!(geometry.hover_position, [100.0, 220.0, 300.0]);
    assert_eq!(geometry.spotlight_target, target.position);
}

#[test]
fn name_only_crop_dust_and_unsettled_models_do_not_mutate_owners() {
    let mut plane = Airplane::default();
    plane.base_mut().push_state(58, SetStateArgs::default());
    let before = plane.record.clone();
    assert_eq!(plane.pursue_goal(), Ok(AircraftGoalOutcome::NoAircraftHandler));
    assert_eq!(
        plane.update_physics(1000.0, &World),
        Err(AircraftUnsupported::FlightModel)
    );
    assert_eq!(plane.record, before);
    let mut heli = Helicopter::default();
    heli.base_mut().push_state(58, SetStateArgs::default());
    let before = heli.record.clone();
    assert_eq!(
        heli.pursue_goal(&Targets(None)),
        Ok(AircraftGoalOutcome::NoAircraftHandler)
    );
    assert_eq!(
        heli.load_drive_parameters(None),
        Err(AircraftUnsupported::DriveParameterMeaning)
    );
    assert_eq!(heli.next_frame(1000.0, &World), Err(AircraftUnsupported::FlightModel));
    assert_eq!(heli.record, before);
}

#[test]
fn airplane_effect_resources_choose_fire_plane_propellers() {
    assert_eq!(
        airplane_effect_names(false),
        [
            "fx_00_planepropstill",
            "fx_00_planeprop",
            "fx_00_skywriting",
            "fx_00_waterdrop",
            "aircraftindicate",
            "jetbomb"
        ]
    );
    let fire = airplane_effect_names(true);
    assert_eq!(&fire[..2], &["fx_00_fireplanepropstill", "fx_00_fireplaneprop"]);
    assert_eq!(&fire[2..], &airplane_effect_names(false)[2..]);
}
