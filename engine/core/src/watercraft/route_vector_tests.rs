// SPDX-License-Identifier: GPL-3.0-or-later
//! Docking, undocking, reservation, direction and following vectors
//! WC7-D01–D07, U01–U03, B03, R01, T04 (§§5.2, 5.3–5.5, 6.3).
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::sync::Arc;

fn station(lot: u64, lot_position: Vec3, position: Vec3, direction: Vec3) -> Arc<Station> {
    Arc::new(Station {
        lot: Arc::new(Resource(lot)),
        occupant: Arc::new(Resource(lot + 100)),
        lot_position,
        position,
        direction,
        route: Arc::from(vec![Cell { x: 1, z: 1 }]),
    })
}

/// Destination A (route start differs from A's berth, so A is "other").
fn docking(position: Vec3, heading: Vec3, berth: Vec3, direction: Vec3) -> Fleet {
    let mut b = boat();
    b.inherited_position = position;
    b.position = position;
    b.inherited_heading = heading;
    b.heading = heading;
    b.route_start = [-50.0, 0.0, -50.0];
    let a = station(1, berth, berth, direction);
    b.stations = [Some(a.clone()), Some(a)];
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    f
}

/// Cell inside, terrain above water: OnLand is true.
struct Land;
impl Terrain for Land {
    fn contains(&self, cell: Cell) -> bool {
        Water.contains(cell)
    }
    fn cell_at(&self, position: Vec3) -> Cell {
        Water.cell_at(position)
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        5.0
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        5.0
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        0.0
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}

#[test]
fn wc7_d01_aligned_unit_distance_translates_then_completes_next_call() {
    let mut f = docking([10.0, 0.0, 10.0], [1.0, 0.0, 0.0], [11.0, 0.0, 10.0], [1.0, 0.0, 0.0]);
    assert_eq!(f.docking_tick(1, 1.0, &Water, &mut None), Ok(true));
    let b = f.get(1).unwrap();
    assert_eq!(b.inherited_position, [11.0, 0.0, 10.0]);
    assert_eq!(b.previous_position, [10.0, 0.0, 10.0]);
    assert_eq!(f.docking_tick(1, 1.0, &Water, &mut None), Ok(false));
}

#[test]
fn wc7_d02_exactly_one_degree_rotates_then_completes_next_call() {
    // Smallest heading whose computed theta is not below 1 degree.
    let mut radians = 1.0_f32.to_radians();
    let theta = |r: f32| {
        let h = math::normalize([r.cos(), 0.0, r.sin()]);
        math::dot(h, [1.0, 0.0, 0.0]).acos().to_degrees()
    };
    while theta(radians) < 1.0 {
        radians = f32::from_bits(radians.to_bits() + 1);
    }
    let heading = [radians.cos(), 0.0, radians.sin()];
    let mut f = docking([10.0, 0.0, 10.0], heading, [10.0, 0.0, 10.0], [1.0, 0.0, 0.0]);
    assert_eq!(f.docking_tick(1, 1.0, &Water, &mut None), Ok(true));
    let b = f.get(1).unwrap();
    assert!(b.inherited_heading[2].abs() < 1e-4, "{:?}", b.inherited_heading);
    assert_eq!(b.inherited_position, [10.0, 0.0, 10.0]);
    assert_eq!(f.docking_tick(1, 1.0, &Water, &mut None), Ok(false));
}

#[test]
fn wc7_d03_inside_both_thresholds_returns_false_before_motion_or_snapshots() {
    let r = 0.5_f32.to_radians();
    let mut f = docking(
        [10.0, 0.0, 10.0],
        [r.cos(), 0.0, r.sin()],
        [10.5, 0.0, 10.0],
        [1.0, 0.0, 0.0],
    );
    f.get_mut(1).unwrap().previous_position = [7.0; 3];
    assert_eq!(f.docking_tick(1, 1.0, &Water, &mut None), Ok(false));
    let b = f.get(1).unwrap();
    assert!(b.events.is_empty());
    assert_eq!(b.previous_position, [7.0; 3]);
    assert_eq!(b.inherited_position, [10.0, 0.0, 10.0]);
}

#[test]
fn wc7_d04_on_land_pushes_72_and_still_translates() {
    let mut f = docking([10.0, 0.0, 10.0], [1.0, 0.0, 0.0], [12.0, 0.0, 10.0], [1.0, 0.0, 0.0]);
    assert_eq!(f.docking_tick(1, 1.0, &Land, &mut None), Ok(true));
    let b = f.get(1).unwrap();
    assert_eq!(b.top_state(), Some(state::IMMEDIATE_REMOVE));
    assert_eq!(b.inherited_position, [11.0, 0.0, 10.0]);
}

#[test]
fn wc7_d05_range_equality_uses_unclamped_final_approach() {
    let mut f = docking([5.0, 0.0, 5.0], [1.0, 0.0, 0.0], [95.0, 0.0, 5.0], [1.0, 0.0, 0.0]);
    assert_eq!(f.get(1).unwrap().drive.0[37], 90.0);
    assert_eq!(f.get(1).unwrap().drive.0[35], 100.0);
    assert_eq!(f.docking_tick(1, 200.0, &Water, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().inherited_position, [95.0, 0.0, 5.0]);
}

#[derive(Default)]
struct Manager {
    berth: bool,
    calls: Vec<&'static str>,
}
impl DockManager for Manager {
    fn take_berth(&mut self, _: &Resource, _: BoatId) -> bool {
        self.calls.push("berth");
        self.berth
    }
    fn take_waiting_spot(&mut self, _: &Resource, _: BoatId) -> bool {
        self.calls.push("waiting");
        true
    }
    fn done_docking(&mut self, _: &Resource, _: BoatId) {
        self.calls.push("done");
    }
    fn may_stay(&mut self, _: BoatId) -> bool {
        unreachable!()
    }
}

#[test]
fn wc7_d06_queued_with_occupied_berth_retains_state_75() {
    let mut f = docking([10.0, 0.0, 10.0], [1.0, 0.0, 0.0], [12.0, 0.0, 10.0], [1.0, 0.0, 0.0]);
    f.push_state(1, State::new(state::QUEUE), &mut None).unwrap();
    let mut manager = Manager::default();
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.dock_requests(1, &mut service).unwrap();
    assert_eq!(manager.calls, ["berth"]);
    let b = f.get(1).unwrap();
    assert_eq!(b.record.base.states.len(), 1);
    assert_eq!(b.top_state(), Some(state::QUEUE));
    assert!(!b.events.contains(&Event::Remove(false)));
}

#[test]
fn wc7_d07_request_radius_is_strict() {
    let mut f = docking([0.0, 0.0, 10.0], [1.0, 0.0, 0.0], [300.0, 0.0, 10.0], [1.0, 0.0, 0.0]);
    assert_eq!(f.get(1).unwrap().drive.0[36], 300.0);
    let mut manager = Manager::default();
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.dock_requests(1, &mut service).unwrap();
    assert!(manager.calls.is_empty());
    f.get_mut(1).unwrap().inherited_position[0] = 0.001;
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.dock_requests(1, &mut service).unwrap();
    assert_eq!(manager.calls, ["berth", "waiting"]);
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::QUEUE));
}

