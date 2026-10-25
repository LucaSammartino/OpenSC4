// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::sync::{Arc, Mutex};

struct TerrainProbe {
    inside: bool,
    land: f32,
    water: f32,
    navigable: bool,
    calls: Mutex<Vec<&'static str>>,
}
impl Default for TerrainProbe {
    fn default() -> Self {
        Self {
            inside: true,
            land: -10.0,
            water: 0.0,
            navigable: true,
            calls: Mutex::default(),
        }
    }
}
impl Terrain for TerrainProbe {
    fn contains(&self, _: Cell) -> bool {
        self.calls.lock().unwrap().push("inside");
        self.inside
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        Cell { x: 17, z: 19 }
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        panic!("OnLand must sample the cell, not world terrain")
    }
    fn terrain_height_at_cell(&self, cell: Cell) -> f32 {
        assert_eq!(cell, Cell { x: 17, z: 19 });
        self.calls.lock().unwrap().push("cell-height");
        self.land
    }
    fn water_height(&self, _: Vec3) -> f32 {
        self.calls.lock().unwrap().push("water");
        self.water
    }
    fn sea_level(&self) -> f32 {
        self.water
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        self.navigable
    }
}
struct Cells;
impl SpatialServices for Cells {
    fn centre(&self, cell: Cell) -> Vec3 {
        [cell.x as f32, 999.0, cell.z as f32]
    }
    fn adjust_segment(&mut self, _: Vec3, _: &mut Vec3, _: u32) {
        unreachable!()
    }
}

#[test]
fn wc3_on_land_samples_cell_height_only_inside_with_strict_water_comparison() {
    let mut probe = TerrainProbe {
        inside: false,
        ..TerrainProbe::default()
    };
    let b = boat();
    assert!(!b.on_land(&probe));
    assert_eq!(*probe.calls.lock().unwrap(), ["inside"]);
    probe.inside = true;
    probe.land = 0.0;
    probe.calls.lock().unwrap().clear();
    assert!(!b.on_land(&probe));
    assert_eq!(*probe.calls.lock().unwrap(), ["inside", "cell-height", "water"]);
    probe.land = 0.01;
    assert!(b.on_land(&probe));
}

#[test]
fn missing_group_reloads_all_defaults_and_ignores_every_drive_channel() {
    let mut b = boat();
    b.group = None;
    b.drive.0.fill(123.0);
    b.load_drive_parameters();
    assert_eq!(b.drive, DriveParameters::default());
    assert_eq!(b.select_gear(100.0, 200.0), None);
    assert_eq!(b.sound_value(), None);
    let constructed = Watercraft::new(b.record.clone(), None, 1.0);
    assert_eq!(constructed.drive, DriveParameters::default());
    assert_eq!(constructed.group_id(), None);
}

#[test]
fn wc7_r07_nullable_neighbor_and_water_level_skip_consume_equality_and_beyond() {
    let cells: Arc<[Cell]> = (3..6).map(|x| Cell { x, z: 0 }).collect::<Vec<_>>().into();
    let route = NeighborRoute {
        occupant: Some(Arc::new(Resource(42))),
        position: [0.0, 10.0, 0.0],
        cells,
        next: 100,
    };
    let probe = TerrainProbe {
        water: 10.0,
        ..TerrainProbe::default()
    };
    let mut b = boat();
    b.radius = 1.0;
    b.set_neighbor_route_core(route.clone(), &probe, &Cells);
    assert_eq!(b.neighbor.as_ref().unwrap().next, 2);
    assert_eq!(*probe.calls.lock().unwrap(), ["water", "water"]);
    let mut null = route;
    null.occupant = None;
    probe.calls.lock().unwrap().clear();
    b.set_neighbor_route_core(null, &probe, &Cells);
    assert_eq!(b.neighbor.as_ref().unwrap().next, 0);
    assert!(probe.calls.lock().unwrap().is_empty());
}

fn station(id: u64, lot: Vec3, berth: Vec3, cells: Arc<[Cell]>) -> Arc<Station> {
    Arc::new(Station {
        lot: Arc::new(Resource(id)),
        occupant: Arc::new(Resource(id + 10)),
        lot_position: lot,
        position: berth,
        direction: [1.0, 0.0, 0.0],
        route: cells,
    })
}
#[derive(Default)]
struct Berths(Vec<(bool, u64)>);
impl DockManager for Berths {
    fn take_berth(&mut self, lot: &Resource, _: BoatId) -> bool {
        self.0.push((true, lot.0));
        true
    }
    fn take_waiting_spot(&mut self, _: &Resource, _: BoatId) -> bool {
        unreachable!()
    }
    fn done_docking(&mut self, lot: &Resource, _: BoatId) {
        self.0.push((false, lot.0));
    }
    fn may_stay(&mut self, _: BoatId) -> bool {
        true
    }
}

