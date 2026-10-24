// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;
use crate::automata::Vec3;
use std::cell::Cell as Counter;
use std::sync::Arc;

struct SinkTerrain {
    height: f32,
    samples: Counter<u32>,
}
impl Terrain for SinkTerrain {
    fn contains(&self, _: Cell) -> bool {
        true
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        Cell { x: 0, z: 0 }
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        self.height
    }
    fn sea_level(&self) -> f32 {
        10.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        self.samples.set(self.samples.get() + 1);
        10.0
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}

#[test]
fn wc5_s01_through_s07_ramps_heights_and_endpoints() {
    let terrain = SinkTerrain {
        height: 14.0,
        samples: Counter::new(0),
    };
    let mut b = boat();
    b.drive.0[21] = 8000.0;
    b.drive.0[22] = 1.0;
    b.drive.0[25] = 1.0;
    b.drive.0[24] = 1.0; // Completion section §5.7.5 / WC5-H10.
    b.crash_elapsed = 2000.0;
    b.transform.translation = [1.0, 7.0, 3.0];
    assert!(b.sinking_tick(&terrain));
    assert_eq!(b.crash_angle, 0.25);
    assert_eq!(b.transform.translation, [1.0, 7.0, 3.0]);
    assert_eq!(terrain.samples.get(), 0);
    b.crash_elapsed = 8000.0;
    b.events.clear();
    let transform = b.transform.clone();
    let heading = b.heading;
    assert!(!b.sinking_tick(&terrain));
    assert_eq!(b.crash_angle, 1.0);
    assert_eq!(b.transform, transform);
    assert_eq!(b.heading, heading);
    assert!(b.events.is_empty());
    b.crash_elapsed = 6000.0;
    b.position = [1.0, 2.0, 3.0];
    assert!(b.sinking_tick(&terrain));
    assert_eq!(b.previous_position, [1.0, 2.0, 3.0]);
    assert_eq!(b.position, [1.0, 11.5, 3.0]);
    let u = (0.75 - 0.6_f32) / 0.6_f32;
    assert_eq!(b.transform.translation[1], 10.0 + 2.0 * u);
    assert_eq!(terrain.samples.get(), 2);
    assert!(b.sinking_tick(&SinkTerrain {
        height: 6.0,
        samples: Counter::new(0)
    }));
    assert_eq!(b.position[1], 10.0);
    assert_eq!(b.transform.translation[1], 10.0);
    b.crash_elapsed = 3200.0;
    b.shore_contact = false;
    b.crash_angle = 0.0;
    assert!(!b.heel_angle_step());
    assert_eq!(b.crash_angle, 1.0);
    b.shore_contact = true;
    b.crash_angle = 0.0;
    assert!(b.heel_angle_step());
    assert_eq!(b.crash_angle, 0.4);
    b.crash_elapsed = 4000.0;
    assert!(!b.heel_angle_step());
    b.crash_elapsed = 8000.0;
    b.crash_angle = 0.123;
    assert!(!b.heel_angle_step());
    assert_eq!(b.crash_angle, 0.123);
}

fn close(a: Vec3, b: Vec3) {
    for (a, b) in a.into_iter().zip(b) {
        assert!((a - b).abs() < 1e-5, "{a} != {b}");
    }
}

#[test]
fn wc5_k01_through_k04_sinking_basis_signs_and_heading_feedback() {
    for (bytes, pitch_sign, roll_sign) in [([0, 0], 1.0, 1.0), ([2, 0], -1.0, 1.0), ([0, 1], 1.0, -1.0)] {
        let mut b = boat();
        b.heading = [0.0, 0.0, 1.0];
        b.previous_heading = b.heading;
        b.crash_direction = bytes;
        b.drive.0[21] = 1.0;
        b.drive.0[22] = 1.0;
        b.crash_elapsed = std::f32::consts::PI / 6.0;
        assert!(b.sinking_tick(&Water));
        let forward = [0.0, pitch_sign * 0.5, 3.0_f32.sqrt() / 2.0];
        close(b.transform.basis[2], forward);
        close(b.heading, forward);
        close(b.inherited_heading, forward);
        let reference = [roll_sign * 0.5, 3.0_f32.sqrt() / 2.0, 0.0];
        let right = super::math::normalize(super::math::cross(reference, forward));
        close(b.transform.basis[0], right);
        close(b.transform.basis[1], super::math::cross(forward, right));
        assert_eq!(b.transform.flags, 4);
        assert_eq!(b.transform.revision, 1);
        assert_eq!(b.previous_heading, [0.0, 0.0, 1.0]);
    }
    let mut b = boat();
    b.transform.scale = 2.0;
    assert!(b.sinking_tick(&Water));
    close(b.transform.basis[0], [0.0, 0.0, -1.0]);
    close(b.heading, [0.0, 0.0, -1.0]);
    assert_eq!(b.previous_heading, [1.0, 0.0, 0.0]);
    assert_eq!(b.transform.scale, 2.0);
}

#[test]
fn inherited_seek_uses_inherited_vectors_overshoots_and_keeps_zero_time_division() {
    let mut b = boat();
    b.inherited_position = [1.0, 2.0, 3.0];
    b.inherited_destination = [-1.0, 20.0, 30.0];
    b.inherited_measured_speed = 5.0;
    b.generic_seek(100.0, 1.0, |_, _, _| panic!("near destination skips adjustment"));
    assert_eq!(b.inherited_destination, [1.0, 2.0, 3.0]);
    assert_eq!(b.inherited_measured_speed, 5.0);
    assert_eq!(b.events, [Event::Notify(4, 0)]);
    b.inherited_destination = [2.0, 2.0, 3.0];
    b.player.speed_tween.target = 0.5;
    b.generic_seek(10.0, 2.0, |p, n, flags| {
        assert_eq!(p, [1.0, 2.0, 3.0]);
        assert_eq!(flags, 0xC49);
        n[2] += 1.0;
    });
    assert_eq!(b.inherited_position, [11.0, 2.0, 4.0]);
    assert_eq!(b.position, [0.0; 3]);
    assert!((b.inherited_measured_speed - 101.0_f32.sqrt() / 10.0).abs() < 1e-6);
    b.record.base.automata_flags |= 0x20000;
    b.generic_seek(0.0, 2.0, |_, _, _| panic!("flag bypass"));
    assert!(b.inherited_measured_speed.is_nan());
    b.player.speed_tween = SpeedTween {
        start: 2.0,
        target: 8.0,
        elapsed: 0.9,
        duration: 1.0,
    };
    b.advance_speed_tween(200.0);
    assert_eq!(
        b.player.speed_tween,
        SpeedTween {
            start: 8.0,
            target: 8.0,
            elapsed: 0.0,
            duration: 0.0
        }
    );
}

#[test]
fn wc7_r02_r08_backtrack_survivor_and_fill_to_ten() {
    let mut b = boat();
    b.travel = (0..6)
        .map(|i| TravelPoint {
            cell: Cell { x: i, z: 0 },
            direction: 3,
            tried: 1,
        })
        .collect();
    b.backtrack_travel();
    assert_eq!(b.travel.len(), 1);
    assert_eq!(b.travel[0].tried, 0x09);
    b.travel.clear();
    b.neighbor = Some(NeighborRoute {
        occupant: Arc::new(Resource(1)),
        position: [0.0; 3],
        cells: (0..12).map(|i| Cell { x: i, z: 0 }).collect::<Vec<_>>().into(),
        next: 0,
    });
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    assert_eq!(f.fill_route_points(1, &Water, &mut None), Ok(true));
    let b = f.get(1).unwrap();
    assert_eq!(b.travel.len(), 10);
    assert!(b.travel.iter().all(|p| p.direction == u32::MAX && p.tried == 0));
    assert_eq!(b.neighbor.as_ref().unwrap().next, 10);
    f.get_mut(1).unwrap().travel.clear();
    assert_eq!(f.fill_route_points(1, &Water, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().travel.len(), 2);
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
}

struct Queries {
    outside: bool,
    calls: Vec<&'static str>,
}
impl CollisionQueries for Queries {
    fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
        self.calls.push("rectangle");
        [Cell {
            x: if self.outside { -1 } else { 0 },
            z: 0,
        }; 4]
    }
    fn occupied(&mut self, _: Cell) -> bool {
        self.calls.push("occupied");
        true
    }
    fn is_wet(&mut self, _: Cell) -> bool {
        self.calls.push("water");
        true
    }
    fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
        panic!("all wet skips normal");
    }
}

