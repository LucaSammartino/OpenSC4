// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 vectors 28–34 and their independently supplied worked cases.
use fc4_core::automata::airplane::*;
use fc4_core::automata::{AircraftBounds, AircraftWorld, Airplane, FlightPhase};
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
    assert_eq!(limit_velocity([f32::INFINITY, 2.0, 0.0], f32::INFINITY), [f32::INFINITY, 2.0, 0.0]);
    assert!(limit_velocity([f32::INFINITY, 2.0, 0.0], f32::NEG_INFINITY).iter().all(|v| v.is_nan()));
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
        AircraftBounds { min_x: 0.0, min_z: 0.0, max_x: 1024.0, max_z: 1024.0 }
    }
    fn terrain_height(&self, x: f32, _: f32) -> f32 {
        self.calls.borrow_mut().push("altitude");
        if x == 32.0 { 100.0 } else { 10.0 }
    }
    fn water_height(&self, _: f32, _: f32) -> Option<f32> { None }
    fn building_height(&self, _: f32, _: f32) -> Option<f32> { None }
    fn prop_height(&self, _: f32, _: f32) -> Option<f32> { None }
}
impl RunwayTerrain for Terrain {
    fn accepts_point(&mut self, _: f32, _: f32) -> bool {
        self.calls.borrow_mut().push("region");
        self.accepted
    }
    fn auxiliary_height(&mut self, mode: u32, x: f32, _: f32) -> f32 {
        assert_eq!(mode, 1);
        self.calls.borrow_mut().push("auxiliary");
        if x == 32.0 { 26.0 } else { 10.0 }
    }
}
#[test]
fn vector29_clearance_samples_region_before_altitude_and_replaces_height() {
    let mut world = Terrain { accepted: true, calls: RefCell::default() };
    assert_eq!(evaluate_clearance(&mut world, [0.0, 10.0, 0.0], [1.0, 0.0, 0.0]), 0.5);
    assert_eq!(*world.calls.borrow(), ["region", "altitude", "auxiliary"].repeat(20));
    world.accepted = false;
    world.calls.borrow_mut().clear();
    assert_eq!(evaluate_clearance(&mut world, [0.0, 10.0, 0.0], [1.0, 0.0, 0.0]), 2.8125);
    assert_eq!(*world.calls.borrow(), ["region", "altitude"].repeat(20));
}
struct Random { index: usize, weight: f64, calls: Vec<&'static str> }
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
    let mut random = Random { index: 1, weight: 1.0, calls: vec![] };
    assert_eq!(weighted_endpoint(&[1.0, 3.0], &mut random), 1);
    assert_eq!(weighted_endpoint(&[0.0, 0.0], &mut random), 1);
    assert_eq!(random.calls, ["weight", "index"]);
    random.weight = 4.0;
    assert_eq!(weighted_endpoint(&[1.0, 3.0], &mut random), 1);
    let mut world = Terrain { accepted: true, calls: RefCell::default() };
    let endpoints = [[0.0, 10.0, 0.0], [0.0, 10.0, 50.0]];
    for flag in [true, false] {
        let RunwayResult::Selected(selected) = pick_runway(&mut world, &endpoints, flag, [1.0, 1.0], &mut random) else { panic!() };
        assert_eq!(selected.start, endpoints[usize::from(flag)]);
        assert_eq!(selected.other_endpoint, endpoints[usize::from(!flag)]);
        assert_eq!(selected.direction, [0.0, 0.0, if flag { -1.0 } else { 1.0 }]);
    }
    world.calls.borrow_mut().clear();
    assert_eq!(pick_runway(&mut world, &endpoints[..1], true, [0.0; 2], &mut random), RunwayResult::RemoveOwner(0));
    assert!(world.calls.borrow().is_empty());
}
#[test]
fn approach_worked_case_minimum_precedes_city_cap_and_uses_measured_legs() {
    let bounds = AircraftBounds { min_x: 0.0, min_z: 0.0, max_x: 1024.0, max_z: 1024.0 };
    let points = approach_box([0.0; 3], [1.0, 0.0, 0.0], 0.1, [10.0, 20.0], bounds);
    assert_eq!(points[0], [0.0; 3]);
    assert_eq!(points[1], [-500.0, 50.0, 0.0]);
    near(points[2][1], (500.0 + (500.0_f32.powi(2) + 50.0_f32.powi(2)).sqrt()) * 0.1);
    let small = AircraftBounds { max_x: 100.0, max_z: 100.0, ..bounds };
    assert_eq!(approach_box([0.0; 3], [1.0, 0.0, 0.0], 0.1, [20.0; 2], small)[1], [-500.0, 50.0, 0.0]);
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
    assert!(!plane.shutdown(|action| calls.borrow_mut().push(Some(action)), || { calls.borrow_mut().push(None); false }));
    let mut expected = vec![Some(AirplaneCleanup::ReleaseFilter)];
    for offset in [0x20C, 0x208, 0x214, 0x210] {
        expected.extend([Some(AirplaneCleanup::StopEffect(offset, 1)), Some(AirplaneCleanup::ClearEffect(offset)), Some(AirplaneCleanup::ReleaseEffect(offset))]);
    }
    expected.push(None);
    assert_eq!(*calls.borrow(), expected);
    assert_eq!(plane.retained_effects_present, [false; 4]);
    assert!(!plane.fire_filter_present);
    assert!(plane.shutdown(|_| panic!("already cleared"), || true));
}