fn undocking(position: Vec3, occupant: Option<u64>, origin: Vec3) -> Fleet {
    let mut b = boat();
    b.position = position;
    b.inherited_position = position;
    b.neighbor = Some(NeighborRoute {
        occupant: occupant.map(|id| Arc::new(Resource(id))),
        position: origin,
        cells: Arc::from(vec![Cell { x: 0, z: 0 }]),
        next: 0,
    });
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    f
}

struct Unnavigable;
impl Terrain for Unnavigable {
    fn contains(&self, cell: Cell) -> bool {
        Water.contains(cell)
    }
    fn cell_at(&self, position: Vec3) -> Cell {
        Water.cell_at(position)
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        -10.0
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        -10.0
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        0.0
    }
    fn navigable(&self, _: Cell, radius: u32, lot: Option<&Resource>) -> bool {
        assert_eq!((radius, lot), (2, Some(&Resource(7))));
        false
    }
}

#[test]
fn wc7_u01_impossible_unnavigable_or_missing_occupant_return_false() {
    let rectangle = [Cell { x: 10, z: 10 }; 4];
    let mut f = undocking([12.0, 0.0, 10.0], Some(7), [10.0, 0.0, 10.0]);
    f.get_mut(1).unwrap().record.base.automata_flags |= 1;
    assert_eq!(f.undocking_tick(1, 1.0, rectangle, &Water, &mut None), Ok(false));
    let mut f = undocking([12.0, 0.0, 10.0], Some(7), [10.0, 0.0, 10.0]);
    assert_eq!(f.undocking_tick(1, 1.0, rectangle, &Unnavigable, &mut None), Ok(false));
    let mut f = undocking([12.0, 0.0, 10.0], None, [10.0, 0.0, 10.0]);
    assert_eq!(f.undocking_tick(1, 1.0, rectangle, &Water, &mut None), Ok(false));
    for f in [&f] {
        assert!(f.get(1).unwrap().events.is_empty());
    }
}

