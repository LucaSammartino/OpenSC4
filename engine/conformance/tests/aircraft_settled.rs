// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 vectors 14–27, using independent supplied inputs.
use fc4_core::automata::{
    AircraftBounds, AircraftGoalOutcome, AircraftWeaponEffects, AircraftWorld, Airplane, ChaseTarget,
    EngineNotification, FlightPhase, Helicopter, PlayerWeaponRecord, SetStateArgs, SoundRequest, WeaponOutcome,
};
use fc4_core::helicopter::*;
use fc4_core::stream::{ByteReader, ByteWriter};

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!((actual - expected).abs() <= tolerance, "{actual} != {expected}");
}
fn controls() -> RotorControls {
    RotorControls {
        lateral: 30.0,
        longitudinal: -30.0,
        collective: 0.0,
        yaw: -20.0,
    }
}
struct PlayerSource {
    direct: Option<PlayerTakeoverRecord>,
    script: Option<PlayerTakeoverRecord>,
}
impl PlayerParameterSource for PlayerSource {
    fn direct_record(&self) -> Option<&PlayerTakeoverRecord> {
        self.direct.as_ref()
    }
    fn script_record(&self) -> Option<&PlayerTakeoverRecord> {
        self.script.as_ref()
    }
}
struct Group(DriveRecord);
impl DriveParameterSource for Group {
    fn helicopter_record(&self) -> Option<&DriveRecord> {
        Some(&self.0)
    }
    fn parent(&self) -> Option<&dyn DriveParameterSource> {
        None
    }
}
#[derive(Default)]
struct Allocator {
    calls: Vec<RuntimeResource>,
    fail: Option<RuntimeResource>,
}
impl RuntimeAllocator for Allocator {
    fn allocate(&mut self, resource: RuntimeResource) -> bool {
        self.calls.push(resource);
        Some(resource) != self.fail
    }
}

#[test]
fn vectors14_16_preset_overlay_order_and_read_reconstruction() {
    let mut heli = Helicopter::default();
    heli.init_instance().unwrap();
    heli.physics_init().unwrap();
    assert_eq!(heli.drive_parameters.0.map(|v| v.unwrap().to_bits()), PRESET0_BITS);
    assert_eq!(heli.phase(), Some(FlightPhase::Phase0));
    assert_eq!(heli.engine_enabled, Some(false));
    let player = PlayerSource {
        direct: Some(PlayerTakeoverRecord {
            speed: 0.0,
            gains: [0.0; 4],
        }),
        script: None,
    };
    let mut second = Helicopter::default();
    second
        .init_instance_with(&mut CoreAllocator, Some(&player), None)
        .unwrap();
    assert_eq!(second.drive_parameters.get(DriveField::MaxSpeed), Some(400.0));
    let group = Group(DriveRecord::import(|name| (name == "MaxSpeed").then_some(3.0)));
    let mut third = Helicopter::default();
    third
        .init_instance_with(&mut CoreAllocator, Some(&player), Some(&group))
        .unwrap();
    near(
        third.drive_parameters.get(DriveField::MaxSpeed).unwrap(),
        10000.0,
        0.001,
    );
    heli.base_mut().push_state(54, SetStateArgs::default());
    heli.set_helicopter_state(FlightPhase::Phase4);
    let mut out = ByteWriter::new();
    assert!(heli.write(&mut out));
    let mut loaded = Helicopter::default();
    assert!(loaded.read(&mut ByteReader::new(out.bytes())));
    assert!(loaded.runtime.is_none());
    assert_eq!(loaded.phase(), None);
    loaded.init_placement();
    loaded.init_instance().unwrap();
    loaded.physics_init().unwrap();
    assert_eq!(loaded.phase(), Some(FlightPhase::Phase0));
    assert_eq!(loaded.base().states.last().unwrap().state_type, 54);
    loaded.drive_parameters.0[0] = Some(7.0);
    let mut allocator = Allocator::default();
    loaded.init_instance_with(&mut allocator, None, None).unwrap();
    assert!(allocator.calls.is_empty());
    loaded.physics_init().unwrap();
    assert_eq!(loaded.drive_parameters.get(DriveField::Mass), Some(7.0));
}

