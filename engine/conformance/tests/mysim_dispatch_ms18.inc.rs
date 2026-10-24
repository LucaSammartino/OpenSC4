// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use fc4_core::mysims::dispatch::{model_selection, scripted::DispatchFixture as D};
use std::cell::Cell;

#[derive(Default)]
struct Services {
    calls: Cell<u32>,
    travel: Cell<u32>,
    released: RefCell<Vec<u64>>,
    position: Cell<(f32, f32)>,
    snap: Cell<Option<bool>>,
    idle_result: Cell<Option<u32>>,
    offsets: RefCell<Vec<i32>>,
}
impl TripServices for Services {
    fn idle_model_kind(&self, _: Option<u64>) -> Option<i32> {
        Some(7)
    }
    fn snap_to_road(&self, _: &mut DispatchLocation) -> Option<bool> {
        self.snap.get()
    }
    fn dispatch_idle_fallback(&self, _: i32) -> Option<u32> {
        self.idle_result.get()
    }
    fn apply_model_offset(&self, _: u64, offset: i32) {
        self.offsets.borrow_mut().push(offset);
    }
    fn cancel_automaton(&self, _: Option<u64>) -> Option<bool> {
        Some(true)
    }
    fn automaton_position(&self, _: u64) -> Option<(f32, f32)> {
        Some(self.position.get())
    }
    fn is_driveable(&self, _: u64) -> bool {
        true
    }
    fn attach_player(&self, _: u64) -> TripOutcome {
        TripOutcome::Accepted
    }
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
    edit(&f, |st| {
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
        edit(&f, |st| st.current.set_deadline(deadline));
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

#[test]
fn ms18_18_callback_sender_identity() {
    let f = fixture(1, 0);
    let s = Services::default();
    let before = f.d.get_state(1);
    f.d.automaton_callback(1, 2, Some(89), &s);
    assert_eq!(f.d.get_state(1), before);
    assert!(f.take().is_empty());
    f.d.automaton_callback(1, 2, Some(88), &s);
    assert_eq!(f.d.get_state(1).search.flags, 0x401);
    assert_eq!(f.d.get_state(1).search.state, 2);
}

#[test]
fn ms18_19_20_approach_equality_and_pending_entry() {
    let f = fixture(1, 0x8000);
    let s = Services::default();
    s.position.set((48.0, 64.0));
    edit(&f, |st| {
        st.destination.set_cell_position(0, 0);
        st.tuning.approach_distance = 5;
    });
    f.d.automaton_callback(1, 9, Some(88), &s);
    let st = f.d.get_state(1);
    assert_eq!((st.current.cell, st.search.flags), ((3, 4), 0));
    edit(&f, |st| {
        st.current.set_cell_position(1, 1);
        st.tuning.approach_distance = 6;
    });
    f.d.automaton_callback(1, 9, Some(88), &s);
    assert!(f.d.check_flags(1, 0x8000));
    edit(&f, |st| st.search.flags = 0x20000);
    f.take();
    f.d.automaton_callback(1, 9, Some(89), &s);
    assert!(f.d.check_flags(1, 0x20000));
    f.d.automaton_callback(1, 9, Some(88), &s);
    assert_eq!(f.d.get_state(1).search.flags, 0x10000);
    assert_eq!(
        f.take().iter().map(|n| (n.2, n.3)).collect::<Vec<_>>(),
        vec![(0x20000, 0), (0, 0x10000)]
    );
}

#[test]
fn ms18_21_mission_data_guard_and_exit() {
    let f = fixture(1, 0x10000);
    let s = Services::default();
    let before = f.d.get_state(1);
    f.d.mission_callback(1, 0, &s);
    assert_eq!(f.d.get_state(1), before);
    f.d.mission_callback(1, 1, &s);
    let st = f.d.get_state(1);
    assert_eq!((st.search.state, st.search.flags, st.travel_type), (-1, 0, 0xFF));
    assert_eq!(st.current, st.search.home);
    assert!(st.queue.is_empty());
    assert_eq!(
        f.take().iter().map(|n| n.1).collect::<Vec<_>>(),
        vec![0x4BB2CFC0, 0x4BB2D020]
    );
}

#[test]
fn ms18_11_idle_precedes_stuck_without_synthetic_dispatch() {
    let f = fixture(0xFE, 0x41);
    edit(&f, |st| st.automata_occupant = None);
    let s = Services::default();
    s.snap.set(Some(false));
    assert_eq!(f.d.route_ready_update(1, marker(1), &s), TripOutcome::Accepted);
    assert_eq!(
        (f.d.get_state(1).search.state, f.d.get_state(1).travel_type),
        (-1, 0xFE)
    );
    assert_eq!(s.calls.get(), 0);
    assert_eq!(
        f.take().iter().map(|n| n.1).collect::<Vec<_>>(),
        vec![0xEBA06C12, 0x4BB2CFC0]
    );
}

#[test]
fn ms18_17_duplicate_idle_notices_survive_fallback_failure() {
    let f = fixture(1, 0x2001);
    edit(&f, |st| st.automata_occupant = None);
    let s = Services::default();
    s.snap.set(Some(true));
    s.idle_result.set(Some(0xFF));
    s.travel.set(1);
    f.d.route_ready_update(1, marker(1), &s);
    assert_eq!(
        f.take().iter().take(3).map(|n| n.1).collect::<Vec<_>>(),
        vec![0xEBA06BD0, 0xEBA06BD0, 0xEBA06C12]
    );
}

#[test]
fn idle_deadline_equality_retains_wait_and_only_positive_offsets_apply() {
    let f = fixture(0xFE, 0x2001);
    let s = Services::default();
    edit(&f, |st| {
        st.route_marker = 2;
        st.current.set_deadline(100);
    });
    for offset in [-1, 0, 1] {
        edit(&f, |st| {
            st.search.flags = 0x2001;
            st.models.insert(
                2,
                (
                    fc4_core::stream::ResKey {
                        type_id: 1,
                        group: 2,
                        instance: 3,
                    },
                    offset,
                ),
            );
        });
        f.d.route_ready_update(1, marker(0xFE), &s);
        assert_eq!(f.d.get_state(1).current.deadline(), 100);
        assert_eq!(s.calls.get(), 0);
    }
    assert_eq!(*s.offsets.borrow(), vec![1]);
}

fn edit(f: &D, change: impl FnOnce(&mut DispatchState)) {
    let mut state = f.d.get_state(1);
    change(&mut state);
    restore(f, &state);
}

#[test]
fn ms18_13_inclusive_fe_equality_copies_active_timing() {
    let f = fixture(1, 0x20);
    edit(&f, |st| {
        st.current.set_cell_position(25, 0);
        st.search.home = st.current;
        st.destination.set_cell_position(0, 0);
        st.destination.set_duration(9);
        st.destination.set_deadline(777);
        st.tuning.proximity = 5;
    });
    f.d.route_ready_update(1, marker(0xFE), &Services::default());
    let st = f.d.get_state(1);
    assert_eq!((st.search.state, st.travel_type, st.search.flags), (3, 0xFE, 2));
    assert_eq!((st.current.duration(), st.current.deadline()), (9, 777));
    assert_eq!(f.take()[0].1, 0xEBA06BD4);
}

#[test]
fn ms18_14_invalid_active_exposes_duration_write() {
    let f = fixture(1, 0x20);
    edit(&f, |st| {
        st.destination = DispatchLocation::default();
        st.current.set_duration(9);
        st.current.set_deadline(777);
    });
    f.d.route_ready_update(1, marker(0xFE), &Services::default());
    let st = f.d.get_state(1);
    assert_eq!(
        (st.search.state, st.current.duration(), st.current.deadline()),
        (3, 109, 777)
    );
    assert_eq!(f.take()[0].1, 0xEBA06BD4);
}

#[test]
fn ms18_15_far_fe_starts_idle_without_arrival() {
    let f = fixture(1, 0x20);
    edit(&f, |st| {
        st.current.set_cell_position(0, 0);
        st.destination.set_cell_position(6, 0);
        st.tuning.proximity = 2;
    });
    f.d.route_ready_update(1, marker(0xFE), &Services::default());
    let st = f.d.get_state(1);
    assert_eq!(
        (st.search.state, st.travel_type, st.search.flags, st.current.deadline()),
        (2, 0xFE, 0x2423, 105)
    );
    let notes = f.take();
    assert_eq!(notes[0].1, 0x2BAC398F);
    assert!(notes.iter().all(|n| n.1 != 0xEBA06BD4));
}

#[test]
fn ms18_16_first_boarding_vehicle_failure_does_not_try_second() {
    let f = fixture(1, 0x10);
    edit(&f, |st| st.current.set_deadline(100));
    let services = TransportForeign::default();
    *services.automata.borrow_mut() = Some(vec![
        BoardingAutomaton {
            occupant: 31,
            group: 0x4200,
            has_automaton_interface: true,
        },
        BoardingAutomaton {
            occupant: 32,
            group: 0x4200,
            has_automaton_interface: true,
        },
    ]);
    services.next.set(Some(0xFF));
    assert_eq!(f.d.route_ready_update(1, marker(2), &services), TripOutcome::Rejected);
    assert_eq!(*services.attempts.borrow(), vec![Some(31)]);
    assert_eq!(f.d.get_state(1).travel_type, 0xFF);
    assert_eq!(
        f.take(),
        vec![
            (1, 0x4BB2D020, 0x10, 0x2411),
            (1, 0x4BB2D020, 0x2411, 0x2451),
            (1, 0xEBA06C12, 0, 0),
        ]
    );
}



