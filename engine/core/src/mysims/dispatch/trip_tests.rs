// SPDX-License-Identifier: GPL-3.0-or-later
use super::scripted::DispatchFixture as D;
use super::*;
use crate::stream::{ByteReader, ByteWriter, RefValue, ResKey};
use std::cell::Cell;

#[derive(Default)]
struct Services {
    cancel: Cell<Option<bool>>,
    search: Cell<Option<TripSearch>>,
    registered: RefCell<Vec<bool>>,
    detached: Cell<u32>,
    released: RefCell<Vec<u64>>,
}
impl TripServices for Services {
    fn cancel_automaton(&self, _: Option<u64>) -> Option<bool> {
        self.cancel.get()
    }
    fn is_driveable(&self, _: u64) -> bool {
        true
    }
    fn attach_player(&self, _: u64) -> TripOutcome {
        TripOutcome::Accepted
    }
    fn detach_player(&self, _: Option<u64>) {
        self.detached.set(self.detached.get() + 1);
    }
    fn release_automaton(&self, o: u64) {
        self.released.borrow_mut().push(o);
    }
    fn search_trip(&self, _: i32, _: &DispatchState) -> TripSearch {
        self.search.get().unwrap_or(TripSearch::UnsupportedMs18)
    }
    fn register_deferred_setup(&self, _: i32, register: bool) {
        self.registered.borrow_mut().push(register);
    }
}

struct Refs;
impl DispatchRecordRefs for Refs {
    fn reference(&self, token: u64) -> RefValue {
        RefValue::Host { token, class_id: 123 }
    }
    fn token(&self, reference: RefValue) -> Option<u64> {
        match reference {
            RefValue::Host { token, .. } => Some(token),
            _ => None,
        }
    }
}

fn location(cell: i32, occupant: bool) -> DispatchLocation {
    let mut l = DispatchLocation {
        occupant: occupant.then_some(cell as u64 + 100),
        ..DispatchLocation::default()
    };
    l.set_cell_position(cell, 1);
    l.set_duration(5);
    l
}

#[test]
fn ms17_01_identity_and_request_not_arrival() {
    let f = D::new();
    let s = Services::default();
    let st = f.d.get_state(1);
    assert_eq!(
        (
            st.search.state,
            st.search.prev_state,
            st.search.flags,
            st.minutes,
            st.travel_type
        ),
        (-1, -1, 0, 0, 255)
    );
    f.d.edit(1, |st| {
        st.current = location(1, true);
        st.search.home = location(2, true);
        st.destination = location(3, true);
        st.search.commute_destination = location(4, true);
        st.search.state = 2;
    });
    s.cancel.set(Some(true));
    assert_eq!(f.d.go_to(1, location(5, true), &s), TripOutcome::Accepted);
    let st = f.d.get_state(1);
    assert_eq!(
        (
            st.current.cell.0,
            st.search.home.cell.0,
            st.destination.cell.0,
            st.search.commute_destination.cell.0
        ),
        (1, 2, 5, 4)
    );
    assert_eq!(st.search.state, 2);
    assert!(f.take().is_empty());
}

#[test]
fn ms17_02_order_counters_and_reentrant_owner() {
    let f = D::new();
    let weak = Rc::downgrade(&f.d);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    *f.w.on_notify.borrow_mut() = Some(Rc::new(move |sim, _, _, _| {
        let d = weak.upgrade().unwrap();
        record.borrow_mut().push(d.get_state(sim).search.state);
        d.add_destination_location(sim, location(4, false), 0);
    }));
    f.d.set_state(1, 3);
    f.d.set_state(1, 3);
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0, 2), (1, 0x4BB2CFC0, u32::MAX, 3)]);
    assert_eq!(*seen.borrow(), vec![3, 3]);
    assert_eq!(f.d.get_state(1).change_counter, 2);
    assert_eq!(f.d.get_state(1).queue.len(), 2);
    f.d.report_flags(1, 0, 999);
    f.take();
    f.d.set_state(1, 3);
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0, 2)]);
    f.d.edit(1, |st| st.change_counter = u32::MAX);
    f.d.report_flags(1, 4, 0);
    assert_eq!(f.d.get_state(1).change_counter, 0);
}