#[test]
fn vectors17_18_reset_hover_and_time_multiplier_eligibility() {
    let preset = DriveParameters::preset0();
    let mut c = controls();
    let demands = c.reset(0.1, 1.0, &preset).unwrap();
    for (a, b) in [c.lateral, c.longitudinal, c.collective, c.yaw]
        .into_iter()
        .zip([20.0, -20.0, -10.0, -10.0])
    {
        near(a, b, 0.0001);
    }
    near(demands.bank, 200.0, 0.0001);
    near(demands.pitch, 200.0, 0.0001);
    let before = c;
    let demands = c.reset(0.0, 0.5, &preset).unwrap();
    assert_eq!(c, before);
    near(demands.pitch, 100.0, 0.0001);
    c.reset(1.0, 1.0, &preset).unwrap();
    assert_eq!(
        [c.lateral, c.longitudinal, c.collective, c.yaw],
        [0.0, 0.0, -100.0, 0.0]
    );
    for (vy, old, height, expected) in [
        (10.0, 0.0, 100.0, -1.575),
        (0.0, 0.0, 1000.0, 10.0),
        (0.0, 99.0, 1000.0, 100.0),
        (0.0, -149.0, -10000.0, -150.0),
    ] {
        c.collective = old;
        c.hover(0.1, 100.0, vy, height);
        near(c.collective, expected, 0.00001);
    }
    near(helicopter_physics_seconds(100.0, 0.5, false, false), 0.05, 0.000001);
    near(helicopter_physics_seconds(100.0, 0.5, true, false), 0.1, 0.000001);
    near(helicopter_physics_seconds(100.0, 0.5, false, true), 0.1, 0.000001);
}

#[test]
fn vector19_chase_lead_opacity_and_distance_boundaries() {
    let mut target = ChaseTarget {
        position: [10.0, 20.0, 30.0],
        orientation: [1.0, 0.0, 0.0],
        speed: 0.02,
        top_state: Some(16),
    };
    let a = chase_actions([0.0, 40.0, 0.0], target, 0.1, 0.0, 123.0);
    near(a.aim[0], 12.0, 0.00001);
    target.position = [200.0, 10.0, 0.0];
    target.speed = 0.0;
    for opacity in [0.0, 1.0, 0.5] {
        let a = chase_actions([0.0, 40.0, 0.0], target, 0.1, opacity, 123.0);
        assert_eq!(a.horizontal_vector, [200.0, 0.0, 0.0]);
        assert_eq!(a.height_control, ChaseHeightControl::BuildingClearance);
        assert_eq!(a.speed_argument, 2.0);
        if opacity == 0.0 {
            assert_eq!(a.teleport, Some([72.0, 40.0, 0.0]));
            assert_eq!(a.opacity_request, Some((1.0, 123.0)));
            assert_eq!(
                a.scaled_rotor_position.unwrap(),
                [72.0, 40.0, 0.0].map(|v| v * f32::from_bits(0x40555555))
            );
        } else {
            assert_eq!(a.teleport, None);
            assert_eq!(
                a.opacity_request,
                if opacity == 1.0 { Some((0.0, 123.0)) } else { None }
            );
        }
    }
    for distance in [192.0, 32.0, 0.0] {
        target.position[0] = distance;
        let a = chase_actions([0.0, 40.0, 0.0], target, 0.1, 0.0, 123.0);
        assert_eq!(a.opacity_request, None);
        assert_eq!(
            a.height_control,
            if distance <= 32.0 {
                ChaseHeightControl::Hover(30.0)
            } else {
                ChaseHeightControl::BuildingClearance
            }
        );
    }
}

