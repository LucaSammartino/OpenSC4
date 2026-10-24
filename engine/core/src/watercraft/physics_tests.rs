// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;

fn close(actual: [f32; 3], expected: [f32; 3]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 0.00001, "{actual:?} != {expected:?}");
    }
}

#[test]
fn wc4_m01_m02_straight_uses_horizontal_heading_and_signed_time() {
    for (dt, x) in [(100.0, 12.0), (-100.0, 8.0)] {
        let mut b = boat();
        b.position = [10.0, 2.0, 20.0];
        b.heading = [3.0, 4.0, 0.0];
        b.speed = 0.1;
        b.movement_position_new(dt, &Water);
        close(b.position, [x, 2.0, 20.0]);
        assert_eq!(b.heading, [3.0, 4.0, 0.0]);
        close(b.transform.translation, [x, 2.0 + b.drive.0[18] / 2.0, 20.0]);
        assert!(matches!(
            b.events.as_slice(),
            [
                Event::Publish(Publication::Transform(_)),
                Event::Publish(Publication::Update(true, true)),
                Event::EffectUpdate,
                Event::ModelUpdate
            ]
        ));
    }
}

#[test]
fn wc4_m03_m04_curved_radius_is_not_a_second_turn_multiplier() {
    for (turn, radius, theta) in [(2.0, 75.0, 0.08_f32), (1.0, 100.0, 0.06_f32)] {
        let mut b = boat();
        b.heading = [0.0, 0.0, 1.0];
        b.speed = 0.3;
        b.turn = turn;
        b.movement_position_new(100.0, &Water);
        close(b.position, [radius * (1.0 - theta.cos()), 0.0, radius * theta.sin()]);
        close(b.heading, [theta.sin(), 0.0, theta.cos()]);
        assert_eq!(b.previous_heading, [0.0, 0.0, 1.0]);
    }
}

#[test]
fn wc4_m05_reverse_curve_and_m06_zero_speed_publication() {
    let mut b = boat();
    b.heading = [0.0, 0.0, 1.0];
    b.speed = -0.1;
    b.turn = -2.0;
    b.movement_position_new(100.0, &Water);
    let theta = 0.26666667_f32;
    close(b.heading, [theta.sin(), 0.0, theta.cos()]);
    close(b.position, [-7.5 + 7.5 * theta.cos(), 0.0, -7.5 * theta.sin()]);
    let mut b = boat();
    b.speed = 0.0;
    b.turn = 1.0;
    b.position = [1.0, 2.0, 3.0];
    b.previous_position = [-1.0; 3];
    let old = b.transform.clone();
    b.movement_position_new(99.0, &Water);
    assert_eq!(b.previous_position, b.position);
    assert_eq!(b.transform, old);
    assert_eq!(b.events.first(), Some(&Event::FindState(16)));
    assert_eq!(b.inherited_position, b.position);
}

#[test]
fn wc4_m07_visual_lift_uses_entry_18_and_m08_zero_time_projects_radius() {
    let mut b = boat();
    b.speed = 0.1;
    b.drive.0[17] = 1.0;
    b.drive.0[18] = 2.0;
    b.drive.0[24] = 4.0;
    b.movement_position_new(0.0, &Water);
    assert_eq!(b.position[1], 0.0);
    assert_eq!(b.transform.translation[1], 1.0);
    let mut b = boat();
    b.position = [0.0, 5.0, 0.0];
    b.heading = [0.0, 0.0, 1.0];
    b.speed = 0.3;
    b.turn = 2.0;
    b.movement_position_new(0.0, &Water);
    assert_ne!(b.position, [0.0, 5.0, 0.0]);
    assert!((super::math::length(super::math::sub(b.position, [75.0, 0.0, 0.0])) - 75.0).abs() < 0.00001);
}

#[test]
fn wc2_p02_slew_and_wc3_j04_j05_angular_quantization() {
    let mut b = boat();
    b.speed = 0.1;
    b.drive.0[14] = 0.002;
    b.slew_ai_targets(10.0, 0.3, -1.0);
    assert!((b.speed - 0.11).abs() < 0.000001);
    assert!((b.turn + 0.02).abs() < 0.000001);
    b.inherited_heading = [3.0, 0.0, 0.0];
    assert!(b.angular_proximity([2.0, 0.0, 0.0]));
    assert_eq!(b.inherited_heading, [1.0, 0.0, 0.0]);
    b.inherited_heading = [0.0, 0.0, 3.0];
    assert!(!b.angular_proximity([2.0, 0.0, 0.0]));
    assert_eq!(b.inherited_heading, [0.0, 0.0, 1.0]);
    b.heading = [3.0, 0.0, 0.0];
    assert_eq!(quantize_heading(b.heading), 3);
    assert_eq!(b.heading, [3.0, 0.0, 0.0]);
}

