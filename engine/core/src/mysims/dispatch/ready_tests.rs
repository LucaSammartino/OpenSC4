// SPDX-License-Identifier: GPL-3.0-or-later
use super::scripted::DispatchFixture as D;
use super::*;
use std::cell::Cell;

#[derive(Default)]
struct Services {
    calls: Cell<u32>,
    travel: Cell<u32>,
    released: RefCell<Vec<u64>>,
}
impl TripServices for Services {
    fn dispatch_next_leg(&self, _: i32, _: Option<u64>) -> Option<u32> {
        self.calls.set(self.calls.get() + 1);
        Some(self.travel.get())
    }
    fn release_automaton(&self, occupant: u64) {
        self.released.borrow_mut().push(occupant);
    }
}

fn marker(mode: u8) -> RouteData {
    RouteData {
        x: 2,
        z: 1,
        travel_type: mode,
    }
}
fn fixture(mode: u32, flags: u32) -> D {
    let f = D::new();
    f.d.edit(1, |st| {
        st.search.state = 2;
        st.search.flags = flags;
        st.minutes = 100;
        st.travel_type = mode;
        st.tuning.idle_minimum = 5;
        st.tuning.switch_minimum = 7;
        st.current.set_cell_position(1, 1);
        st.current.occupant = Some(99);
        st.search.home = st.current;
        st.destination.set_cell_position(2, 1);
        st.automata_occupant = Some(88);
        st.queue.push_back(st.destination);
    });
    f
}

#[test]
fn ms18_06_all_42_ordinary_mode_pairs() {
    for source in [0, 1, 2, 3, 6, 7, 8] {
        for target in [0, 1, 2, 3, 6, 7, 8] {
            if source == target {
                continue;
            }
            let f = fixture(source, 0x20);
            let services = Services::default();
            assert_eq!(
                f.d.route_ready_update(1, marker(target as u8), &services),
                TripOutcome::Accepted
            );
            let st = f.d.get_state(1);
            assert_eq!(st.search.state, 2);
            assert_eq!(st.travel_type, source);
            assert_eq!(st.current.occupant, None);
            assert_eq!(st.automata_occupant, None);
            assert_eq!(*services.released.borrow(), vec![88]);
            assert_eq!(services.calls.get(), 0);
            assert_eq!(
                (st.current.deadline(), st.search.flags),
                if target <= 1 { (105, 0x2401) } else { (107, 0x10) }
            );
            assert!(f.take().iter().all(|n| n.1 == 0x4BB2D020));
        }
    }
}

#[test]
fn ms18_07_same_mode_success_still_runs_switch_fallback() {
    let f = fixture(1, 0x420);
    let services = Services::default();
    services.travel.set(1);
    f.d.route_ready_update(1, marker(1), &services);
    let st = f.d.get_state(1);
    assert_eq!(
        (st.search.state, st.travel_type, st.current.deadline(), st.search.flags),
        (2, 0xFE, 105, 0x2603)
    );
    assert!(st.queue.is_empty());
    assert_eq!(
        f.take().iter().map(|n| (n.2, n.3)).collect::<Vec<_>>(),
        vec![
            (0x420, 0x620),
            (0x620, 0x220),
            (0x220, 0x2621),
            (0x2621, 0x2601),
            (0x2601, 0x2603)
        ]
    );
}

#[test]
fn ms18_08_sentinels_select_failure_before_fallback_flags() {
    for source in [0xFE, 0xFF] {
        let f = fixture(source, 0x20);
        f.d.route_ready_update(1, marker(1), &Services::default());
        let st = f.d.get_state(1);
        assert_eq!(
            (st.search.state, st.travel_type, st.search.flags, st.current.deadline()),
            (-1, 0xFE, 0x2403, 105)
        );
        assert!(st.queue.is_empty());
        assert_eq!(f.take()[0].1, 0x4BB2CFC0);
    }
}

#[test]
fn ms18_09_10_delay_equality_and_stuck_notice_order() {
    for (deadline, result) in [(100, 1), (99, 1), (99, 0xFF)] {
        let f = fixture(1, 4);
        f.d.edit(1, |st| st.current.set_deadline(deadline));
        let s = Services::default();
        s.travel.set(result);
        f.d.route_ready_update(1, marker(1), &s);
        let st = f.d.get_state(1);
        if deadline == 100 {
            assert_eq!(s.calls.get(), 0);
            assert_eq!(st.search.flags, 4);
            assert!(f.take().is_empty());
        } else if result == 1 {
            assert_eq!((st.travel_type, st.search.flags), (1, 0x200));
            assert_eq!(
                f.take().iter().map(|n| (n.2, n.3)).collect::<Vec<_>>(),
                vec![(4, 0), (0, 0x200)]
            );
        } else {
            assert_eq!((st.search.state, st.travel_type, st.search.flags), (2, 0xFF, 0x444));
            assert_eq!(
                f.take().iter().map(|n| n.1).collect::<Vec<_>>(),
                vec![0x4BB2D020, 0xEBA06C12]
            );
            f.d.route_ready_update(1, marker(1), &s);
            assert_eq!(f.d.get_state(1).search.state, -1);
        }
    }
}

#[test]
fn ms18_12_retry_is_always_false_and_preserves_location() {
    let f = fixture(1, 0);
    for local in [true, false] {
        for index in [0, 1, 2, 99] {
            let mut location = f.d.get_state(1).current;
            location.in_city = local;
            let before = location;
            assert!(!f.d.update_neighbor_location(index, &mut location));
            assert_eq!(location, before);
        }
    }
}

#[test]
fn ms20_02_only_assigned_constructor_fields_are_promised() {
    let st = DispatchState::default();
    assert_eq!(
        (
            st.tuning.approach_distance,
            st.tuning.proximity,
            st.tuning.idle_minimum,
            st.tuning.switch_minimum
        ),
        (10, 10, 5, 5)
    );
    assert_eq!(
        (
            st.change_counter,
            st.previous_change_counter,
            st.tracking_value,
            st.wait_timestamp,
            st.route_marker
        ),
        (0, 0, u32::MAX, 0, -1)
    );
    for location in [
        st.current,
        st.search.home,
        st.destination,
        st.search.commute_destination,
    ] {
        assert_eq!(location, DispatchLocation::default());
    }
    for (mode, group) in [
        (0, 0x4107),
        (1, 0x420B),
        (2, 0x4200),
        (3, 0x4215),
        (6, 0x4217),
        (7, 0x422C),
        (8, 0x422E),
    ] {
        assert_eq!(model_selection(mode, false, true, true).0, group);
    }
    assert_eq!(model_selection(0, false, true, false), (0x4149, 1));
    assert_eq!(model_selection(0xFE, true, false, false), (0x414A, 2));
    assert_eq!(model_selection(5, false, true, true), (0, -1));
    assert_eq!(fixture(1, 0).d.model_offset(1, 2), -1);
}