#[test]
fn vector20_loiter_spends_on_entry_countdown_expiry_and_unsaved_progress() {
    let mut plane = Airplane::default();
    assert_eq!(plane.ammunition(), Some(3));
    plane.set_ammunition(0);
    plane.begin_loiter();
    assert_eq!(plane.pursue_goal(), Ok(AircraftGoalOutcome::Waiting));
    assert_eq!(plane.ammunition(), Some(-1));
    assert_eq!(plane.loiter_remaining_seconds, 2.0);
    assert_eq!(plane.dispatch_request, Some(5));
    plane.pursue_goal().unwrap();
    assert_eq!(plane.ammunition(), Some(-1));
    let mut out = ByteWriter::new();
    assert!(plane.write(&mut out));
    let mut loaded = Airplane::default();
    assert!(loaded.read(&mut ByteReader::new(out.bytes())));
    loaded.pursue_goal().unwrap();
    assert_eq!(loaded.ammunition(), Some(2));
    assert_eq!(loaded.loiter_remaining_seconds, 2.0);
    assert!(plane.read(&mut ByteReader::new(out.bytes())));
    assert_eq!(plane.ammunition(), Some(-1));
    for remaining in [0.0001, 0.0, -0.0001] {
        let mut plane = loaded.clone();
        plane.loiter_remaining_seconds = remaining;
        assert_eq!(
            plane.pursue_goal(),
            Ok(if remaining > 0.0 {
                AircraftGoalOutcome::Waiting
            } else {
                AircraftGoalOutcome::Popped
            })
        );
        assert_eq!(plane.water_drop_active, remaining > 0.0);
    }
    loaded.loiter_remaining_seconds = 0.125;
    assert_eq!(loaded.tick_loiter(125.0), Ok(AircraftGoalOutcome::Popped));
    let mut capped = Airplane::default();
    capped.begin_loiter();
    capped.tick_loiter(10000.0).unwrap();
    assert_eq!(capped.loiter_remaining_seconds, 2.0); // first visit follows tick advance
    capped.tick_loiter(10000.0).unwrap();
    near(capped.loiter_remaining_seconds, 1.8, 0.000001);
}

#[test]
fn vectors21_22_distinct_phase_actions_and_repeat_rules() {
    use SoundRequest::{End, Start};
    let stopped = vec![End(9), End(8), End(10), Start(10)];
    let mut plane = Airplane::default();
    plane.player_driven = true;
    plane.set_airplane_state(FlightPhase::Phase0);
    assert_eq!(
        plane.set_airplane_state(FlightPhase::Phase1).unwrap().sounds,
        vec![Start(9)]
    );
    plane.set_ammunition(1);
    assert!(plane.set_airplane_state(FlightPhase::Phase1).is_none());
    assert_eq!(plane.ammunition(), Some(1));
    assert!(plane.set_airplane_state(FlightPhase::Phase3).unwrap().sounds.is_empty());
    assert_eq!(
        plane.set_airplane_state(FlightPhase::Phase1).unwrap().sounds,
        vec![End(13), Start(13)]
    );
    assert_eq!(plane.set_airplane_state(FlightPhase::Phase0).unwrap().sounds, stopped);
    plane.player_driven = false;
    assert!(plane.set_airplane_state(FlightPhase::Phase1).unwrap().sounds.is_empty());
    assert_eq!(plane.ammunition(), Some(3));
    let mut heli = Helicopter::default();
    heli.player_driven = true;
    heli.linear_velocity = Some([2.0, -3.0, 4.0]);
    heli.angular_velocity = Some([-0.0, 0.0, -0.0]);
    for phase in [
        FlightPhase::Phase4,
        FlightPhase::Phase4,
        FlightPhase::Phase0,
        FlightPhase::Phase0,
    ] {
        let a = heli.set_helicopter_state(phase).unwrap();
        let engine = phase == FlightPhase::Phase4;
        assert_eq!(a.sounds, if engine { vec![Start(9)] } else { stopped.clone() });
        assert_eq!(
            a.engine_notifications,
            vec![
                EngineNotification::Base(engine),
                EngineNotification::Embedded {
                    offset: 0x240,
                    operation: if engine { 0x18 } else { 0x1C },
                    argument: u32::from(!engine)
                },
                EngineNotification::Embedded {
                    offset: 0x22C,
                    operation: if engine { 0x1C } else { 0x18 },
                    argument: u32::from(engine)
                }
            ]
        );
    }
    assert_eq!(heli.linear_velocity.unwrap().map(f32::to_bits), [0, 0x80000000, 0]);
    assert_eq!(
        heli.angular_velocity.unwrap().map(f32::to_bits),
        [0x80000000, 0, 0x80000000]
    );
    for enabled in [false, true] {
        heli.engine_enabled = Some(enabled);
        let a = heli.set_helicopter_state(FlightPhase::Phase3).unwrap();
        assert_eq!(heli.engine_enabled, Some(enabled));
        assert!(a.sounds.is_empty());
    }
}

