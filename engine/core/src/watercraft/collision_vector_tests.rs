// SPDX-License-Identifier: GPL-3.0-or-later
//! Composed boat-collision vectors WC5-B01–B06, B08, F05 and S08 (§§5.7.0–5.7.3).
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};

fn geometry(x: f32) -> Option<ModelGeometry> {
    Some(ModelGeometry {
        bounds: [[-1.0, 0.0, -1.0], [1.0, 1.0, 1.0]],
        transform: Transform {
            translation: [x, 0.0, 0.0],
            ..Transform::default()
        },
        queried_interface: false,
    })
}

fn colliding(position: Vec3, length: f32, model_x: f32) -> Watercraft {
    let mut b = boat();
    b.inherited_position = position;
    b.position = position;
    b.previous_position = position;
    b.occupant_bounds = [[0.0; 3], [length, 1.0, 1.0]];
    b.model_geometry = geometry(model_x);
    b
}

fn sized(mut b: Watercraft, id: u32) -> Watercraft {
    b.group = std::sync::Arc::new(Group {
        id,
        ..b.group.as_deref().unwrap().clone()
    })
    .into();
    b
}

#[test]
fn wc5_b01_broad_phase_uses_maximum_length_and_rejects_equality() {
    for (distance, found) in [(10.0, None), (9.999, Some(2))] {
        let mut f = Fleet::default();
        f.insert(1, colliding([0.0; 3], 10.0, 0.0)).unwrap();
        f.insert(2, colliding([0.0, 0.0, distance], 6.0, 0.0)).unwrap();
        assert_eq!(f.find_overlapping_boat(1, [0.0; 3], 1.0), Ok(found));
    }
}

#[test]
fn wc5_b02_footprint_edge_contact_overlaps_and_invalid_rejects() {
    let a = Footprint {
        valid: true,
        center: [0.0, 0.0],
        half_extents: [1.0, 1.0],
        axis: [1.0, 0.0],
        scratch: [0.0; 2],
    };
    let mut b = Footprint {
        center: [2.0, 0.0],
        ..a.clone()
    };
    assert!(a.overlaps(&b) && b.overlaps(&a));
    b.center[0] = 2.01;
    assert!(!a.overlaps(&b));
    b.center[0] = 2.0;
    let mut invalid = a.clone();
    invalid.valid = false;
    assert!(!invalid.overlaps(&b) && !b.overlaps(&invalid));
}

#[test]
fn wc5_b03_size_class_prefers_4414_over_4413() {
    let parent = std::sync::Arc::new(Group {
        id: 0x4413,
        parameters: None,
        aems_value: None,
        parent: None,
    });
    let both = Group {
        id: 0x4414,
        parameters: None,
        aems_value: None,
        parent: Some(parent.clone()),
    };
    assert_eq!(both.size_class(), 3);
    assert_eq!(parent.size_class(), 2);
    let neither = Group {
        id: 1,
        parameters: None,
        aems_value: None,
        parent: None,
    };
    assert_eq!(neither.size_class(), 1);
}

#[test]
fn wc5_b04_player_and_ai_equal_class_both_sink_with_own_durations() {
    let mut f = Fleet::default();
    let mut a = boat();
    a.speed = 0.3;
    a.drive.0[21] = 1000.0;
    f.insert(1, a).unwrap();
    f.push_state(1, State::new(state::PLAYER_DRIVE), &mut None).unwrap();
    let mut b = boat();
    b.drive.0[21] = 2000.0;
    f.insert(2, b).unwrap();
    f.get_mut(1).unwrap().events.clear();
    assert_eq!(
        f.respond_to_boat_collision(1, 2, [Cell { x: 1, z: 1 }; 4], &Water, &mut None),
        Ok(true)
    );
    let a = f.get(1).unwrap();
    assert!(a.sunk_by_collision);
    assert_eq!(a.top_state(), Some(state::SINK));
    assert_eq!(a.record.base.states.last().unwrap().base.countdown_ms, 1000.0);
    assert_eq!(a.events, [Event::Aems(7), Event::Notify(0xB, 0), Event::Notify(0xB, 0)]);
    let b = f.get(2).unwrap();
    assert!(!b.sunk_by_collision);
    assert_eq!(b.record.base.states.last().unwrap().base.countdown_ms, 2000.0);
    assert_eq!(b.events, [Event::Notify(0xB, 0), Event::Notify(0xB, 0)]);
}

#[derive(Default)]
struct Counting(usize);
impl CollisionQueries for Counting {
    fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
        self.0 += 1;
        [Cell { x: 10, z: 10 }; 4]
    }
    fn occupied(&mut self, _: Cell) -> bool {
        panic!("boat crash skips shore detection")
    }
    fn is_wet(&mut self, _: Cell) -> bool {
        panic!("boat crash skips shore detection")
    }
    fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
        panic!("boat crash skips shore detection")
    }
}

struct Clear;
impl CollisionQueries for Clear {
    fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
        [Cell { x: 10, z: 10 }; 4]
    }
    fn occupied(&mut self, _: Cell) -> bool {
        false
    }
    fn is_wet(&mut self, _: Cell) -> bool {
        true
    }
    fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
        [0.0, 1.0, 0.0]
    }
}

