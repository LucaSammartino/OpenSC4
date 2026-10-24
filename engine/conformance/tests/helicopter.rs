// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 vector 7; automata.md §§6.2/6.4/6.6 and §14 vectors 15/31.

use fc4_core::automata::{
    state_type, AircraftBounds, AircraftGoalOutcome, AircraftTargets, AircraftUnsupported, AircraftWorld,
    AutomatonBase, ChaseTarget, FlightPhase, SetStateArgs, StateExtra,
};
use fc4_core::helicopter::{
    DriveField, DriveParameterSource, DriveParameters, DriveRecord, Helicopter, HelicopterChaseGeometry, DRIVE_FIELDS,
};
use fc4_core::stream::{ByteReader, ByteWriter, RefValue};

struct Group<'a> {
    record: Option<DriveRecord>,
    parent: Option<&'a dyn DriveParameterSource>,
}

impl DriveParameterSource for Group<'_> {
    fn helicopter_record(&self) -> Option<&DriveRecord> {
        self.record.as_ref()
    }
    fn parent(&self) -> Option<&dyn DriveParameterSource> {
        self.parent
    }
}

fn preset_bits(parameters: &DriveParameters) -> [Option<u32>; 13] {
    parameters.0.map(|value| value.map(f32::to_bits))
}

#[test]
fn vector7_zero_record_preserves_preset_and_bank_only_does_not_merge_parent_pitch() {
    let ancestor = Group {
        record: Some(DriveRecord::import(|name| (name == "PitchMax").then_some(2.0))),
        parent: None,
    };
    let mut group = Group {
        record: Some(DriveRecord::default()),
        parent: Some(&ancestor),
    };
    let mut heli = Helicopter::default();
    heli.drive_parameters = DriveParameters::from_preset(std::array::from_fn(|i| i as f32 + 10.0));
    let before = heli.drive_parameters.clone();
    assert_eq!(heli.load_drive_parameters(Some(&group)), Ok(()));
    assert_eq!(heli.drive_parameters, before);
    group.record = Some(DriveRecord::import(|name| (name == "BankMax").then_some(1.0)));
    assert_eq!(heli.load_drive_parameters(Some(&group)), Ok(()));
    assert_eq!(
        heli.drive_parameters.get(DriveField::BankMax).unwrap().to_bits(),
        0x440F3D4D
    );
    for index in 0..13 {
        if index != DriveField::BankMax as usize {
            assert_eq!(heli.drive_parameters.0[index], before.0[index]);
        }
    }
}

#[test]
fn all_thirteen_named_fields_use_the_specified_destinations_and_exact_constants() {
    let mut imported = Vec::new();
    let record = DriveRecord::import(|name| {
        imported.push(name.to_owned());
        Some(1.0)
    });
    assert_eq!(imported, DRIVE_FIELDS);
    let mut parameters = DriveParameters::default();
    parameters.overlay(&record);
    let expected = [
        0x3F800000, 0x3F800000, 0x3F800000, 0x45505555, 0x3F800000, 0x3F800000, 0x440F3D4D, 0x440F3D4D, 0x440F3D4D,
        0x440F3D4D, 0x440F3D4D, 0x3F800000, 0x3F800000,
    ];
    assert_eq!(preset_bits(&parameters), expected.map(Some));
    for field in [
        DriveField::Mass,
        DriveField::Power,
        DriveField::SpeedMultiplier,
        DriveField::MaxSpeed,
        DriveField::DragFactor,
        DriveField::RotationalDragFactor,
        DriveField::BankMax,
        DriveField::BankChangeRate,
        DriveField::PitchMax,
        DriveField::PitchChangeRate,
        DriveField::YawRateMax,
        DriveField::BankMultiplier,
        DriveField::PitchMultiplier,
    ] {
        assert_eq!(parameters.get(field).unwrap().to_bits(), expected[field as usize]);
    }
}

#[test]
fn missing_fields_are_zero_and_record_selection_uses_first_present_ancestor() {
    let root = Group {
        record: Some(DriveRecord::import(|name| (name == "PitchMax").then_some(2.0))),
        parent: None,
    };
    let missing = Group {
        record: None,
        parent: Some(&root),
    };
    let mut parameters = DriveParameters::from_preset([17.0; 13]);
    parameters.load(Some(&missing));
    assert_eq!(parameters.get(DriveField::PitchMax).unwrap().to_bits(), 0x448F3D4D);
    assert_eq!(parameters.get(DriveField::Mass), Some(17.0));
    let before = parameters.clone();
    parameters.load(None);
    parameters.load(Some(&Group {
        record: None,
        parent: None,
    }));
    assert_eq!(parameters, before);
    assert_eq!(DriveRecord::import(|_| None), DriveRecord::default());
}