#[derive(Default)]
struct Weapons {
    calls: Vec<&'static str>,
    successes: Vec<&'static str>,
}
impl AircraftWeaponEffects for Weapons {
    fn translate_base_commands(&mut self, _: &mut PlayerWeaponRecord) {
        self.calls.push("base");
    }
    fn preliminary(&mut self) {
        self.calls.push("preliminary");
    }
    fn attempt(&mut self, name: &'static str, a: u32, b: u32) -> bool {
        assert_eq!((a, b), (0, 0));
        self.calls.push(name);
        self.successes.contains(&name)
    }
}
#[test]
fn vector23_weapons_fallback_readiness_and_wrapping_use_count() {
    for success in [Some("helicoptermissle"), Some("ufolaser"), None] {
        let mut player = PlayerWeaponRecord {
            command: 0.0,
            ready: true,
            repeat_timer: 7.0,
            uses: u32::MAX,
        };
        let before = player;
        let mut effects = Weapons {
            successes: success.into_iter().collect(),
            ..Weapons::default()
        };
        let result = Helicopter::default().weapon_requests(&mut player, &mut effects);
        assert_eq!(&effects.calls[..3], &["base", "preliminary", "helicoptermissle"]);
        assert_eq!(
            effects.calls.len(),
            if success == Some("helicoptermissle") { 3 } else { 4 }
        );
        if let Some(name) = success {
            assert_eq!(result, WeaponOutcome::Used(name));
            assert!(!player.ready);
            assert_eq!(player.repeat_timer, 0.0);
            assert_eq!(player.uses, 0);
        } else {
            assert_eq!(result, WeaponOutcome::Failed);
            assert_eq!(player, before);
        }
    }
    for (command, ready, expected) in [(0.0, false, vec!["base", "preliminary"]), (-1.0, true, vec!["base"])] {
        let mut p = PlayerWeaponRecord {
            command,
            ready,
            repeat_timer: 9.0,
            uses: 2,
        };
        let mut effects = Weapons::default();
        Helicopter::default().weapon_requests(&mut p, &mut effects);
        assert_eq!(effects.calls, expected);
    }
    let mut plane = Airplane::default();
    let mut effects = Weapons {
        successes: vec!["jetbomb"],
        ..Weapons::default()
    };
    let mut p = PlayerWeaponRecord {
        command: 0.0,
        ready: true,
        repeat_timer: 9.0,
        uses: 2,
    };
    assert_eq!(
        plane.weapon_requests(&mut p, &mut effects),
        WeaponOutcome::Used("jetbomb")
    );
    assert_eq!(plane.ammunition(), Some(3));
}

struct World {
    accepted: bool,
    calls: Vec<(u32, f32, f32)>,
}
impl AircraftWorld for World {
    fn bounds(&self) -> AircraftBounds {
        AircraftBounds {
            min_x: 0.0,
            min_z: 0.0,
            max_x: 1000.0,
            max_z: 1000.0,
        }
    }
    fn terrain_height(&self, x: f32, _: f32) -> f32 {
        if x == 10.0 {
            30.0
        } else {
            40.0
        }
    }
    fn water_height(&self, _: f32, _: f32) -> Option<f32> {
        None
    }
    fn building_height(&self, _: f32, _: f32) -> Option<f32> {
        None
    }
    fn prop_height(&self, _: f32, _: f32) -> Option<f32> {
        None
    }
}
impl ClearanceWorld for World {
    fn acquire_auxiliary_height(&mut self) {
        self.calls.push((0, 0.0, 0.0));
    }
    fn accepts_terrain_point(&mut self, x: f32, z: f32) -> bool {
        self.calls.push((2, x, z));
        self.accepted
    }
    fn auxiliary_height(&mut self, mode: u32, x: f32, z: f32) -> f32 {
        self.calls.push((mode, x, z));
        if x == 10.0 {
            35.0
        } else {
            60.0
        }
    }
}
#[test]
fn vector24_two_samples_home_floor_and_descending_demand() {
    for (y, expected) in [(100.0, 60.0), (130.0, 30.0), (180.0, 30.0), (70.0, 80.0)] {
        let mut world = World {
            accepted: true,
            calls: vec![],
        };
        let mut heli = Helicopter::default();
        heli.controls = Some(controls());
        heli.base_mut().vectors[0] = [10.0, y, 20.0];
        heli.record.home_position[1] = 50.0;
        let mut expected_controls = controls();
        expected_controls.hover(0.1, y, 0.0, expected);
        assert_eq!(
            heli.ai_control_lift_over_buildings(&mut world, 0.1, [10.0, 0.0, 20.0]),
            Ok(expected)
        );
        assert_eq!(heli.controls.unwrap(), expected_controls);
        assert_eq!(
            world.calls,
            vec![
                (0, 0.0, 0.0),
                (2, 10.0, 20.0),
                (1, 10.0, 20.0),
                (2, 13.0, 26.0),
                (1, 13.0, 26.0)
            ]
        );
        assert_eq!(heli.base().vectors[0], [10.0, y, 20.0]);
    }
    let mut world = World {
        accepted: false,
        calls: vec![],
    };
    assert_eq!(
        building_clearance_height(&mut world, [10.0, 70.0, 20.0], [10.0, 0.0, 20.0], 50.0),
        70.0
    );
    assert_eq!(world.calls.len(), 3);
}