#[test]
fn detector_outside_removal_precedes_queries_and_shore_bump_preserves_speed() {
    let mut f = Fleet::default();
    f.insert(1, boat()).unwrap();
    let mut queries = Queries {
        outside: true,
        calls: Vec::new(),
    };
    assert_eq!(f.detect_collision(1, false, &Water, &mut queries, &mut None), Ok(true));
    assert_eq!(queries.calls, ["rectangle"]);
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::IMMEDIATE_REMOVE));
    let mut b = boat();
    b.previous_position = [1.0, 2.0, 3.0];
    b.speed = 0.01;
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    queries.outside = false;
    queries.calls.clear();
    assert_eq!(f.detect_collision(1, false, &Water, &mut queries, &mut None), Ok(false));
    assert_eq!(f.get(1).unwrap().position, [1.0, 2.0, 3.0]);
    assert_eq!(f.get(1).unwrap().speed, 0.01);
    assert_eq!(queries.calls, ["rectangle", "rectangle", "occupied"]);
    f.get_mut(1).unwrap().speed = 0.3;
    queries.calls.clear();
    assert_eq!(f.detect_collision(1, false, &Water, &mut queries, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
    assert_eq!(
        queries.calls,
        [
            "rectangle",
            "rectangle",
            "occupied",
            "rectangle",
            "water",
            "water",
            "water",
            "water"
        ]
    );
}
