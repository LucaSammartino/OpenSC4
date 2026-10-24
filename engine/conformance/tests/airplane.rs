// SPDX-License-Identifier: GPL-3.0-or-later
//! aircraft.md §8 tests 4–5, 8–13; full owner codec tests are in aircraft.rs.

use fc4_core::automata::airplane::*;
use fc4_core::automata::{AircraftGoalOutcome, Airplane, AirplaneDriveParameters, DriveParameterSource, SetStateArgs};

#[test]
fn name_only_airplane_crop_dust_pops_without_a_flight_model() {
    let mut plane = Airplane::default();
    assert!(plane.base_mut().push_state(51, SetStateArgs::default()));
    assert!(plane.base_mut().push_state(58, SetStateArgs::default()));
    assert_eq!(plane.pursue_goal(), Ok(AircraftGoalOutcome::Popped));
    assert_eq!(plane.base().states.last().unwrap().state_type, 51);
}

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} (tolerance {tolerance})"
    );
}

struct Group<'a> {
    record: Option<[f32; 18]>,
    parent: Option<&'a dyn DriveParameterSource>,
}

impl DriveParameterSource for Group<'_> {
    fn drive_parameters(&self) -> Option<&[f32]> {
        self.record.as_ref().map(|record| record.as_slice())
    }
    fn parent(&self) -> Option<&dyn DriveParameterSource> {
        self.parent
    }
}

#[test]
fn named_record_missing_fields_and_whole_record_inheritance() {
    let parent = Group {
        record: Some([2.0; 18]),
        parent: None,
    };
    let record = import_airplane_record(|name| (name == "ElevatorMax").then_some(0.4));
    for (i, value) in record.into_iter().enumerate() {
        assert_eq!(value, if i == 4 { 0.4 } else { 0.0 });
    }
    let child = Group {
        record: Some(record),
        parent: Some(&parent),
    };
    let loaded = AirplaneDriveParameters::load(Some(&child)).0;
    assert_eq!(loaded[5].to_bits(), 0.1f32.to_bits());
    assert_eq!(loaded[0], 100.0);
    let empty = Group {
        record: Some(import_airplane_record(|_| None)),
        parent: Some(&parent),
    };
    assert_eq!(AirplaneDriveParameters::load(Some(&empty)).0[0], 100.0);
    let child = Group {
        record: None,
        parent: Some(&parent),
    };
    assert_eq!(AirplaneDriveParameters::load(Some(&child)).0[0], 2.0);
    let child = Group {
        record: None,
        parent: None,
    };
    let defaults = AirplaneDriveParameters::load(Some(&child)).0;
    assert_eq!(defaults[9].to_bits(), 0x3D32B8C3);
    assert_eq!(defaults[4].to_bits(), 0x3E860A92);
}

#[test]
fn parameters_preserve_negative_nonfinite_and_signed_zero_rules() {
    let mut record = [0.0; 18];
    record[2] = -3.0;
    record[3] = f32::INFINITY;
    record[4] = f32::NAN;
    record[10] = -0.0;
    let p = AirplaneDriveParameters::from_list(Some(&record)).0;
    assert_eq!(p[2], -3000.0);
    assert_eq!(p[3], f32::INFINITY);
    assert!(p[4].is_nan());
    assert!(p[5].is_nan());
    assert_eq!(p[10], 10000.0);
    record[2] = -0.0;
    assert_eq!(AirplaneDriveParameters::from_list(Some(&record)).0[2], 25.0);
}

#[test]
#[should_panic(expected = "airplane record requires 18 binary32 fields")]
fn raw_short_record_is_not_zero_padded() {
    AirplaneDriveParameters::from_list(Some(&[0.0; 17]));
}

