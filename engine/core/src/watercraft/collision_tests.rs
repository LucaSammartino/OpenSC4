// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;
use crate::automata::State;

#[test]
fn wc5_f01_through_f04_model_footprint_builder() {
    let mut model = ModelGeometry {
        bounds: [[-4.0, -1.0, -8.0], [6.0, 3.0, 12.0]],
        transform: Transform {
            translation: [100.0, 5.0, 200.0],
            ..Transform::default()
        },
        queried_interface: false,
    };
    let mut footprint = Footprint::default();
    footprint.build(Some(&model), [3.0, 99.0, -4.0], 2.0);
    assert_eq!(footprint.center, [103.0, 196.0]);
    assert_eq!(footprint.half_extents, [9.5, 19.5]);
    assert_eq!(footprint.axis, [1.0, 0.0]);
    assert!(footprint.valid);
    model.queried_interface = true;
    footprint.build(Some(&model), [3.0, 99.0, -4.0], 2.0);
    assert_eq!(footprint.half_extents, [8.0, 19.0]);
    model.transform.basis[0][0] = 0.0;
    model.transform.basis[2][0] = 2.0;
    footprint.build(Some(&model), [0.0; 3], 1.0);
    assert_eq!(footprint.axis, [0.0, -1.0]);
    footprint.axis = [0.0, 1.0];
    footprint.build(None, [1.0; 3], 999.0);
    assert_eq!(footprint.center, [0.0; 2]);
    assert_eq!(footprint.half_extents, [0.0; 2]);
    assert_eq!(footprint.axis, [0.0, 1.0]);
    assert!(footprint.valid);
}

#[test]
fn footprint_sat_includes_touching_edges_and_rejects_invalid() {
    let a = Footprint {
        valid: true,
        center: [0.0; 2],
        half_extents: [1.0, 1.0],
        axis: [1.0, 0.0],
        scratch: [0.0; 2],
    };
    let mut b = a.clone();
    b.center = [2.0, 0.0];
    assert!(a.overlaps(&b));
    b.center[0] = 2.0001;
    assert!(!a.overlaps(&b));
    b = a.clone();
    b.valid = false;
    assert!(!a.overlaps(&b));
}

#[test]
fn wc5_i01_through_i05_all_inanimate_corner_results_and_query_order() {
    let expected = [
        [[1, 0], [1, 0], [0, 1], [0, 1], [0, 0], [1, 1], [0, 0], [1, 1]],
        [[1, 1], [1, 1], [0, 0], [0, 0], [1, 0], [0, 1], [1, 0], [0, 1]],
        [[0, 0], [0, 0], [1, 1], [1, 1], [0, 1], [1, 0], [0, 1], [1, 0]],
        [[0, 1], [0, 1], [1, 0], [1, 0], [1, 1], [0, 0], [1, 1], [0, 0]],
    ];
    let headings = [
        [2.0, 0.0, -1.0],
        [2.0, 0.0, 1.0],
        [-2.0, 0.0, -1.0],
        [-2.0, 0.0, 1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, -1.0],
        [-1.0, 0.0, 1.0],
    ];
    let corners = [
        Cell { x: 2, z: 3 },
        Cell { x: 2, z: 5 },
        Cell { x: 4, z: 3 },
        Cell { x: 4, z: 5 },
    ];
    for (corner, results) in expected.into_iter().enumerate() {
        for (column, heading) in headings.into_iter().enumerate() {
            let mut b = boat();
            b.inherited_heading = heading;
            let mut queried = Vec::new();
            assert!(b.inanimate_collision(
                [2, 3, 4, 5],
                |c| {
                    queried.push(c);
                    c == corners[corner]
                },
                |_| panic!("occupants take priority")
            ));
            assert_eq!(queried, &corners[..=corner]);
            assert_eq!(b.crash_direction, results[column]);
        }
    }
    let mut b = boat();
    b.docking_in_progress = true;
    b.crash_direction = [1, 0];
    assert!(!b.inanimate_collision([2, 3, 4, 5], |_| false, |_| panic!("docking bypasses navigability")));
    assert_eq!(b.crash_direction, [1, 0]);
    b.inherited_heading = [0.0; 3];
    assert!(b.inanimate_collision([2, 3, 4, 5], |_| true, |_| false));
    assert_eq!(b.crash_direction, [1, 1]);
}

#[test]
fn wc5_b07_b08_b09_b10_size_ties_speed_threshold_and_bump_transfer() {
    let rectangle = [Cell { x: 1, z: 1 }; 4];
    let mut f = Fleet::default();
    let mut a = boat();
    a.speed = 0.2;
    a.position = [4.0, 5.0, 6.0];
    a.previous_position = [1.0, 2.0, 3.0];
    f.insert(1, a).unwrap();
    let mut b = boat();
    b.speed = 0.2;
    b.heading = [-1.0, 0.0, 0.0];
    f.insert(2, b).unwrap();
    assert_eq!(
        f.respond_to_boat_collision(1, 2, rectangle, &Water, &mut None),
        Ok(false)
    );
    assert_eq!(f.get(1).unwrap().position, [1.0, 2.0, 3.0]);
    assert_eq!(f.get(1).unwrap().speed, 0.15);
    assert_eq!(f.get(2).unwrap().speed, 0.2);
    f.get_mut(1).unwrap().speed = 0.3;
    f.get_mut(2).unwrap().speed = 0.0;
    assert_eq!(
        f.respond_to_boat_collision(1, 2, rectangle, &Water, &mut None),
        Ok(true)
    );
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
    assert_eq!(f.get(2).unwrap().top_state(), Some(state::SLOW_REMOVE));
    assert!(!f
        .get(1)
        .unwrap()
        .events
        .iter()
        .any(|e| matches!(e, Event::Notify(_, _))));
    let mut f = Fleet::default();
    let mut a = boat();
    a.speed = 0.2;
    f.insert(1, a).unwrap();
    let mut b = boat();
    b.speed = 0.2;
    b.record.base.automata_flags |= 1;
    f.insert(2, b).unwrap();
    assert_eq!(
        f.respond_to_boat_collision(1, 2, rectangle, &Water, &mut None),
        Ok(false)
    );
    assert_eq!(f.get(2).unwrap().speed, 0.2);
}

#[test]
fn player_involvement_sinks_ai_without_player_mushroom_byte() {
    let mut f = Fleet::default();
    let mut a = boat();
    a.speed = 0.3;
    a.group = std::sync::Arc::new(Group {
        id: 0x4414,
        ..(*a.group).clone()
    });
    f.insert(1, a).unwrap();
    f.push_state(1, State::new(state::PLAYER_DRIVE), &mut None).unwrap();
    f.insert(2, boat()).unwrap();
    assert_eq!(
        f.respond_to_boat_collision(1, 2, [Cell { x: 1, z: 1 }; 4], &Water, &mut None),
        Ok(true)
    );
    let b = f.get(2).unwrap();
    assert_eq!(b.top_state(), Some(state::SINK));
    assert!(!b.sunk_by_collision);
    assert_eq!(b.events.iter().filter(|e| **e == Event::Notify(0xb, 0)).count(), 2);
}