#[test]
fn wc7_b01_b04_requests_and_cleanup_use_lot_position_not_berth_or_runtime() {
    let a = station(1, [1000.0, 0.0, 0.0], [10.0, 0.0, 0.0], Arc::from([]));
    let dest = station(2, [20.0, 0.0, 0.0], [9000.0, 0.0, 0.0], Arc::from([]));
    let mut b = boat();
    b.set_station_information([a.clone(), dest], true);
    b.held_berth = false;
    b.inherited_position = [20.0, 0.0, 0.0];
    b.position = [9999.0; 3];
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    let mut manager = Berths::default();
    f.dock_requests(1, &mut Some(&mut manager)).unwrap();
    assert_eq!(manager.0, [(true, 2)]);
    let b = f.get_mut(1).unwrap();
    assert!(Arc::ptr_eq(b.current_station.as_ref().unwrap(), &a));
    assert!(!b.held_berth);
    b.held_berth = true;
    b.inherited_position = [1000.0 + b.drive.0[36], 0.0, 0.0];
    f.clear_dock_requests(1, &mut Some(&mut manager)).unwrap();
    assert!(f.get(1).unwrap().held_berth);
    f.get_mut(1).unwrap().inherited_position[0] += 1.0;
    f.clear_dock_requests(1, &mut None).unwrap();
    assert!(f.get(1).unwrap().held_berth);
    assert!(!f.get(1).unwrap().docking_in_progress);
    f.clear_dock_requests(1, &mut Some(&mut manager)).unwrap();
    assert_eq!(manager.0, [(true, 2), (false, 1)]);
    assert!(!f.get(1).unwrap().held_berth);
    assert!(Arc::ptr_eq(f.get(1).unwrap().current_station.as_ref().unwrap(), &a));
}

#[test]
fn wc7_r08_b02_ferry_fill_preserves_buffer_and_used_count_at_exhaustion() {
    let cells: Arc<[Cell]> = (0..15).map(|x| Cell { x, z: 0 }).collect::<Vec<_>>().into();
    let a = station(1, [0.0; 3], [0.0; 3], cells);
    let mut b = boat();
    b.ferry = true;
    b.initialize_current_route(a.clone());
    assert!(b.held_berth);
    assert_eq!(b.used_cells, 2);
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    assert!(f.fill_route_points(1, &Water, &mut None).unwrap());
    let b = f.get_mut(1).unwrap();
    assert_eq!(b.travel.len(), 10);
    assert_eq!(b.travel[0].cell.x, 2);
    assert_eq!(b.travel[9].cell.x, 11);
    assert_eq!(b.used_cells, 12);
    b.travel.clear();
    assert!(f.fill_route_points(1, &Water, &mut None).unwrap());
    assert_eq!(f.get(1).unwrap().travel.len(), 3);
    assert!(f.fill_route_points(1, &Water, &mut None).unwrap());
    f.get_mut(1).unwrap().travel.clear();
    assert!(!f.fill_route_points(1, &Water, &mut None).unwrap());
    let b = f.get_mut(1).unwrap();
    assert!(b.events.is_empty());
    assert!(Arc::ptr_eq(b.current_station.as_ref().unwrap(), &a));
    b.initialize_current_route(a);
    assert_eq!(b.used_cells, 15);
    assert!(!b.route_started);
}

#[test]
fn compressed_route_coordinates_wrap_as_signed_sixteen_bit_cells() {
    let mut bytes = vec![1, 255, 0];
    for _ in 0..129 {
        bytes.extend_from_slice(&[0x41, 255]);
        bytes.extend_from_slice(&[0xaa; 64]);
    }
    let reader = RouteReader::from_bytes(&bytes).unwrap();
    assert_eq!(reader.cells().len(), 1 + 129 * 255);
    assert_eq!(
        reader.cells().last().unwrap().x,
        (255u16.wrapping_add(129 * 255)) as i16 as i32
    );
}

#[test]
fn undocking_missing_occupant_blocked_and_land_continue_have_separate_results() {
    let mut b = boat();
    b.position = [1.0, 0.0, 0.0];
    b.radius = 5.0;
    b.neighbor = Some(NeighborRoute {
        occupant: None,
        position: [0.0; 3],
        cells: Arc::from([]),
        next: 0,
    });
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    let rectangle = [Cell { x: 1, z: 1 }; 4];
    assert!(!f.undocking_tick(1, 1.0, rectangle, &Water, &mut None).unwrap());
    f.get_mut(1).unwrap().neighbor.as_mut().unwrap().occupant = Some(Arc::new(Resource(1)));
    let mut probe = TerrainProbe {
        navigable: false,
        ..TerrainProbe::default()
    };
    assert!(!f.undocking_tick(1, 1.0, rectangle, &probe, &mut None).unwrap());
    probe.navigable = true;
    probe.land = 1.0;
    assert!(f.undocking_tick(1, 1.0, rectangle, &probe, &mut None).unwrap());
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::IMMEDIATE_REMOVE));
    assert_eq!(f.get(1).unwrap().position, [2.0, 0.0, 0.0]);
    f.get_mut(1).unwrap().record.base.states = vec![State::new(state::SINK)];
    assert!(!f.undocking_tick(1, 1.0, rectangle, &Water, &mut None).unwrap());
}

