// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 vectors 28–34 and their independently supplied worked cases.
use fc4_core::automata::airplane::*;
use fc4_core::automata::{
    AircraftBounds, AircraftWorld, Airplane, FlightPhase, Helicopter, SetStateArgs, SoundRequest,
};
use fc4_core::helicopter::{
    DispatchAction, HelicopterGoalAction, RotorControls, RotorFrameStages, RotorFrameState, RotorPower,
};
use std::cell::RefCell;

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn vector28_nonfinite_limiter_force_direction_and_time() {
    let unchanged = limit_velocity([f32::NAN, 2.0, 3.0], 150.0);
    assert!(unchanged[0].is_nan());
    assert_eq!(unchanged[1..], [2.0, 3.0]);
    let infinite = limit_velocity([f32::INFINITY, 2.0, 0.0], 150.0);
    assert!(infinite[0].is_nan());
    assert_eq!(infinite[1].to_bits(), 0);
    assert_eq!(infinite[2].to_bits(), 0);
    assert_eq!(
        limit_velocity([f32::INFINITY, 2.0, 0.0], f32::INFINITY),
        [f32::INFINITY, 2.0, 0.0]
    );
    assert!(limit_velocity([f32::INFINITY, 2.0, 0.0], f32::NEG_INFINITY)
        .iter()
        .all(|v| v.is_nan()));
    let negative = limit_velocity([2.0, -3.0, 0.0], f32::NEG_INFINITY);
    assert_eq!(negative[..2], [f32::NEG_INFINITY, f32::INFINITY]);
    assert!(negative[2].is_nan());
    assert_eq!(limit_velocity([2.0, 3.0, 0.0], f32::NAN), [2.0, 3.0, 0.0]);
    near(limit_velocity([0.00001, 0.0, 0.0], f32::NAN)[0], 1.0);
    assert_eq!(physics_seconds(f32::INFINITY, None), 0.2);
    assert_eq!(physics_seconds(f32::INFINITY, Some(-1.0)), f32::NEG_INFINITY);
    assert!(physics_seconds(0.0, Some(f32::INFINITY)).is_nan());
    assert!(physics_seconds(f32::NAN, None).is_nan());
    let parameters = AirplaneDriveParameters::default();
    let scalars = parameters.force_scalars([f32::NAN, 2.0, 3.0], [1.0, 0.0, 0.0], 0.0, 0.0, 0.0, 2);
    assert!(scalars.direction.iter().all(|v| v.is_nan()));
    let scalars = parameters.force_scalars([0.0; 3], [0.0; 3], 0.0, 0.0, 0.0, 2);
    assert!(scalars.direction.iter().all(|v| v.is_nan()));
}