#[test]
fn overlay_preserves_signed_zero_and_does_not_sanitize_negative_nan_or_infinity() {
    let original = DriveParameters::from_preset([f32::from_bits(0x7FC01234); 13]);
    for zero in [0.0, -0.0] {
        let mut parameters = original.clone();
        parameters.overlay(&DriveRecord([zero; 13]));
        assert_eq!(preset_bits(&parameters), preset_bits(&original));
    }
    for given in [-1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let mut parameters = DriveParameters::from_preset([5.0; 13]);
        parameters.overlay(&DriveRecord([given; 13]));
        for (index, field) in parameters.0.into_iter().enumerate() {
            let value = field.unwrap();
            if given.is_nan() {
                assert!(value.is_nan());
            } else if given.is_infinite() {
                assert_eq!(value, given);
            } else {
                assert!(value < 0.0);
                assert_ne!(value, 5.0, "field {index}");
            }
        }
    }
    let mut parameters = DriveParameters::default();
    parameters.overlay(&DriveRecord([f32::MAX; 13]));
    assert_eq!(parameters.get(DriveField::MaxSpeed), Some(f32::INFINITY));
}

#[test]
fn unknown_constructor_preset_is_not_fabricated_and_known_overlay_can_fill_it() {
    let mut heli = Helicopter::default();
    assert_eq!(heli.drive_parameters.0, [None; 13]); // AF9 / AF4
    assert_eq!(heli.phase(), None);
    let group = Group {
        record: Some(DriveRecord::import(|name| (name == "BankMax").then_some(1.0))),
        parent: None,
    };
    heli.load_drive_parameters(Some(&group)).unwrap();
    assert_eq!(
        heli.drive_parameters.0.iter().filter(|value| value.is_some()).count(),
        1
    );
    assert_eq!(
        heli.drive_parameters.get(DriveField::BankMax).unwrap().to_bits(),
        0x440F3D4D
    );
}

struct Targets(Option<ChaseTarget>);
impl AircraftTargets for Targets {
    fn chase_target(&self, _: &RefValue) -> Option<ChaseTarget> {
        self.0
    }
}

fn push_chase(heli: &mut Helicopter, reference: RefValue) {
    assert!(heli.base_mut().push_state(52, SetStateArgs::default()));
    heli.base_mut().states.last_mut().unwrap().extra = StateExtra::Chase(reference);
}

#[test]
fn default_dispatch_pops_every_unlisted_numeric_goal_including_crop_dust() {
    for goal in (1..=300).chain([u32::MAX]) {
        if matches!(goal, 50..=56) {
            continue;
        }
        let mut heli = Helicopter::default();
        assert!(heli.base_mut().push_state(0, SetStateArgs::default()));
        assert!(heli.base_mut().push_state(goal, SetStateArgs::default()));
        assert_eq!(
            heli.pursue_goal(&Targets(None)),
            Ok(AircraftGoalOutcome::Popped),
            "goal {goal}"
        );
        assert_eq!(heli.base().states.len(), 1);
        assert_eq!(heli.base().states[0].state_type, 0);
        assert_eq!(heli.base().states[0].base.stack_index, 0);
    }
}

#[test]
fn default_pop_obeys_final_state_rules_and_empty_stack_has_no_handler() {
    let mut heli = Helicopter::default();
    assert_eq!(
        heli.pursue_goal(&Targets(None)),
        Ok(AircraftGoalOutcome::NoAircraftHandler)
    );
    heli.base_mut().push_state(
        58,
        SetStateArgs {
            final_flag: 1,
            ..SetStateArgs::default()
        },
    );
    assert_eq!(heli.pursue_goal(&Targets(None)), Ok(AircraftGoalOutcome::Popped));
    assert_eq!(heli.base().states.last().unwrap().state_type, 1); // DISAPPEAR
}