#[test]
fn debug_text_is_borrowed_from_one_shared_object_and_replaced_by_the_next_boat() {
    let a = boat();
    let mut b = boat();
    b.speed = 123.0;
    let first = a.borrowed_debug_extra_text([1, 2], Cell { x: 3, z: 4 });
    let address = (&*first as *const String) as usize;
    assert!(first.contains("Speed: 0.00"));
    drop(first);
    let second = b.borrowed_debug_extra_text([5, 6], Cell { x: 7, z: 8 });
    assert_eq!((&*second as *const String) as usize, address);
    assert!(second.contains("Speed: 123.00"));
    assert!(second.contains("(FT,MT): (5,6)"));
}

#[test]
fn wc7_t01_t02_first_geometric_tow_candidate_radius_equality_and_filter_rejection() {
    for (first_distance, first_eligible, expected) in
        [(15.0, true, Some(2)), (20.0, true, Some(3)), (15.0, false, None)]
    {
        let mut f = Fleet::default();
        let mut tug = boat();
        tug.tow_armed = true;
        f.insert(1, tug).unwrap();
        for (id, distance, eligible) in [(2, first_distance, first_eligible), (3, 1.0, true)] {
            let mut candidate = boat();
            candidate.position = [distance, 0.0, 0.0];
            candidate.radius = 10.0;
            candidate.group = Some(Arc::new(Group {
                id: if eligible { 0x4417 } else { 0x4400 },
                parameters: None,
                aems_value: None,
                parent: None,
            }));
            f.insert(id, candidate).unwrap();
        }
        f.do_message(
            1,
            Message::TowLanding(Some([0.0; 3])),
            &mut Some(&mut Berths::default()),
        )
        .unwrap();
        assert_eq!(f.get(1).unwrap().towed, expected);
        assert!(!f.get(1).unwrap().tow_armed);
    }
}

#[test]
fn tow_message_rejected_first_state_does_not_retry_and_unarmed_messages_forward() {
    for rejected in [
        state::RUN_AGROUND,
        state::SINK,
        state::IMMEDIATE_REMOVE,
        state::SLOW_REMOVE,
    ] {
        let mut f = Fleet::default();
        let mut tug = boat();
        tug.tow_armed = true;
        f.insert(1, tug).unwrap();
        for id in [2, 3] {
            let mut candidate = boat();
            candidate.radius = 10.0;
            candidate.group = Some(Arc::new(Group {
                id: 0x4417,
                parameters: None,
                aems_value: None,
                parent: None,
            }));
            if id == 2 {
                candidate.record.base.states = vec![State::new(rejected)];
            }
            f.insert(id, candidate).unwrap();
        }
        f.do_message(
            1,
            Message::TowLanding(Some([0.0; 3])),
            &mut Some(&mut Berths::default()),
        )
        .unwrap();
        assert_eq!(f.get(1).unwrap().towed, None);
        assert_eq!(
            f.do_message(1, Message::TowLanding(None), &mut None).unwrap(),
            MessageDisposition::ForwardToAutomaton
        );
    }
}

#[test]
fn wc7_t03_t05_endpoint_gap_zero_time_slack_and_missing_tug() {
    for (length, dt) in [(25.0, 0.0), (25.0, -10.0), (15.0, 1000.0)] {
        let mut f = Fleet::default();
        let mut follower = boat();
        follower.radius = 5.0;
        follower.tug = Some(2);
        follower.position = [0.0; 3];
        follower.heading = [1.0, 0.0, 0.0];
        follower.inherited_heading = [1.0, 0.0, 0.0];
        follower.inherited_position = [0.0; 3];
        let mut tug = boat();
        tug.radius = 2.0;
        tug.position = [length + 7.0, 0.0, 0.0];
        tug.heading = [1.0, 0.0, 0.0];
        f.insert(1, follower).unwrap();
        f.insert(2, tug).unwrap();
        assert!(f.following_tick(1, dt, &Water).unwrap());
        let b = f.get(1).unwrap();
        if length == 25.0 {
            assert_eq!(b.position, [10.0, 0.0, 0.0]);
            // Negative dt is retained by the signed minimum, not clamped.
            let angle = if dt < 0.0 {
                dt * b.drive.0[38] * 0.017_453_292
            } else {
                0.0
            };
            assert!((b.heading[0] - angle.cos()).abs() < 1e-5);
            assert!((b.heading[2] + angle.sin()).abs() < 1e-5);
        } else {
            assert_eq!(b.position, [0.0; 3]);
            assert!(b.events.is_empty());
        }
        let b = f.get_mut(1).unwrap();
        b.tug = None;
        b.inherited_position[1] = 42.0;
        assert!(!f.following_tick(1, 100.0, &Water).unwrap());
        assert_eq!(f.get(1).unwrap().inherited_position[1], 42.0);
    }
}