#[test]
fn ms17_03_clock_wait_and_strict_dwell() {
    let f = D::new();
    let s = Services::default();
    f.d.minute_message(1);
    assert_eq!(f.d.get_state(1).minutes, 0);
    f.d.init(1);
    f.d.edit(1, |st| {
        st.search.state = 3;
        st.current = location(1, true);
        st.minutes = 100;
        st.current.set_deadline(105);
        st.queue.push_back(location(2, true));
    });
    f.d.minute_message(1);
    assert_eq!(f.d.get_state(1).search.state, 3);
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).queue.len(), 1);
    f.d.minute_message(1);
    f.d.animation_message(1, &s);
    let st = f.d.get_state(1);
    assert_eq!((st.minutes, st.search.state, st.destination.deadline()), (110, 2, 115));
    f.d.edit(1, |st| {
        st.wait_timestamp = 100;
        st.minutes = 115;
    });
    assert_eq!(f.d.wait_value(1), 100);
    f.d.edit(1, |st| st.minutes = 116);
    assert_eq!(f.d.wait_value(1), 0);
    f.d.edit(1, |st| {
        st.wait_timestamp = 100;
        st.search.flags |= 0x10;
    });
    assert_eq!(f.d.wait_value(1), 100);
    f.d.edit(1, |st| st.minutes = i32::MAX);
    f.d.minute_message(1);
    assert_eq!(f.d.get_state(1).minutes, i32::MIN + 4);
}

#[test]
fn ms17_04_tuning_and_signed_boundaries() {
    let f = D::new();
    let s = Services::default();
    f.d.set_tuning(
        1,
        DispatchTuning {
            location_minimum: 30,
            approach_distance: 8,
            switch_minimum: -3,
            idle_minimum: -7,
            ..DispatchTuning::default()
        },
    );
    f.d.go_to(1, location(2, true), &s);
    assert_eq!(f.d.get_state(1).queue[0].duration(), 30);
    f.d.go_to(1, location(2, false), &s);
    assert_eq!(f.d.get_state(1).queue[0].duration(), 5);
    f.d.edit(1, |st| {
        st.destination.set_cell_position(0, 0);
        st.minutes = 100;
    });
    for (cell, near) in [(7, true), (8, false), (9, false), (7, true)] {
        f.d.automaton_cell_changed(1, (cell, 0));
        assert_eq!(f.d.check_flags(1, 0x8000), near);
    }
    assert_eq!(
        f.take()
            .iter()
            .filter(|(_, id, old, new)| *id == 0x4BB2D020 && old & 0x8000 == 0 && new & 0x8000 != 0)
            .count(),
        2
    );
    f.d.report_flags(1, 0x18000, 0);
    f.take();
    f.d.automaton_cell_changed(1, (30, 0));
    assert!(f.d.check_flags(1, 0x8000));
    assert!(f.take().is_empty());
    f.d.begin_located_switch(1);
    let st = f.d.get_state(1);
    assert_eq!((st.wait_timestamp, st.current.deadline()), (100, 97));
    f.d.begin_idle(1);
    assert_eq!(f.d.get_state(1).current.deadline(), 93);
    assert!(!f.d.within_proximity(1, 100, false));
    assert!(f.d.within_proximity(1, 100, true));
    f.d.edit(1, |st| st.tuning.proximity = 46341);
    assert!(!f.d.within_proximity(1, 0, true));
}