#[test]
fn wc5_b05_larger_player_sinks_only_ai_and_skips_shore_detection() {
    let mut f = Fleet::default();
    let mut a = sized(colliding([0.0; 3], 10.0, 0.0), 0x4414);
    a.speed = 0.3;
    f.insert(1, a).unwrap();
    f.push_state(1, State::new(state::PLAYER_DRIVE), &mut None).unwrap();
    f.insert(2, colliding([1.0, 0.0, 0.0], 10.0, 0.5)).unwrap();
    let mut queries = Counting::default();
    assert_eq!(f.detect_collision(1, false, &Water, &mut queries, &mut None), Ok(true));
    assert_eq!(queries.0, 1);
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::PLAYER_DRIVE));
    let b = f.get(2).unwrap();
    assert_eq!(b.top_state(), Some(state::SINK));
    assert!(!b.sunk_by_collision);
}

#[test]
fn wc5_b06_smaller_ai_caller_sinks_without_aems_or_collision_byte() {
    let mut f = Fleet::default();
    let mut a = colliding([0.0; 3], 10.0, 0.0);
    a.speed = 0.3;
    f.insert(1, a).unwrap();
    f.insert(2, sized(colliding([1.0, 0.0, 0.0], 10.0, 0.5), 0x4414))
        .unwrap();
    f.push_state(2, State::new(state::PLAYER_DRIVE), &mut None).unwrap();
    let mut queries = Counting::default();
    assert_eq!(f.detect_collision(1, true, &Water, &mut queries, &mut None), Ok(true));
    let a = f.get(1).unwrap();
    assert_eq!(a.top_state(), Some(state::SINK));
    assert!(!a.sunk_by_collision);
    assert!(!a.events.contains(&Event::Aems(7)));
    assert_eq!(f.get(2).unwrap().top_state(), Some(state::PLAYER_DRIVE));
    // Without the AI-collision enable an AI caller performs no boat scan.
    let mut f = Fleet::default();
    let mut a = colliding([0.0; 3], 10.0, 0.0);
    a.speed = 0.3;
    a.docking_in_progress = true;
    f.insert(1, a).unwrap();
    f.insert(2, colliding([1.0, 0.0, 0.0], 10.0, 0.5)).unwrap();
    assert_eq!(f.detect_collision(1, false, &Water, &mut Clear, &mut None), Ok(false));
    assert_eq!(f.get(1).unwrap().top_state(), None);
    assert_eq!(f.get(2).unwrap().top_state(), None);
}

#[test]
fn wc5_b08_exact_threshold_equality_bumps_and_transfers_speed() {
    let mut f = Fleet::default();
    let mut a = boat();
    a.drive.0[7] = 1.0;
    a.drive.0[15] = 0.1;
    a.drive.0[29] = 0.75;
    a.speed = 0.2;
    f.insert(1, a).unwrap();
    let mut b = boat();
    b.speed = 0.1;
    b.heading = [-1.0, 0.0, 0.0];
    f.insert(2, b).unwrap();
    assert_eq!(0.2_f32 - 0.1_f32, 0.1_f32);
    assert_eq!(
        f.respond_to_boat_collision(1, 2, [Cell { x: 1, z: 1 }; 4], &Water, &mut None),
        Ok(false)
    );
    assert_eq!(f.get(2).unwrap().speed, 0.2);
    assert_eq!(f.get(1).unwrap().speed, 0.75 * 0.2);
    assert_eq!(f.get(2).unwrap().top_state(), None);
}

#[test]
fn wc5_f05_first_listed_candidate_wins_and_candidates_ignore_caller_offset() {
    let mut f = Fleet::default();
    f.insert(1, colliding([0.0; 3], 10.0, 0.0)).unwrap();
    f.insert(2, colliding([3.0, 0.0, 0.0], 10.0, 100.4)).unwrap();
    f.insert(3, colliding([1.0, 0.0, 0.0], 10.0, 100.0)).unwrap();
    assert_eq!(f.find_overlapping_boat(1, [100.0, 0.0, 0.0], 1.0), Ok(Some(2)));
    // Without the caller offset neither candidate overlaps the caller.
    assert_eq!(f.find_overlapping_boat(1, [0.0; 3], 1.0), Ok(None));
    // The caller multiplier scales only the caller footprint.
    assert_eq!(f.find_overlapping_boat(1, [98.0, 0.0, 0.0], 3.0), Ok(Some(2)));
}

#[test]
fn wc5_s08_bump_rollback_restores_previous_translation_then_publishes() {
    let mut b = boat();
    b.previous_position = [1.0, 2.0, 3.0];
    b.position = [4.0, 5.0, 6.0];
    b.inherited_position = b.position;
    b.transform.translation = b.position;
    b.speed = 0.2;
    b.turn = 0.7;
    let basis = b.transform.basis;
    let heading = b.heading;
    b.bump_rollback();
    assert_eq!(b.position, [1.0, 2.0, 3.0]);
    assert_eq!(b.inherited_position, [1.0, 2.0, 3.0]);
    assert_eq!(b.transform.translation, [1.0, 2.0, 3.0]);
    assert_eq!(b.transform.basis, basis);
    assert_eq!(b.heading, heading);
    assert_eq!((b.speed, b.turn), (0.2, 0.7));
    assert_eq!(
        b.events,
        [
            Event::Publish(Publication::Transform(b.transform.clone())),
            Event::Publish(Publication::Update(true, true)),
            Event::EffectUpdate,
            Event::ModelUpdate,
        ]
    );
}