#[test]
fn unsettled_goal_handlers_are_explicit_and_preserve_the_owner() {
    for goal in [0, 50, 51, 53, 54, 55, 56] {
        let mut heli = Helicopter::default();
        heli.base_mut().push_state(goal, SetStateArgs::default());
        let before = heli.record.clone();
        assert_eq!(heli.pursue_goal(&Targets(None)), Err(AircraftUnsupported::GoalControl));
        assert_eq!(heli.record, before);
    }
}

#[test]
fn chase_rejects_null_unresolved_and_nonplayer_targets_and_keeps_valid_targets() {
    let reference = RefValue::Host {
        token: 123,
        class_id: 0x896E714A,
    };
    let target = ChaseTarget {
        position: [100.0, 200.0, 300.0],
        orientation: [1.0, 0.0, -1.0],
        speed: 0.25,
        top_state: Some(state_type::PLAYER_DRIVE),
    };
    for (reference, target) in [
        (RefValue::Null, Some(target)),
        (reference.clone(), None),
        (
            reference.clone(),
            Some(ChaseTarget {
                top_state: None,
                ..target
            }),
        ),
        (
            reference.clone(),
            Some(ChaseTarget {
                top_state: Some(0),
                ..target
            }),
        ),
    ] {
        let mut heli = Helicopter::default();
        push_chase(&mut heli, reference);
        assert_eq!(heli.pursue_goal(&Targets(target)), Ok(AircraftGoalOutcome::Popped));
        assert!(heli.base().states.is_empty());
    }
    let mut heli = Helicopter::default();
    push_chase(&mut heli, reference);
    let before = heli.record.clone();
    assert_eq!(
        heli.pursue_goal(&Targets(Some(target))),
        Err(AircraftUnsupported::GoalControl)
    );
    assert_eq!(heli.record, before);
    let geometry = HelicopterChaseGeometry::for_target(target, 2.0);
    assert_eq!(geometry.lead_position, [600.0, 200.0, -200.0]);
    assert_eq!(geometry.hover_position, [600.0, 220.0, -200.0]);
    assert_eq!(geometry.spotlight_target, geometry.lead_position);
}

#[test]
fn phase_changes_are_independent_of_stack_and_mission_loiter_has_6000ms_timeout() {
    let mut heli = Helicopter::default();
    for phase in [FlightPhase::Phase0, FlightPhase::Phase1, FlightPhase::Phase3] {
        let change = heli.set_helicopter_state(phase).unwrap();
        assert_eq!(change.current, phase);
        assert_eq!(heli.phase(), Some(phase));
        assert!(heli.set_helicopter_state(phase).is_some());
        assert!(heli.base().states.is_empty());
    }
    assert!(heli.push_mission_loiter());
    let top = heli.base().states.last().unwrap();
    assert_eq!(top.state_type, 53);
    assert_eq!(top.base.countdown_ms, 6000.0);
    assert_eq!(heli.phase(), Some(FlightPhase::Phase3));
}

fn saved_helicopter() -> Vec<u8> {
    let mut heli = Helicopter::default();
    heli.record.home_position = [1.25, -2.5, 30.0];
    heli.base_mut().vectors[0] = [11.0, 22.0, 33.0];
    for goal in [0, 50, 51, 53, 54, 55, 56, 58] {
        heli.base_mut().push_state(goal, SetStateArgs::default());
    }
    push_chase(&mut heli, RefValue::Null);
    let mut out = ByteWriter::new();
    assert!(heli.write(&mut out));
    out.bytes().to_vec()
}

#[test]
fn vector15_same_input_record_round_trip_nests_base_and_home_without_traffic() {
    let bytes = saved_helicopter();
    assert_eq!(&bytes[..4], &[1, 0, 1, 0]);
    let mut base_reader = ByteReader::new(&bytes[4..]);
    let base = AutomatonBase::read(&mut base_reader).unwrap();
    assert_eq!(base.states.last().unwrap().state_type, 52);
    assert!(matches!(
        base.states.last().unwrap().extra,
        StateExtra::Chase(RefValue::Null)
    ));
    assert_eq!(bytes.len(), 4 + base_reader.position() + 12);
    let mut heli = Helicopter::default();
    assert!(heli.read(&mut ByteReader::new(&bytes)));
    assert_eq!(heli.class_id(), 0x8BA98EDA);
    assert!(heli.record.position_loaded);
    assert_eq!(heli.record.home_position, [1.25, -2.5, 30.0]);
    let mut out = ByteWriter::new();
    assert!(heli.write(&mut out));
    assert_eq!(out.bytes(), bytes);
    heli.init_placement();
    assert_eq!(heli.base().vectors[0], [1.25, -2.5, 30.0]);
    heli.base_mut().vectors[0] = [9.0; 3];
    heli.init_placement();
    assert_eq!(heli.base().vectors[0], [9.0; 3]);
}