struct Terrain {
    accepted: bool,
    calls: RefCell<Vec<&'static str>>,
}
impl AircraftWorld for Terrain {
    fn bounds(&self) -> AircraftBounds {
        AircraftBounds {
            min_x: 0.0,
            min_z: 0.0,
            max_x: 1024.0,
            max_z: 1024.0,
        }
    }
    fn terrain_height(&self, x: f32, _: f32) -> f32 {
        self.calls.borrow_mut().push("altitude");
        if x == 32.0 {
            100.0
        } else {
            10.0
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
impl RunwayTerrain for Terrain {
    fn accepts_point(&mut self, _: f32, _: f32) -> bool {
        self.calls.borrow_mut().push("region");
        self.accepted
    }
    fn auxiliary_height(&mut self, mode: u32, x: f32, _: f32) -> f32 {
        assert_eq!(mode, 1);
        self.calls.borrow_mut().push("auxiliary");
        if x == 32.0 {
            26.0
        } else {
            10.0
        }
    }
}
#[test]
fn vector29_clearance_samples_region_before_altitude_and_replaces_height() {
    let mut world = Terrain {
        accepted: true,
        calls: RefCell::default(),
    };
    assert_eq!(evaluate_clearance(&mut world, [0.0, 10.0, 0.0], [1.0, 0.0, 0.0]), 0.5);
    assert_eq!(*world.calls.borrow(), ["region", "altitude", "auxiliary"].repeat(20));
    world.accepted = false;
    world.calls.borrow_mut().clear();
    assert_eq!(
        evaluate_clearance(&mut world, [0.0, 10.0, 0.0], [1.0, 0.0, 0.0]),
        2.8125
    );
    assert_eq!(*world.calls.borrow(), ["region", "altitude"].repeat(20));
}
struct Random {
    index: usize,
    weight: f64,
    calls: Vec<&'static str>,
}
impl RunwayRandom for Random {
    fn uniform_index(&mut self, count: usize) -> usize {
        assert!(self.index < count);
        self.calls.push("index");
        self.index
    }
    fn uniform_weight(&mut self, total: f64) -> f64 {
        assert_eq!(total, 4.0);
        self.calls.push("weight");
        self.weight
    }
}
#[test]
fn vector30_weighted_boundary_uniform_fallback_and_direction_flag() {
    let mut random = Random {
        index: 1,
        weight: 1.0,
        calls: vec![],
    };
    assert_eq!(weighted_endpoint(&[1.0, 3.0], &mut random), 1);
    assert_eq!(weighted_endpoint(&[0.0, 0.0], &mut random), 1);
    assert_eq!(random.calls, ["weight", "index"]);
    random.weight = 4.0;
    assert_eq!(weighted_endpoint(&[1.0, 3.0], &mut random), 1);
    let mut world = Terrain {
        accepted: true,
        calls: RefCell::default(),
    };
    let endpoints = [[0.0, 10.0, 0.0], [0.0, 10.0, 50.0]];
    for flag in [true, false] {
        let RunwayResult::Selected(selected) = pick_runway(&mut world, &endpoints, flag, [1.0, 1.0], &mut random)
        else {
            panic!()
        };
        assert_eq!(selected.start, endpoints[usize::from(flag)]);
        assert_eq!(selected.other_endpoint, endpoints[usize::from(!flag)]);
        assert_eq!(selected.direction, [0.0, 0.0, if flag { -1.0 } else { 1.0 }]);
    }
    world.calls.borrow_mut().clear();
    assert_eq!(
        pick_runway(&mut world, &endpoints[..1], true, [0.0; 2], &mut random),
        RunwayResult::RemoveOwner(0)
    );
    assert!(world.calls.borrow().is_empty());
}
#[test]
fn approach_worked_case_minimum_precedes_city_cap_and_uses_measured_legs() {
    let bounds = AircraftBounds {
        min_x: 0.0,
        min_z: 0.0,
        max_x: 1024.0,
        max_z: 1024.0,
    };
    let points = approach_box([0.0; 3], [1.0, 0.0, 0.0], 0.1, [10.0, 20.0], bounds);
    assert_eq!(points[0], [0.0; 3]);
    assert_eq!(points[1], [-500.0, 50.0, 0.0]);
    near(
        points[2][1],
        (500.0 + (500.0_f32.powi(2) + 50.0_f32.powi(2)).sqrt()) * 0.1,
    );
    let small = AircraftBounds {
        max_x: 100.0,
        max_z: 100.0,
        ..bounds
    };
    assert_eq!(
        approach_box([0.0; 3], [1.0, 0.0, 0.0], 0.1, [20.0; 2], small)[1],
        [-500.0, 50.0, 0.0]
    );
}
#[test]
fn vector33_airplane_constructor_and_teardown_precede_false_base() {
    let mut plane = Airplane::default();
    assert_eq!(plane.phase(), Some(FlightPhase::Phase0));
    assert_eq!(plane.ammunition(), Some(3));
    assert_eq!(plane.loiter_remaining_seconds, 0.0);
    assert_eq!(plane.velocity, [0.0; 3]);
    assert_eq!(plane.angular_fields, [0.0; 3]);
    assert_eq!(plane.fire_search_anchor, [-1.0, -2.0, -3.0]);
    plane.retained_effects_present = [true; 4];
    plane.fire_filter_present = true;
    let calls = RefCell::new(vec![]);
    assert!(!plane.shutdown(
        |action| calls.borrow_mut().push(Some(action)),
        || {
            calls.borrow_mut().push(None);
            false
        }
    ));
    let mut expected = vec![Some(AirplaneCleanup::ReleaseFilter)];
    for offset in [0x20C, 0x208, 0x214, 0x210] {
        expected.extend([
            Some(AirplaneCleanup::StopEffect(offset, 1)),
            Some(AirplaneCleanup::ClearEffect(offset)),
            Some(AirplaneCleanup::ReleaseEffect(offset)),
        ]);
    }
    expected.push(None);
    assert_eq!(*calls.borrow(), expected);
    assert_eq!(plane.retained_effects_present, [false; 4]);
    assert!(!plane.fire_filter_present);
    assert!(plane.shutdown(|_| panic!("already cleared"), || true));
}

struct Fires {
    count: Option<usize>,
    calls: Vec<&'static str>,
    query: Option<FireQuery>,
}
impl FireSearch for Fires {
    fn query(&mut self, query: FireQuery) -> Option<usize> {
        self.calls.push("query");
        self.query = Some(query);
        self.count
    }
    fn first_position(&mut self) -> [f32; 3] {
        self.calls.push("first");
        [99.0, 0.0, 99.0]
    }
    fn release_list(&mut self) {
        self.calls.push("release-list");
    }
    fn release_service(&mut self) {
        self.calls.push("release-service");
    }
}
#[test]
fn fire_search_uses_box_first_occupant_and_balances_query_ownership() {
    let mut plane = Airplane::default();
    plane.fire_search_anchor = [0.0; 3];
    plane.fire_filter_present = true;
    for count in [Some(2), Some(0), None] {
        plane.base_mut().vectors[2] = [7.0; 3];
        let mut fires = Fires {
            count,
            calls: vec![],
            query: None,
        };
        assert_eq!(plane.search_fire(&mut fires), count == Some(2));
        assert_eq!(
            fires.query.unwrap(),
            FireQuery {
                minimum: [-100.0, -f32::MAX, -100.0],
                maximum: [100.0, f32::MAX, 100.0],
                filter_present: true,
                argument: -1,
            }
        );
        assert_eq!(
            plane.base().vectors[2],
            if count == Some(2) { [99.0, 0.0, 99.0] } else { [7.0; 3] }
        );
        assert_eq!(
            fires.calls,
            match count {
                Some(2) => vec!["query", "first", "release-list", "release-service"],
                Some(0) => vec!["query", "release-list", "release-service"],
                _ => vec!["query", "release-service"],
            }
        );
    }
}

fn plane_goal(goal: u32) -> Airplane {
    let mut plane = Airplane::default();
    plane.base_mut().push_state(goal, SetStateArgs::default());
    plane
}
#[test]
fn vector31_fire_land_and_default_dispatch_order() {
    use AirplaneGoalAction::*;
    let mut plane = plane_goal(0x39);
    plane.set_ammunition(1);
    assert_eq!(
        plane.goal_actions(AirplaneGoalInputs::default()),
        [SearchFire, Pop, Dispatch(3), Push(0x32), Push(0x35), Push(0x33)]
    );
    assert_eq!(
        plane.goal_actions(AirplaneGoalInputs {
            fire_found: true,
            ..Default::default()
        }),
        [SearchFire, Dispatch(3), Push(0x35), Push(0x33)]
    );
    plane.set_ammunition(0);
    for found in [false, true] {
        assert_eq!(
            plane.goal_actions(AirplaneGoalInputs {
                fire_found: found,
                ..Default::default()
            }),
            [SearchFire, Dispatch(4), Push(0x32)]
        );
    }
    let mut plane = plane_goal(0x32);
    let landed = AirplaneGoalInputs {
        land_succeeded: true,
        ..Default::default()
    };
    for state in [0, 4] {
        plane.dispatch_state = state;
        assert_eq!(plane.goal_actions(landed), [Land, Dispatch(0), Pop]);
    }
    plane.dispatch_state = 3;
    assert_eq!(plane.goal_actions(landed), [Dispatch(6), Land, Dispatch(0), Pop]);
    let mut plane = plane_goal(0xFE);
    plane.set_airplane_state(FlightPhase::Phase3);
    assert_eq!(plane.goal_actions(Default::default()), [Pop, Push(0x32)]);
    plane.set_airplane_state(FlightPhase::Phase1);
    assert_eq!(plane.goal_actions(Default::default()), [Pop, Stop]);
    let plane = plane_goal(0x33);
    assert_eq!(
        plane.goal_actions(AirplaneGoalInputs {
            destination_sentinel: true,
            ..Default::default()
        }),
        [ClipDestination, Pop]
    );
}

fn rotor(engine_flag: u32) -> RotorPower {
    RotorPower {
        engine_flag,
        rotor_speed: 0.0,
        collective: 0.0,
        power: 240.0,
        vertical_velocity: 0.0,
        mass: 1.5,
        pitch: 0.0,
        horizontal_speed: 0.0,
        collective_fraction: -99.0,
        vertical_force: -99.0,
    }
}
#[test]
fn vector32_rotor_power_flags_corrections_and_clamps() {
    for flag in [1, 2] {
        let mut state = rotor(flag);
        state.update(1.0);
        assert_eq!(state.rotor_speed, 100.0);
        near(state.collective_fraction, 0.5);
        near(state.vertical_force, if flag == 1 { 30.0 } else { 0.0 });
    }
    let mut state = rotor(0);
    state.rotor_speed = 450.0;
    state.update(0.0);
    assert_eq!(state.rotor_speed, 400.0);
    state.update(10.0);
    assert_eq!(state.rotor_speed, 0.0);
    state.engine_flag = 1;
    state.rotor_speed = 450.0;
    state.update(1.0);
    assert_eq!(state.rotor_speed, 450.0); // Powered E>=400 is left unchanged.
    for (flag, vy, pitch, speed, collective, expected) in [
        (1, 2.0, 0.0, 0.0, 0.0, 120.0 - 4.0 * 0.018 * 4.0 * 1.5),
        (1, -2.0, 0.0, 0.0, 0.0, 120.0 + 4.0 * 0.018 * 1.5),
        (2, 2.0, -1.0, 11.0, 200.0, 4.0 * 0.02352 * 1.5 * 1.5 * 4.0),
        (0, 2.0, -1.0, 10.0, 200.0, 0.0),
        (0, 2.0, 0.0, 11.0, -200.0, 4.0 * 0.02352 * 1.5),
    ] {
        let mut state = RotorPower {
            engine_flag: flag,
            rotor_speed: 400.0,
            vertical_velocity: vy,
            pitch,
            horizontal_speed: speed,
            collective,
            ..rotor(flag)
        };
        state.update(0.0);
        near(state.vertical_force, expected);
        if flag == 2 {
            near(state.collective_fraction, 1.5);
        }
    }
}
#[derive(Default)]
struct Frame {
    calls: Vec<(&'static str, f32)>,
}
impl RotorFrameStages for Frame {
    fn engine_power(&mut self, h: f32) {
        self.calls.push(("power", h));
    }
    fn attitude_slide(&mut self, h: f32) {
        self.calls.push(("attitude", h));
    }
    fn horizontal_motion(&mut self, h: f32) {
        self.calls.push(("horizontal", h));
    }
    fn vertical_motion(&mut self, h: f32) {
        self.calls.push(("vertical", h));
    }
    fn transform_xz(&self) -> [f32; 2] {
        assert_eq!(self.calls.len(), 4);
        [3.0, 7.0]
    }
}
#[test]
fn vector32_frame_order_phase1_clock_and_phase6_bypass() {
    for phase in [0, 1, 6] {
        let mut state = RotorFrameState {
            phase,
            copied_xz: [0.0; 2],
            applied_xz: [0.0; 2],
            phase1_seconds: 2.0,
        };
        let mut frame = Frame::default();
        state.next_frame(0.1, &mut frame);
        if phase == 6 {
            assert!(frame.calls.is_empty());
            assert_eq!(state.applied_xz, [0.0; 2]);
        } else {
            assert_eq!(
                frame.calls,
                [
                    ("power", 0.1),
                    ("attitude", 0.1),
                    ("horizontal", 0.1),
                    ("vertical", 0.1)
                ]
            );
            assert_eq!(state.copied_xz, [3.0, 7.0]);
            assert_eq!(state.applied_xz, [3.0, 7.0]);
        }
        assert_eq!(state.phase1_seconds, if phase == 1 { 2.1 } else { 2.0 });
    }
}
fn heli() -> Helicopter {
    let mut heli = Helicopter::default();
    heli.init_instance().unwrap();
    heli.physics_init().unwrap();
    heli.controls = Some(RotorControls {
        lateral: 30.0,
        longitudinal: -30.0,
        collective: 0.0,
        yaw: 30.0,
    });
    heli
}
#[test]
fn vector32_home_completion_blend_snap_reset_and_skip() {
    let mut heli = heli();
    heli.base_mut().push_state(0x32, SetStateArgs::default());
    heli.base_mut().vectors[0] = [30.0, 0.0, 0.0];
    heli.landing_latch = Some(true);
    assert!(heli
        .physics_home_stage(0.3, 2.0, |_, _| panic!("rotor/collision must be skipped"))
        .unwrap()
        .is_none());
    near(heli.base().vectors[0][0], 27.0);
    near(heli.scaled_rotor_position.unwrap()[0], 90.0);
    assert_eq!(heli.controls.unwrap().collective, -30.000002);
    heli.base_mut().vectors[0] = [0.5, 0.0, 0.0];
    let snapped = heli
        .physics_home_stage(0.3, 2.0, |_, _| panic!("must skip"))
        .unwrap()
        .unwrap();
    assert_eq!(snapped.current, FlightPhase::Phase0);
    assert_eq!(heli.base().vectors[0], [0.0; 3]);
    assert_eq!(heli.scaled_rotor_position, Some([0.0; 3]));
    assert_eq!(heli.landing_latch, Some(false));
    let called = RefCell::new(false);
    heli.physics_home_stage(0.1, 1.0, |_, h| {
        assert_eq!(h, 0.1);
        *called.borrow_mut() = true;
        Ok(None)
    })
    .unwrap();
    assert!(*called.borrow());
    heli.landing_latch = Some(true);
    heli.base_mut().push_state(0x35, SetStateArgs::default());
    heli.physics_home_stage(0.1, 1.0, |_, _| panic!("other goal clears flag but skips this visit"))
        .unwrap();
    assert_eq!(heli.landing_latch, Some(false));
}
#[test]
fn startup_phase_gate_and_ordered_dispatch_repeat_rules() {
    let mut heli = heli();
    heli.player_driven = true;
    let start = heli.start_stage(0.0, 0.0).unwrap();
    assert_eq!(start.sounds, [SoundRequest::Start(9)]);
    assert_eq!(start.notifications.len(), 3);
    assert_eq!(heli.phase(), Some(FlightPhase::Phase4));
    assert!(heli.start_stage(400.0, 0.0).unwrap().notifications.is_empty());
    assert_eq!(heli.phase(), Some(FlightPhase::Phase4));
    assert!(heli.start_stage(f32::NAN, 1.0).unwrap().notifications.is_empty());
    let flight = heli.start_stage(400.0, 1.0).unwrap();
    assert!(flight.sounds.is_empty());
    assert_eq!(flight.notifications, start.notifications);
    assert_eq!(heli.phase(), Some(FlightPhase::Phase1));
    heli.set_helicopter_state(FlightPhase::Phase3);
    heli.engine_enabled = Some(false);
    assert!(heli.start_stage(400.0, 1.0).unwrap().notifications.is_empty());
    assert_eq!(heli.engine_enabled, Some(false));
    heli.base_mut().push_state(0x36, SetStateArgs::default());
    heli.dispatch_state = Some(3);
    let mut actions = vec![];
    heli.set_dispatch_state(5, true, |a| actions.push(a));
    assert_eq!(actions, [DispatchAction::AddRiotRepulsor, DispatchAction::Notify(7, 5)]);
    heli.set_dispatch_state(5, true, |_| panic!("repeat"));
    actions.clear();
    heli.set_dispatch_state(2, true, |a| actions.push(a));
    assert_eq!(
        actions,
        [DispatchAction::RemoveRiotRepulsor, DispatchAction::Notify(7, 2)]
    );
}
#[test]
fn vector31_helicopter_land_home_and_loiter_actions() {
    use HelicopterGoalAction::*;
    let mut heli = heli();
    heli.base_mut().push_state(0x32, SetStateArgs::default());
    assert_eq!(
        heli.goal_actions(false, false).unwrap(),
        [Dispatch(0), Pop, Reset, Spotlight(false)]
    );
    heli.base_mut().states.clear();
    heli.set_helicopter_state(FlightPhase::Phase3);
    heli.base_mut().vectors[0] = [8.0, 30.0, 0.0];
    assert_eq!(
        heli.goal_actions(false, false).unwrap(),
        [Start, Spotlight(false), StopHorizontal, Hover(6.0)]
    );
    heli.base_mut().vectors[0] = [48.0, 0.0, 0.0];
    assert!(heli.goal_actions(false, false).unwrap().contains(&Hover(46.0)));
    heli.base_mut().vectors[0] = [0.0, 23.0, 0.0];
    assert!(heli.goal_actions(false, false).unwrap().contains(&HomeComplete));
    heli.base_mut().push_state(0x36, SetStateArgs::default());
    heli.base_mut().push_state(0x35, SetStateArgs::default());
    assert_eq!(
        heli.goal_actions(false, false).unwrap(),
        [
            Dispatch(5),
            Start,
            Spotlight(true),
            AimGround([0.0; 3]),
            RespondToCrimeScene,
            StopHorizontal,
            BuildingLift
        ]
    );
}
#[test]
fn vector33_duplicate_claim_removal_precedes_base_and_cleanup() {
    let mut heli = heli();
    heli.base_mut().vectors[2] = [10.0, 20.0, 30.0];
    let mut claims = vec![[10.0, 20.0, 30.0], [11.0, 20.0, 30.0], [10.0, 20.0, 30.0]];
    let result = heli.shutdown_with_claims(&mut claims, |owner, remaining| {
        assert_eq!(*remaining, [[11.0, 20.0, 30.0]]);
        assert_eq!(owner.base().vectors[2], [-1.0, -2.0, -3.0]);
        assert!(owner.runtime.is_some());
        false
    });
    assert!(!result.base_result);
    assert!(heli.runtime.is_none());
    let epsilon = f32::from_bits(0x38D1B717);
    heli.base_mut().vectors[2] = [0.0; 3];
    claims = vec![[0.0; 3], [epsilon, 0.0, 0.0], [f32::NAN, 0.0, 0.0]];
    heli.shutdown_with_claims(&mut claims, |_, _| true);
    assert_eq!(claims.len(), 2);
    assert_eq!(claims[0][0], epsilon);
    assert!(claims[1][0].is_nan());
    claims.push([-1.0, -2.0, -3.0]);
    heli.shutdown_with_claims(&mut claims, |_, remaining| {
        assert_eq!(remaining.len(), 3);
        true
    });
}

struct PlayerSource {
    direct: Option<AirplanePlayerRecord>,
    script: Option<AirplanePlayerRecord>,
}
impl AirplanePlayerSource for PlayerSource {
    fn direct_record(&self) -> Option<&AirplanePlayerRecord> {
        self.direct.as_ref()
    }
    fn script_record(&self) -> Option<&AirplanePlayerRecord> {
        self.script.as_ref()
    }
}
#[test]
fn vector34_airplane_refresh_preserves_zero_and_callbacks_reload_ai() {
    let mut plane = Airplane::default();
    plane.player_gains[0] = 2.0;
    let source = PlayerSource {
        direct: Some(AirplanePlayerRecord {
            speed: 0.2,
            gains: [0.0; 12],
        }),
        script: Some(AirplanePlayerRecord {
            speed: 9.0,
            gains: [99.0; 12],
        }),
    };
    plane.refresh_player_parameters(Some(&source));
    near(plane.drive_parameters.0[3], 200.0);
    assert_eq!(plane.player_gains[0], 2.0);
    plane.refresh_player_parameters(None);
    near(plane.drive_parameters.0[3], 200.0);
    plane.control_fields = [9.0; 4];
    plane.water_drop_active = true;
    assert!(!plane.exit_state(Some(0x10), None, || false));
    assert_eq!(plane.drive_parameters.0[3], 150.0);
    assert_eq!(plane.control_fields, [0.0; 4]);
    assert!(!plane.water_drop_active);
    plane.set_airplane_state(FlightPhase::Phase3);
    let callback = plane.enter_state(Some(0x10), Some(&source), || false);
    assert!(!callback.base_result);
    assert_eq!(callback.limits, Some([0.0, 1.0]));
    assert_eq!(callback.sounds, [SoundRequest::Start(8)]);
    plane.base_mut().vectors[2] = [1.0, 2.0, 3.0];
    plane.enter_state(Some(0x39), None, || true);
    assert_eq!(plane.fire_search_anchor, [1.0, 2.0, 3.0]);
    let source = PlayerSource {
        direct: Some(AirplanePlayerRecord::import(|offset| offset as f32)),
        script: None,
    };
    plane.refresh_player_parameters(Some(&source));
    assert_eq!(plane.drive_parameters.0[3], 8000.0);
    for (gain, offset) in plane.player_gains.into_iter().zip(PLAYER_GAIN_RECORD_OFFSETS) {
        near(gain, offset as f32 * 0.01);
    }
}

#[test]
fn calibration_seeds_are_synthetic_and_use_current_player_gains() {
    let mut plane = Airplane::default();
    plane.drive_parameters.0[4] = 0.0;
    plane.player_gains[7] = 2.0;
    plane.player_gains[8] = 3.0;
    let seeds = plane.calibration_inputs();
    assert_eq!(
        seeds.map(|s| s.controls),
        [
            [1.0, 0.0, 0.0, 1.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0, 2.0, 1.0, 0.0],
            [3.0, 1.0, 1.0, 2.0, 1.0, 0.0],
        ]
    );
    for seed in seeds {
        assert_eq!(seed.orientation, [0.0, 0.0, 1.0]);
        assert_eq!(seed.velocity, [0.0, 0.0, 25.0]);
        assert_eq!(seed.seconds, 0.1);
        assert_eq!(seed.maximum_iterations, 100);
    }
}
#[test]
fn helicopter_callbacks_keep_base_result_limits_mass_and_exit_dispatch() {
    use fc4_core::helicopter::{PlayerParameterSource, PlayerTakeoverRecord};
    struct Source(PlayerTakeoverRecord);
    impl PlayerParameterSource for Source {
        fn direct_record(&self) -> Option<&PlayerTakeoverRecord> {
            Some(&self.0)
        }
        fn script_record(&self) -> Option<&PlayerTakeoverRecord> {
            panic!("direct precedence");
        }
    }
    let mut heli = heli();
    let source = Source(PlayerTakeoverRecord {
        speed: 0.0,
        gains: [5.0, 0.0, -2.0, 3.0],
    });
    let callback = heli.enter_state(Some(0x10), Some(&source), || false).unwrap();
    assert!(!callback.base_result);
    assert!(callback.refresh_helper);
    assert_eq!(callback.limits, Some([-100.0, 100.0]));
    assert_eq!(heli.input_gains, [Some(5.0), None, Some(-2.0), Some(3.0)]);
    assert!(!heli.exit_state(Some(0x10), None, false, || false, |_| panic!()));
    assert_eq!(heli.runtime_masses, [Some(1.5); 2]);
    let mut actions = vec![];
    heli.exit_state(Some(0x35), None, false, || true, |a| actions.push(a));
    assert_eq!(actions, [DispatchAction::Notify(7, 2)]);
    heli.exit_state(None, None, true, || true, |_| panic!("null exit"));
}