#[test]
fn wc7_u02_u03_distance_equality_and_short_target_return_false_without_motion() {
    let rectangle = [Cell { x: 10, z: 10 }; 4];
    // radius 5: exactly 10 away.
    let mut f = undocking([20.0, 0.0, 10.0], Some(7), [10.0, 0.0, 10.0]);
    assert_eq!(f.undocking_tick(1, 1.0, rectangle, &Water, &mut None), Ok(false));
    assert_eq!(f.get(1).unwrap().position, [20.0, 0.0, 10.0]);
    // 9.5 away: outward target displacement 0.5.
    let mut f = undocking([19.5, 0.0, 10.0], Some(7), [10.0, 0.0, 10.0]);
    assert_eq!(f.undocking_tick(1, 1.0, rectangle, &Water, &mut None), Ok(false));
    assert_eq!(f.get(1).unwrap().position, [19.5, 0.0, 10.0]);
    // 9 away: displacement 1 moves by dt*p[39] and returns true.
    let mut f = undocking([19.0, 0.0, 10.0], Some(7), [10.0, 0.0, 10.0]);
    assert_eq!(f.undocking_tick(1, 0.5, rectangle, &Water, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().position, [19.5, 0.0, 10.0]);
}

#[test]
fn wc7_b03_clear_dock_requests_is_strict_and_releases_with_manager() {
    let a = station(1, [300.0, 0.0, 0.0], [0.0; 3], [1.0, 0.0, 0.0]);
    let mut b = boat();
    b.current_station = Some(a);
    b.held_berth = true;
    b.docking_in_progress = true;
    b.record.base.automata_flags |= 0x1000_0000;
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    let mut manager = Manager::default();
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.clear_dock_requests(1, &mut service).unwrap();
    assert!(manager.calls.is_empty());
    let b = f.get(1).unwrap();
    assert!(b.held_berth && b.docking_in_progress);
    f.get_mut(1).unwrap().inherited_position = [-0.01, 0.0, 0.0];
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.clear_dock_requests(1, &mut service).unwrap();
    assert_eq!(manager.calls, ["done"]);
    let b = f.get(1).unwrap();
    assert!(!b.held_berth && !b.docking_in_progress);
    assert_eq!(b.record.base.automata_flags & 0x1000_0000, 0);
    assert!(b.current_station.is_some());
}

#[test]
fn wc7_r01_direction_helper_uses_signed_remainder() {
    let values: Vec<i32> = (0..8).map(|k| assist_pick_new_dir(0, k)).collect();
    assert_eq!(values, [0, 1, -1, 2, -2, 3, -3, 4]);
    let values: Vec<i32> = (0..8).map(|k| assist_pick_new_dir(2, k)).collect();
    assert_eq!(values, [2, 3, 1, 4, 0, 5, -1, 6]);
    // The picker skips the reverse direction; negative values have no tried bit.
    let picked: Vec<i32> = candidate_directions(0, 0xFF & !0b11).collect();
    assert_eq!(picked, [0, 1, -1, -2, -3]);
}

#[test]
fn wc7_t04_zero_gap_neither_moves_nor_turns_but_sets_water_height() {
    let mut f = Fleet::default();
    let mut tug = boat();
    tug.position = [20.0, 0.0, 0.0];
    tug.heading = [1.0, 0.0, 0.0];
    f.insert(1, tug).unwrap();
    let mut b = boat();
    // Follower front endpoint 15 behind the tug's back point, heading 90 degrees wrong.
    b.position = [-5.0, 0.0, 0.0];
    b.heading = [1.0, 0.0, 0.0];
    b.inherited_position = [0.0, 3.0, 0.0];
    b.inherited_heading = [0.0, 0.0, 1.0];
    b.tug = Some(1);
    f.insert(2, b).unwrap();
    let rear = f.get(1).unwrap().rear_endpoint();
    let front = f.get(2).unwrap().front_endpoint();
    assert_eq!(math::length(math::sub(rear, front)), 15.0);
    assert_eq!(f.following_tick(2, 1000.0, &Water), Ok(true));
    let b = f.get(2).unwrap();
    assert_eq!(b.inherited_position, [0.0, 0.0, 0.0]);
    assert_eq!(b.inherited_heading, [0.0, 0.0, 1.0]);
    assert!(b.events.is_empty());
}