#[test]
fn reads_are_atomic_at_every_truncation_and_reject_bad_nested_versions() {
    let bytes = saved_helicopter();
    let mut heli = Helicopter::default();
    heli.drive_parameters = DriveParameters::from_preset([23.0; 13]);
    heli.set_helicopter_state(FlightPhase::Phase3);
    let before = heli.record.clone();
    for end in 0..bytes.len() {
        assert!(!heli.read(&mut ByteReader::new(&bytes[..end])), "prefix {end}");
        assert_eq!(heli.record, before);
        assert_eq!(heli.phase(), Some(FlightPhase::Phase3));
        assert_eq!(heli.drive_parameters.0, [Some(23.0); 13]);
    }
    for (offset, versions) in [(0, [2u16, 65535]), (2, [0u16, 2])] {
        for version in versions {
            let mut invalid = bytes.clone();
            invalid[offset..offset + 2].copy_from_slice(&version.to_le_bytes());
            assert!(!heli.read(&mut ByteReader::new(&invalid)));
            assert_eq!(heli.record, before);
        }
    }
    let mut zero_version = bytes.clone();
    zero_version[..2].copy_from_slice(&0u16.to_le_bytes());
    assert!(heli.read(&mut ByteReader::new(&zero_version)));
    let mut out = ByteWriter::new();
    assert!(heli.write(&mut out));
    assert_eq!(out.bytes(), bytes); // concrete writers always emit 1
    assert_eq!(heli.phase(), Some(FlightPhase::Phase3));
    assert_eq!(heli.drive_parameters.0, [Some(23.0); 13]); // Read does not construct/reset a runtime.
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
        3.0
    }
    fn water_height(&self, _: f32, _: f32) -> Option<f32> {
        Some(7.0)
    }
    fn building_height(&self, _: f32, _: f32) -> Option<f32> {
        Some(40.0)
    }
    fn prop_height(&self, _: f32, _: f32) -> Option<f32> {
        Some(50.0)
    }
}

#[test]
fn placement_capture_altitude_and_world_margin_reuse_aircraft_contracts() {
    let mut heli = Helicopter::default();
    heli.base_mut().vectors[0] = [4.0, 5.0, 6.0];
    heli.init_placement();
    heli.base_mut().vectors[0] = [8.0; 3];
    heli.init_placement();
    assert_eq!(heli.record.home_position, [4.0, 5.0, 6.0]);
    assert_eq!(heli.get_altitude(&World, 10.0, 20.0), 50.0);
    assert_eq!(
        heli.clip_to_world(&World, [-2000.0, -9.0, 5000.0]),
        [-1000.0, -9.0, 3048.0]
    );
}

#[test]
fn open_rotor_ai_lifecycle_and_player_methods_never_report_success_or_mutate() {
    let mut heli = Helicopter::default();
    let before = heli.record.clone();
    assert_eq!(heli.physics_init(), Err(AircraftUnsupported::EffectsAndLifecycle));
    assert_eq!(
        heli.update_rotor_sounds(),
        Err(AircraftUnsupported::EffectsAndLifecycle)
    );
    for result in [
        heli.next_frame(16.0, &World),
        heli.yaw_pitch_roll_slide_real(),
        heli.move_xz_real(),
        heli.move_y_real(),
    ] {
        assert_eq!(result, Err(AircraftUnsupported::FlightModel));
    }
    assert_eq!(heli.ai_control_move_xz(), Err(AircraftUnsupported::GoalControl));
    for result in [
        heli.update_spotlight(),
        heli.translate_player_drive_commands(),
        heli.do_player_drive_tick(),
    ] {
        assert_eq!(result, Err(AircraftUnsupported::PlayerControls));
    }
    assert_eq!(heli.record, before);
    assert_eq!(heli.drive_parameters.0, [None; 13]);
    assert_eq!(heli.phase(), None);
}