#[test]
fn vector25_allocation_cleanup_order_false_propagation_and_fault_diagnostic() {
    let mut heli = Helicopter::default();
    let mut allocator = Allocator::default();
    heli.init_instance_with(&mut allocator, None, None).unwrap();
    let mut resources = vec![RuntimeResource::State, RuntimeResource::Parameters];
    resources.extend(TRANSFORM_OFFSETS.map(RuntimeResource::Transform));
    assert_eq!(allocator.calls, resources);
    assert_eq!(heli.runtime.as_ref().unwrap().completion_word, Some(0));
    heli.init_instance_with(&mut allocator, None, None).unwrap();
    assert_eq!(allocator.calls, resources);
    heli.retained_effects_present = [true; 4];
    heli.controls = Some(controls());
    let result = heli.shutdown_after_base(false);
    assert!(!result.base_result);
    let mut expected = vec![];
    for offset in [0x200, 0x204, 0x224, 0x228] {
        expected.push(CleanupAction::StopEffect { offset, argument: 1 });
        expected.push(CleanupAction::ReleaseRetainedEffect(offset));
    }
    for offset in [0x22C, 0x240] {
        expected.push(CleanupAction::Embedded {
            offset,
            operation: 0x1C,
            argument: Some(1),
        });
        expected.push(CleanupAction::Embedded {
            offset,
            operation: 0x04,
            argument: None,
        });
    }
    expected.extend(TRANSFORM_OFFSETS.map(|o| CleanupAction::Free(RuntimeResource::Transform(o))));
    expected.extend([
        CleanupAction::Free(RuntimeResource::Parameters),
        CleanupAction::Free(RuntimeResource::State),
        CleanupAction::Free(RuntimeResource::Controls),
    ]);
    assert_eq!(result.actions, expected);
    assert!(heli.runtime.is_none());
    assert!(heli.controls.is_none());
    assert_eq!(heli.retained_effects_present, [false; 4]);
    heli.controls = Some(controls());
    assert_eq!(
        heli.shutdown_after_base(true).actions.last(),
        Some(&CleanupAction::Free(RuntimeResource::Controls))
    );
    let mut failed = Helicopter::default();
    let resource = RuntimeResource::Transform(0xA8);
    let mut allocator = Allocator {
        fail: Some(resource),
        ..Allocator::default()
    };
    assert_eq!(
        failed.init_instance_with(&mut allocator, None, None),
        Err(AllocationFault(resource))
    );
    assert!(failed.runtime.as_ref().unwrap().completion_word.is_none());
    assert!(failed.physics_init().is_err()); // partial allocation is not a usable rotor
}