#[test]
fn pre_force_limiter_vectors() {
    for (input, expected) in [
        ([0.0; 3], [0.0; 3]),
        ([0.00001, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([90.0, 120.0, 0.0], [90.0, 120.0, 0.0]),
        ([120.0, 160.0, 0.0], [90.0, 120.0, 0.0]),
    ] {
        for (actual, expected) in limit_velocity(input, 150.0).into_iter().zip(expected) {
            near(actual, expected, 0.00001);
        }
    }
    assert_eq!(limit_velocity([0.00001, 0.0, 0.0], 0.5), [1.0, 0.0, 0.0]);
    assert_eq!(limit_velocity([0.0, 0.0, 200.0], 150.0), [0.0, 0.0, 150.0]);
}

#[test]
fn candidate_motion_uses_mean_velocity_and_no_second_cap() {
    let motion = integrate_candidate([1.0, 2.0, 3.0], [10.0, 0.0, 0.0], [200.0, 0.0, 0.0], 100.0, 0.1);
    near(motion.velocity[0], 10.2, 0.00001);
    near(motion.position[0], 2.01, 0.00001);
    assert_eq!(&motion.position[1..], &[2.0, 3.0]);
    let motion = integrate_candidate([0.0; 3], [150.0, 0.0, 0.0], [200.0, 0.0, 0.0], 100.0, 0.1);
    near(motion.velocity[0], 150.2, 0.00001);
    let negative_mass = integrate_candidate([0.0; 3], [0.0; 3], [200.0, 0.0, 0.0], -100.0, 0.1);
    near(negative_mass.velocity[0], -0.2, 0.00001);
    for phase in [0, 1] {
        assert_eq!(
            project_ground_velocity([2.0, 3.0, 4.0], [0.0, 0.0, 1.0], phase),
            [0.0, 0.0, 4.0]
        );
    }
    assert_eq!(
        project_ground_velocity([2.0, 3.0, 4.0], [0.0, 0.0, 1.0], 3),
        [2.0, 3.0, 4.0]
    );
}

#[test]
fn time_conversion_caps_once_without_a_lower_clamp() {
    near(physics_seconds(100.0, None), 0.1, 0.000001);
    assert_eq!(physics_seconds(1000.0, None).to_bits(), 0x3E4CCCCD);
    assert_eq!(physics_seconds(0.0, None), 0.0);
    near(physics_seconds(-100.0, None), -0.1, 0.000001);
    near(physics_seconds(100.0, Some(0.5)), 0.05, 0.000001);
    assert_eq!(physics_seconds(100.0, Some(3.0)).to_bits(), 0x3E4CCCCD);
}

#[test]
fn scalar_lift_curve_and_horizontal_inverse_cosine() {
    let a = f32::from_bits(0x3FC90FDB);
    let sigma = a / 3.0;
    for (angle, expected) in [
        (0.0, 0.08),
        (sigma / 2.0, 0.38),
        (sigma, 0.68),
        ((sigma + a) / 2.0, 0.34),
        (a, 0.0),
    ] {
        near(lift_coefficient(angle, sigma, 0.08, 0.68, 0.0), expected, 0.000001);
    }
    near(lift_coefficient(a, a, 0.08, 0.68, 0.0), 0.68, 0.000001);
    let params = AirplaneDriveParameters::default();
    for orientation in [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0]] {
        let terms = params.force_scalars([0.0, 0.0, 10.0], orientation, 0.0, 0.0, 1.0, 2);
        near(terms.angle, a, 0.000001);
    }
}

#[test]
fn pressure_ground_resistance_and_drag_normalization() {
    let params = AirplaneDriveParameters::default();
    let terms = params.force_scalars([10.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, 0.0, 1.0, 1);
    near(terms.pressure, 50.0, 0.0001);
    let flaps = params.force_scalars([10.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0, 1.0, 1.0, 1);
    near(flaps.pressure, 70.0, 0.0001);
    for phase in [0, 1] {
        near(terms.rolling_resistance(100.0, 0.0, phase), 100.0, 0.0001);
    }
    assert_eq!(terms.rolling_resistance(100.0, 0.0, 2), 0.0);
    let weight = terms.gravity.abs();
    assert_eq!(terms.rolling_resistance(100.0, weight, 1), 0.0);
    near(terms.rolling_resistance(100.0, weight * 0.5, 1), 50.0, 0.0001);
    near(terms.drag_normalization, 10000.0 / (150.0 * 150.0 * 0.5 * 0.1), 0.00001);
    let drag = terms.drag_magnitude(100.0);
    near(
        drag,
        100.0 + (terms.angle * terms.angle + 0.1) * 50.0 * terms.drag_normalization,
        0.0001,
    );
    let zero = params.force_scalars([0.0; 3], [0.0, 0.0, 2.0], 0.0, 0.0, 1.0, 0);
    assert_eq!(zero.direction, [0.0, 0.0, 1.0]);
    assert_eq!(zero.pressure, 0.0);
}

#[test]
fn reverse_throttle_gate_only_blocks_backward_ground_motion() {
    let params = AirplaneDriveParameters::default();
    for (velocity, ground_thrust) in [
        ([0.0, 0.0, 10.0], -5000.0),
        ([0.0; 3], -5000.0),
        ([0.0, 0.0, -10.0], 0.0),
    ] {
        for phase in [0, 1] {
            assert_eq!(
                params
                    .force_scalars(velocity, [0.0, 0.0, 1.0], -0.5, 0.0, 1.0, phase)
                    .thrust,
                ground_thrust
            );
        }
        assert_eq!(
            params
                .force_scalars(velocity, [0.0, 0.0, 1.0], -0.5, 0.0, 1.0, 2)
                .thrust,
            -5000.0
        );
    }
}