#[test]
fn ms17_05_queue_and_deadline() {
    let f = D::new();
    let s = Services::default();
    f.d.edit(1, |st| {
        st.search.state = 3;
        st.minutes = 100;
        st.current.set_deadline(200);
    });
    for cell in [1, 2] {
        assert!(f.d.add_destination_location(1, location(cell, true), 0));
    }
    f.d.add_destination_location(1, location(3, true), 4);
    f.d.add_destination_location(1, location(4, true), 1 | 0x80);
    let st = f.d.get_state(1);
    assert_eq!(st.queue.iter().map(|l| l.cell.0).collect::<Vec<_>>(), vec![4, 3]);
    assert_eq!(st.current.deadline(), 200);
    assert_eq!(f.d.go_to(1, location(5, false), &s), TripOutcome::Accepted);
    let st = f.d.get_state(1);
    assert_eq!((st.queue.len(), st.queue[0].cell.0, st.current.deadline()), (1, 5, 99));
    assert!(f.take().is_empty());
}

#[test]
fn ms17_06_commute_requests() {
    let f = D::new();
    for (hour, duration) in [(7, 5), (8, 540), (17, 0), (18, 5)] {
        assert_eq!(f.d.dispatch_commute(1, true, 2, hour), TripOutcome::Accepted);
        let st = f.d.get_state(1);
        assert_eq!(st.queue.len(), 1);
        assert!(st.queue[0].by_commute);
        assert_eq!(st.queue[0].duration(), duration);
    }
    let old = f.d.get_state(1).queue;
    assert_eq!(f.d.dispatch_commute(1, false, 2, 8), TripOutcome::Rejected);
    for state in [0, 4, 5, 6] {
        assert_eq!(f.d.dispatch_commute(1, true, state, 8), TripOutcome::Accepted);
    }
    assert_eq!(f.d.get_state(1).queue, old);
    f.d.edit(1, |st| st.search.home = location(1, true));
    f.d.dispatch_commute(1, true, 3, 8);
    assert_eq!(f.d.get_state(1).queue[0].cell.0, 1);
    assert_eq!(f.d.dispatch_commute(1, true, 1, 8), TripOutcome::UnsupportedMs19);
}

#[test]
fn ms17_07_record_bytes_roundtrip_and_deferred_setup() {
    let mut st = DispatchState {
        auxiliary_flag: true,
        travel_type: 0xFE,
        opaque_28: 0xDEADBEEF,
        tuning: DispatchTuning {
            approach_distance: 8,
            proximity: 4,
            idle_minimum: 1,
            location_minimum: 30,
            switch_minimum: 1,
        },
        ..DispatchState::default()
    };
    st.search.route.extend([
        RouteData {
            x: 2,
            z: 3,
            travel_type: 1,
        },
        RouteData {
            x: 4,
            z: 5,
            travel_type: 0,
        },
    ]);
    st.models.insert(
        2,
        (
            ResKey {
                type_id: 8,
                group: 9,
                instance: 10,
            },
            -1,
        ),
    );
    st.models.insert(
        1,
        (
            ResKey {
                type_id: 11,
                group: 12,
                instance: 13,
            },
            14,
        ),
    );
    st.models.insert(
        -1,
        (
            ResKey {
                type_id: 21,
                group: 22,
                instance: 23,
            },
            24,
        ),
    );
    st.queue.extend([location(1, false), location(2, false)]);
    let mut out = ByteWriter::new();
    assert!(st.write(&mut out, &Refs));
    let bytes = out.bytes();
    assert_eq!(
        &bytes[..25],
        &[1, 0, 0, 0, 1, 8, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 30, 0, 0, 0, 1, 0, 0, 0]
    );
    assert_eq!(&bytes[49..53], &[255; 4]);
    assert_eq!(&bytes[65..69], &[254, 0, 0, 0]);
    assert_eq!(&bytes[77..81], &[0; 4]);
    assert_eq!(&bytes[253..263], &[2, 0, 0, 0, 2, 3, 1, 4, 5, 0]);
    assert_eq!(&bytes[267..271], &[255; 4]);
    let mut inp = ByteReader::new(bytes);
    let restored = DispatchState::read(&mut inp, &Refs).unwrap();
    assert_eq!(restored, st);
    let mut future = bytes.to_vec();
    future[..4].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(DispatchState::read(&mut ByteReader::new(&future), &Refs).unwrap(), st);
    for len in [0, 1, 4, 76, 77, bytes.len() - 1] {
        assert!(DispatchState::read(&mut ByteReader::new(&bytes[..len]), &Refs).is_none());
    }
    let f = D::new();
    let s = Services::default();
    assert!(f.d.read_record(1, &mut ByteReader::new(bytes), &Refs, &s));
    assert!(f.d.job_pathfinder_config(1).is_none());
    assert!(f.take().is_empty());
    assert!(f.d.deferred_setup_message(1, &s));
    assert_eq!(
        f.take(),
        vec![(1, 0x4BB2D020, 0, 0), (1, 0x4BB2CFC0, (-2i32) as u32, u32::MAX)]
    );
    assert_eq!(*s.registered.borrow(), vec![true, false]);
    assert_eq!(f.d.get_state(1).queue, st.queue);
    assert!(!f.d.deferred_setup_message(1, &s));
    assert!(!f.d.read_record(2, &mut ByteReader::new(&[1]), &Refs, &s));
    assert!(!f.d.deferred_setup_message(2, &s));
}