#[test]
fn vector26_takeover_direct_precedence_zero_fallback_and_gain_refresh() {
    let mut heli = Helicopter::default();
    heli.init_instance().unwrap();
    heli.input_gains = [Some(10.0), Some(20.0), Some(30.0), Some(4.0)];
    let mut source = PlayerSource {
        direct: Some(PlayerTakeoverRecord {
            speed: 0.0,
            gains: [0.0; 4],
        }),
        script: Some(PlayerTakeoverRecord {
            speed: 1.0,
            gains: [9.0; 4],
        }),
    };
    heli.on_player_take_over(Some(&source)).unwrap();
    assert_eq!(heli.drive_parameters.get(DriveField::MaxSpeed), Some(400.0));
    assert_eq!(heli.input_gains, [Some(10.0), Some(20.0), Some(30.0), Some(4.0)]);
    source.direct = Some(PlayerTakeoverRecord {
        speed: 0.03,
        gains: [5.0, 0.0, -2.0, 3.0],
    });
    heli.on_player_take_over(Some(&source)).unwrap();
    near(heli.drive_parameters.get(DriveField::MaxSpeed).unwrap(), 100.0, 0.00002);
    assert_eq!(heli.input_gains, [Some(5.0), Some(20.0), Some(-2.0), Some(3.0)]);
    let before = heli.drive_parameters.clone();
    heli.on_player_take_over(None).unwrap();
    assert_eq!(heli.drive_parameters, before);
    source.direct = Some(PlayerTakeoverRecord {
        speed: -0.0,
        gains: [-0.0; 4],
    });
    heli.on_player_take_over(Some(&source)).unwrap();
    assert_eq!(heli.input_gains, [Some(5.0), Some(20.0), Some(-2.0), Some(3.0)]);
    source.direct = None;
    heli.on_player_take_over(Some(&source)).unwrap();
    assert_eq!(heli.input_gains, [Some(9.0); 4]);
}

struct Lights {
    present: [bool; 2],
    active: [bool; 2],
    environment: u32,
    calls: Vec<(u32, usize, u32)>,
}
impl SpotlightEffects for Lights {
    fn present(&self, i: usize) -> bool {
        self.present[i]
    }
    fn environment_mode(&mut self) -> u32 {
        self.calls.push((0x58, 0, 0));
        self.environment
    }
    fn active(&mut self, i: usize) -> bool {
        self.calls.push((0x14, i, 0));
        self.active[i]
    }
    fn start(&mut self, i: usize, a: u32) {
        self.calls.push((0x0C, i, a));
        self.active[i] = true;
    }
    fn stop(&mut self, i: usize, a: u32) {
        self.calls.push((0x10, i, a));
        self.active[i] = false;
    }
}
#[test]
fn vector27_spotlight_pair_gate_idempotence_and_geometry_continuation() {
    let mut heli = Helicopter::default();
    heli.set_helicopter_state(FlightPhase::Phase4);
    heli.spotlight_enabled = Some(true);
    let mut lights = Lights {
        present: [true; 2],
        active: [false, true],
        environment: 1,
        calls: vec![],
    };
    for present in [[false, true], [true, false]] {
        lights.present = present;
        assert_eq!(
            heli.spotlight_activation_stage(&mut lights),
            Ok(SpotlightStage::MissingPair)
        );
        assert!(lights.calls.is_empty());
    }
    lights.present = [true; 2];
    for (environment, enabled, phase, expected_active, changes) in [
        (1, true, FlightPhase::Phase4, true, vec![(0x0C, 0, 0)]),
        (1, true, FlightPhase::Phase4, true, vec![]),
        (2, true, FlightPhase::Phase4, false, vec![(0x10, 0, 1), (0x10, 1, 1)]),
        (2, true, FlightPhase::Phase4, false, vec![]),
        (1, false, FlightPhase::Phase4, false, vec![]),
        (1, true, FlightPhase::Phase0, false, vec![]),
    ] {
        lights.environment = environment;
        heli.spotlight_enabled = Some(enabled);
        heli.set_helicopter_state(phase);
        lights.calls.clear();
        assert_eq!(
            heli.spotlight_activation_stage(&mut lights),
            Ok(SpotlightStage::NeedsGeometry {
                player_drive_state_present: false
            })
        );
        assert_eq!(lights.active, [expected_active; 2]);
        assert_eq!(
            lights
                .calls
                .iter()
                .copied()
                .filter(|c| matches!(c.0, 0x0C | 0x10))
                .collect::<Vec<_>>(),
            changes
        );
    }
}