#[test]
fn wc7_r03_through_r06_compressed_reader_endpoints_runs_and_absolute() {
    for (bytes, expected) in [
        (vec![], vec![]),
        (vec![1, 10, 20], vec![(10, 20)]),
        (
            vec![1, 10, 20, 0x41, 4, 0xe4],
            vec![(10, 20), (9, 20), (9, 19), (10, 19), (10, 20)],
        ),
        (vec![1, 10, 20, 0x41, 0], vec![(10, 20)]),
        (vec![1, 10, 20, 0x82, 30, 40], vec![(10, 20), (30, 40)]),
    ] {
        let mut reader = RouteReader::from_bytes(&bytes).unwrap();
        let expected: Vec<_> = expected.into_iter().map(|(x, z)| Cell { x, z }).collect();
        assert_eq!(reader.cells(), expected);
        for c in &expected {
            assert_eq!(reader.get_next(), Some(*c));
        }
        assert_eq!(reader.get_next(), None);
        assert_eq!(reader.get_next(), None);
        reader.reset();
        assert_eq!(reader.get_next(), expected.first().copied());
    }
}

#[test]
fn wc3_l01_l05_l06_constructor_erase_and_sign() {
    let mut b = Watercraft::new(boat().record, boat().group, 5.0);
    assert_eq!(b.travel.len(), 11);
    assert_eq!(b.travel.capacity(), 11);
    assert!(b
        .travel
        .iter()
        .all(|p| p.cell == Cell { x: 0, z: 0 } && p.tried == 0 && p.direction == 0));
    b.travel.truncate(3);
    b.travel[2].cell.x = 3;
    let capacity = b.travel.capacity();
    assert_eq!(b.erase_travel_point(1), 1);
    assert_eq!(b.travel[1].cell.x, 3);
    assert_eq!(b.travel.capacity(), capacity);
    assert_eq!(b.erase_travel_point(1), b.travel.len());
    for (v, s) in [(-0.0, 1.0), (0.0, 1.0), (-2.0, -1.0), (2.0, 1.0)] {
        assert_eq!(sign(v), s);
    }
}

#[test]
fn wc4_v01_through_v04_visual_basis_and_degenerate_axis() {
    let mut b = boat();
    b.heading = [0.0, 0.0, 1.0];
    b.speed = 0.1;
    b.drive.0[17] = 1.0;
    let mut transform = Transform::default();
    b.apply_visual_attitude(&mut transform);
    close(transform.basis[0], [1.0, 0.0, 0.0]);
    close(transform.basis[1], [0.0, 0.1_f32.cos(), -0.1_f32.sin()]);
    close(transform.basis[2], [0.0, 0.1_f32.sin(), 0.1_f32.cos()]);
    assert_eq!(b.heading, [0.0, 0.0, 1.0]);
    b.drive.0[17] = 0.0;
    b.turn = 1.0;
    b.speed = 0.3;
    let a = b.drive.0[19];
    b.apply_visual_attitude(&mut transform);
    close(transform.basis[0], [a.cos(), -a.sin(), 0.0]);
    close(transform.basis[1], [a.sin(), a.cos(), 0.0]);
    close(transform.basis[2], [0.0, 0.0, 1.0]);
    b.speed = -0.1;
    b.drive.0[17] = 1.0;
    transform = Transform::default();
    b.apply_visual_attitude(&mut transform);
    assert_eq!(transform.basis, Transform::default().basis);
    b.heading = [0.0, 1.0, 0.0];
    b.apply_visual_attitude(&mut transform);
    assert!(transform.basis.iter().flatten().any(|x| x.is_nan()));
}

#[test]
fn following_uses_endpoint_gap_and_slack_does_not_publish() {
    let mut f = Fleet::default();
    let mut tug = boat();
    tug.position = [100.0, 0.0, 0.0];
    f.insert(1, tug).unwrap();
    let mut follower = boat();
    follower.tug = Some(1);
    f.insert(2, follower).unwrap();
    assert_eq!(f.following_tick(2, 0.0, &Water), Ok(true));
    // Tug rear=95, follower front=5, length=90, gap=75.
    assert_eq!(f.get(2).unwrap().position, [75.0, 0.0, 0.0]);
    f.get_mut(2).unwrap().events.clear();
    assert_eq!(f.following_tick(2, 1000.0, &Water), Ok(true));
    assert!(f.get(2).unwrap().events.is_empty());
    f.get_mut(2).unwrap().tug = None;
    assert_eq!(f.following_tick(2, 1000.0, &Water), Ok(false));
}

#[test]
fn record_same_input_round_trip_stabilizes_inherited_loaded_status() {
    use crate::stream::{ByteReader, ByteWriter};
    let record = boat().record;
    let mut initial = ByteWriter::new();
    assert!(record.write(&mut initial));
    let loaded = WatercraftRecord::read(&mut ByteReader::new(initial.bytes())).unwrap();
    let mut canonical = ByteWriter::new();
    assert!(loaded.write(&mut canonical));
    let loaded_again = WatercraftRecord::read(&mut ByteReader::new(canonical.bytes())).unwrap();
    let mut rewritten = ByteWriter::new();
    assert!(loaded_again.write(&mut rewritten));
    assert_eq!(loaded_again, loaded);
    assert_eq!(rewritten.bytes(), canonical.bytes());
}