#[test]
fn ms17_08_player_entry_exit_and_no_synthetic_arrival() {
    let f = D::new();
    let s = Services::default();
    f.d.edit(1, |st| {
        st.search.state = 3;
        st.search.home = location(1, true);
        st.current = st.search.home;
        st.search.commute_destination = location(2, true);
    });
    assert!(f.d.can_u_drive_it(1, &s));
    assert_eq!(f.d.enter_u_drive_it(1, &s), TripOutcome::Accepted);
    assert!(f.d.check_flags(1, 0x20000));
    assert_eq!(f.d.get_state(1).queue[0].cell.0, 2);
    f.take();
    assert!(f.d.exit_u_drive_it(1, &s));
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0x20000, 0)]);
    f.d.edit(1, |st| {
        st.search.state = 2;
        st.search.flags = 0x30000;
        st.travel_type = 1;
        st.automata_occupant = Some(77);
    });
    assert!(f.d.exit_u_drive_it(1, &s));
    let st = f.d.get_state(1);
    assert_eq!(
        (st.search.state, st.travel_type, st.current, st.destination),
        (-1, 255, st.search.home, st.search.home)
    );
    assert!(st.queue.is_empty());
    assert_eq!(st.automata_occupant, None);
    assert_eq!(
        f.take(),
        vec![(1, 0x4BB2CFC0, 2, u32::MAX), (1, 0x4BB2D020, 0x30000, 0)]
    );
    assert_eq!(s.detached.get(), 1);
    assert_eq!(*s.released.borrow(), vec![77]);
}

#[test]
fn recovery_selection_and_transport_boundaries() {
    let f = D::new();
    let s = Services::default();
    f.d.init(1);
    f.d.set_default_destination(1, location(1, true));
    f.take();
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).search.state, 3);
    let notes = f.take();
    assert_eq!(notes[0].1, 0xEBA06C12);
    assert_eq!(notes[1].1, 0x4BB2D020);
    assert_eq!(notes[2].1, 0x4BB2CFC0);
    f.d.add_destination_location(1, DispatchLocation::default(), 0);
    f.d.add_destination_location(1, location(2, false), 0);
    f.d.edit(1, |st| {
        st.minutes = 20;
        st.search.flags |= 0x5800;
    });
    f.d.animation_message(1, &s);
    let st = f.d.get_state(1);
    assert_eq!((st.search.state, st.destination.cell.0, st.neighbor_retries), (2, 2, 0));
    let notes = f.take();
    assert_eq!(notes[0].1, 0x4BB2CFC0);
    assert_eq!(notes[1].1, 0x4BB2D020);
    s.search.set(Some(TripSearch::Suspended));
    let st = f.d.get_state(1);
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1), st);
    assert!(f.take().is_empty());
    s.search.set(Some(TripSearch::Failed {
        neighbor_retries_exhausted: true,
    }));
    assert_eq!(f.d.animation_message(1, &s), TripOutcome::Rejected);
    assert_eq!(
        f.take().iter().map(|n| n.1).collect::<Vec<_>>(),
        vec![0x4BB2CFC0, 0xEBA06C12]
    );
    f.d.edit(1, |st| {
        st.search.state = 2;
        st.search.flags = 0x11040;
    });
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).search.state, 2);
    f.d.edit(1, |st| st.search.flags &= !0x10000);
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).search.state, -1);
    f.d.edit(1, |st| {
        st.search.state = 2;
        st.search.flags = 0x1000;
    });
    assert_eq!(f.d.animation_message(1, &s), TripOutcome::UnsupportedMs18);
}

#[test]
fn recovery_home_fallback_and_no_selection() {
    let f = D::new();
    let s = Services::default();
    f.d.init(1);
    f.d.edit(1, |st| {
        st.search.home = location(1, true);
        st.current = st.search.home;
        st.search.flags = FLAG_HAS_HOME | 0x5800;
        st.queue.push_back(location(2, false));
    });
    f.d.animation_message(1, &s);
    let notes = f.take();
    assert!(notes.iter().all(|n| n.1 != 0xEBA06C12));
    assert_eq!(f.d.get_state(1).queue.len(), 1);
    f.d.edit(1, |st| {
        st.current = DispatchLocation::default();
        st.queue.clear();
    });
    f.d.animation_message(1, &s);
    assert_eq!(f.take(), vec![(1, 0xEBA06C12, 1, 0)]);
    assert_eq!(f.d.get_state(1).current.cell.0, 1);
    // At home with no queued selection stays at location after expiration.
    f.d.edit(1, |st| st.minutes = 100);
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).search.state, 3);
    // Away from home with no queue chooses home and derives its deadline.
    f.d.edit(1, |st| st.current = location(2, false));
    f.d.animation_message(1, &s);
    let st = f.d.get_state(1);
    assert_eq!((st.search.state, st.destination.cell.0, st.destination.deadline()), (2, 1, 105));
    // No home and no destinations recovers; recovery retains a pending queue.
    f.d.edit(1, |st| { st.search.state = 3; st.search.flags = 0; });
    f.d.animation_message(1, &s);
    assert_eq!(f.d.get_state(1).search.state, -1);
    f.d.add_destination_location(1, location(3, false), 0);
    f.d.animation_message(1, &s);
    assert_eq!((f.d.get_state(1).search.state, f.d.get_state(1).queue.len()), (3, 1));
}

#[test]
fn goto_cancellation_rejection_and_player_guards() {
    let f = D::new();
    let s = Services::default();
    f.d.edit(1, |st| { st.search.state = 2; st.current = location(1, true); });
    assert!(!f.d.can_u_drive_it(1, &s));
    assert_eq!(f.d.go_to(1, location(2, false), &s), TripOutcome::UnsupportedMs18);
    s.cancel.set(Some(false));
    s.search.set(Some(TripSearch::Suspended));
    f.take();
    assert_eq!(f.d.go_to(1, location(3, false), &s), TripOutcome::Rejected);
    assert!(f.d.check_flags(1, 0x4040));
    assert_eq!(f.d.get_state(1).current.cell.0, 1);
    f.d.edit(1, |st| {
        st.search.route.push_back(RouteData { x: 1, z: 1, travel_type: 1 });
        st.path_indices = (0, 1);
        st.search.flags = 0;
    });
    assert_eq!(f.d.go_to(1, location(4, false), &s), TripOutcome::Accepted);
    f.d.edit(1, |st| { st.travel_type = 1; st.automata_occupant = Some(42); st.search.flags = 0x20000; });
    assert!(f.d.can_u_drive_it(1, &s));
    let destination = f.d.get_state(1).destination;
    assert_eq!(f.d.held_automaton_player_update(1, &s), TripOutcome::Accepted);
    assert!(f.d.check_flags(1, 0x10000));
    assert!(!f.d.check_flags(1, 0x20000));
    assert_eq!(f.d.get_state(1).destination, destination);
    f.d.edit(1, |st| st.travel_type = 0);
    assert!(!f.d.can_u_drive_it(1, &s));
    assert_eq!(f.d.enter_u_drive_it(1, &s), TripOutcome::Rejected);
}
