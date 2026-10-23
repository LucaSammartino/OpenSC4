// SPDX-License-Identifier: GPL-3.0-or-later
//! Executable mysims.md §12 dispatch specification vectors, not captured traces.
//! MySim-owned state/sound handling in vector 06 is outside dispatch closure.
use fc4_core::mysims::dispatch::{
    scripted::DispatchFixture, DispatchLocation, DispatchRecordRefs, DispatchState, DispatchTuning, TripOutcome,
    TripServices,
};
use fc4_core::pathfinder::RouteData;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, ResKey, SaveRegistry};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct Foreign {
    cancel: Cell<Option<bool>>,
}
impl TripServices for Foreign {
    fn cancel_automaton(&self, _: Option<u64>) -> Option<bool> {
        self.cancel.get()
    }
}
struct Refs;
impl DispatchRecordRefs for Refs {
    fn reference(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: 0x1234,
        }
    }
    fn token(&self, reference: RefValue) -> Option<u64> {
        match reference {
            RefValue::Host { token, .. } => Some(token),
            _ => None,
        }
    }
}
struct Resolver(Rc<RefCell<SaveRegistry>>);
impl RefResolver for Resolver {
    fn resolve(&mut self, class: u32, id: u32) -> Option<RefValue> {
        self.0
            .borrow()
            .entries
            .iter()
            .find(|(c, i, _, _)| (*c, *i) == (class, id))
            .map(|(_, _, _, token)| RefValue::Host {
                token: *token,
                class_id: class,
            })
    }
}
fn loc(cell: i32, occupant: bool) -> DispatchLocation {
    let mut l = DispatchLocation {
        occupant: occupant.then_some(cell as u64 + 100),
        ..DispatchLocation::default()
    };
    l.set_cell_position(cell, 1);
    l.set_duration(5);
    l
}
fn restore(f: &DispatchFixture, state: &DispatchState) {
    let registry = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(registry.clone());
    assert!(state.write(&mut out, &Refs));
    let mut resolver = Resolver(registry);
    assert!(f.d.read_record(
        1,
        &mut ByteReader::with_resolver(out.bytes(), &mut resolver),
        &Refs,
        &Foreign::default()
    ));
    f.take();
}

#[test]
fn ms19_01_exact_identity() {
    let a = loc(4, true);
    let mut b = a;
    b.world = (f32::NAN, 999.0);
    b.set_duration(-1);
    b.set_deadline(99);
    assert!(a.same_location(&b));
    let variants = [
        DispatchLocation { in_city: false, ..a },
        DispatchLocation { by_commute: true, ..a },
        DispatchLocation { occupant: Some(77), ..a },
        DispatchLocation { cell: (5, a.cell.1), ..a },
        DispatchLocation { cell: (a.cell.0, 5), ..a },
        DispatchLocation { neighbor_city: (2, -1), ..a },
        DispatchLocation { neighbor_city: (-1, 2), ..a },
    ];
    for b in variants {
        assert!(!a.same_location(&b));
        assert!(!b.same_location(&a));
    }
}

#[test]
fn ms19_02_null_and_neighbor() {
    let a = DispatchLocation {
        occupant: None,
        in_city: false,
        by_commute: true,
        cell: (4, 5),
        neighbor_city: (2, 3),
        ..DispatchLocation::default()
    };
    assert!(a.same_location(&a));
    for b in [
        DispatchLocation { neighbor_city: (2, 4), ..a },
        DispatchLocation { by_commute: false, ..a },
        DispatchLocation { occupant: Some(7), ..a },
    ] {
        assert!(!a.same_location(&b));
    }
    let b = DispatchLocation { occupant: Some(7), ..a };
    assert!(!b.same_location(&DispatchLocation { neighbor_city: (2, 4), ..b }));
}

#[test]
fn ms19_03_teleport_ordering() {
    let f = DispatchFixture::new();
    let mut st = f.d.get_state(1);
    st.search.state = 2;
    st.travel_type = 1;
    st.change_counter = 10;
    st.minutes = 100;
    st.current = loc(1, true);
    st.search.home = loc(2, true);
    st.destination = loc(3, true);
    st.search.commute_destination = loc(4, true);
    st.search.commute_destination.set_duration(7);
    st.search.commute_destination.set_deadline(999);
    st.queue.push_back(loc(5, true));
    st.automata_occupant = Some(123);
    restore(&f, &st);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_cb = seen.clone();
    let weak = Rc::downgrade(&f.d);
    *f.w.on_notify.borrow_mut() = Some(Rc::new(move |sim, _, _, _| {
        seen_cb.borrow_mut().push(weak.upgrade().unwrap().get_state(sim));
    }));
    f.d.teleport_to_commute_location(1);
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0, 2), (1, 0x4BB2CFC0, 2, 3)]);
    let got = f.d.get_state(1);
    assert_eq!((got.search.prev_state, got.search.state, got.travel_type, got.change_counter), (2, 3, 0xFE, 14));
    let mut expected = st.search.commute_destination;
    expected.set_deadline(107);
    assert_eq!(got.current, expected);
    assert_eq!(got.destination, st.destination);
    assert_eq!(got.search.home, st.search.home);
    assert_eq!(got.search.commute_destination, st.search.commute_destination);
    assert_eq!(got.queue, st.queue);
    assert_eq!(got.automata_occupant, Some(123));
    for (i, snapshot) in seen.borrow().iter().enumerate() {
        assert_eq!(snapshot.current, expected);
        assert_eq!((snapshot.search.state, snapshot.search.flags, snapshot.travel_type), (3, 2, 0xFE));
        assert_eq!(snapshot.change_counter, 12 + i as u32);
    }
}

#[test]
fn ms19_04_repeated_invalid_teleport() {
    let f = DispatchFixture::new();
    let mut st = f.d.get_state(1);
    st.search.state = 3;
    st.search.flags = 2;
    st.travel_type = 0xFE;
    st.change_counter = 10;
    st.minutes = 100;
    st.search.commute_destination.set_duration(-1);
    restore(&f, &st);
    for count in [11, 12] {
        f.d.teleport_to_commute_location(1);
        let got = f.d.get_state(1);
        assert_eq!((got.current.deadline(), got.search.prev_state, got.search.state, got.change_counter), (99, 3, 3, count));
        assert!(f.take().is_empty());
        f.w.absent_owners.borrow_mut().push(1);
    }
    st.search.state = 2;
    st.search.flags = 0;
    st.travel_type = 1;
    st.minutes = i32::MIN;
    st.change_counter = u32::MAX - 1;
    restore(&f, &st);
    f.d.teleport_to_commute_location(1);
    let got = f.d.get_state(1);
    assert_eq!((got.current.deadline(), got.change_counter), (i32::MAX, 2));
    assert!(f.take().is_empty());
}

#[test]
fn ms19_05_state_one_commute_request() {
    let f = DispatchFixture::new();
    let mut st = f.d.get_state(1);
    st.search.state = 3;
    st.search.flags = 2;
    st.travel_type = 0xFE;
    st.minutes = 100;
    st.change_counter = 10;
    st.search.home = loc(2, true);
    st.search.home.set_duration(9);
    st.search.commute_destination.set_duration(-1);
    st.queue.push_back(loc(3, true));
    restore(&f, &st);
    assert_eq!(f.d.dispatch_commute(1, false, 1, 8), TripOutcome::Rejected);
    assert_eq!(f.d.get_state(1).queue, st.queue);
    assert_eq!(f.d.dispatch_commute(1, true, 1, 8), TripOutcome::Accepted);
    let got = f.d.get_state(1);
    assert_eq!(got.current.deadline(), 99);
    assert_eq!(got.search.commute_destination, st.search.commute_destination);
    assert_eq!(got.change_counter, 11);
    assert_eq!(got.queue, [st.search.home]);
    assert!(f.take().is_empty());
}

#[test]
fn ms17_01_dispatch_identity() {
    let f = DispatchFixture::new();
    let foreign = Foreign::default();
    let fresh = f.d.get_state(1);
    assert_eq!(
        (
            fresh.search.state,
            fresh.search.prev_state,
            fresh.search.flags,
            fresh.minutes,
            fresh.travel_type
        ),
        (-1, -1, 0, 0, 255)
    );
    let mut st = fresh;
    st.current = loc(1, true);
    st.search.home = loc(2, true);
    st.destination = loc(3, true);
    st.search.commute_destination = loc(4, true);
    st.search.state = 2;
    restore(&f, &st);
    foreign.cancel.set(Some(true));
    assert_eq!(f.d.go_to(1, loc(5, true), &foreign), TripOutcome::Accepted);
    let got = f.d.get_state(1);
    assert_eq!(
        (
            got.current,
            got.search.home,
            got.destination,
            got.search.commute_destination
        ),
        (st.current, st.search.home, loc(5, true), st.search.commute_destination)
    );
    assert_eq!(got.search.state, 2);
    assert!(f.take().is_empty());
}

#[test]
fn ms17_02_notification_order() {
    let f = DispatchFixture::new();
    let weak = Rc::downgrade(&f.d);
    *f.w.on_notify.borrow_mut() = Some(Rc::new(move |sim, _, _, _| {
        assert_eq!(weak.upgrade().unwrap().get_state(sim).search.state, 3);
    }));
    f.d.set_state(1, 3);
    f.d.set_state(1, 3);
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0, 2), (1, 0x4BB2CFC0, u32::MAX, 3)]);
    assert_eq!(f.d.get_state(1).change_counter, 2);
}

#[test]
fn ms17_03_clock_and_wait() {
    let f = DispatchFixture::new();
    let foreign = Foreign::default();
    let mut st = DispatchState {
        current: loc(1, true),
        minutes: 100,
        ..DispatchState::default()
    };
    st.search.state = 3;
    st.current.set_deadline(105);
    st.queue.push_back(loc(2, true));
    restore(&f, &st);
    f.d.init(1);
    f.d.minute_message(1);
    assert_eq!(f.d.get_state(1).minutes, 105);
    assert_eq!(f.d.get_state(1).search.state, 3);
    f.d.animation_message(1, &foreign);
    assert_eq!(f.d.get_state(1).queue.len(), 1);
    f.d.minute_message(1);
    f.d.animation_message(1, &foreign);
    assert_eq!(f.d.get_state(1).search.state, 2);
    for (now, flags, want) in [(115, 0, 100), (116, 0, 0), (116, 0x10, 100)] {
        st.minutes = now;
        st.wait_timestamp = 100;
        st.search.flags = flags;
        restore(&f, &st);
        assert_eq!(f.d.wait_value(1), want);
        assert_eq!(f.d.get_state(1).wait_timestamp, want);
    }
}

#[test]
fn ms17_04_tuning() {
    let f = DispatchFixture::new();
    let foreign = Foreign::default();
    f.d.set_tuning(
        1,
        DispatchTuning {
            location_minimum: 30,
            approach_distance: 8,
            ..DispatchTuning::default()
        },
    );
    f.d.go_to(1, loc(2, true), &foreign);
    assert_eq!(f.d.get_state(1).queue[0].duration(), 30);
    f.d.go_to(1, loc(2, false), &foreign);
    assert_eq!(f.d.get_state(1).queue[0].duration(), 5);
    let mut st = f.d.get_state(1);
    st.destination.set_cell_position(0, 0);
    restore(&f, &st);
    for (cell, want) in [(7, true), (8, false), (9, false)] {
        f.d.automaton_cell_changed(1, (cell, 0));
        assert_eq!(f.d.check_flags(1, 0x8000), want);
    }
    let notes = f.take();
    assert_eq!(notes.len(), 2);
    assert_eq!((notes[0].2, notes[0].3), (0, 0x8000));
    assert_eq!((notes[1].2, notes[1].3), (0x8000, 0));
    st.search.flags = 0x18000;
    restore(&f, &st);
    f.d.automaton_cell_changed(1, (20, 0));
    assert!(f.d.check_flags(1, 0x8000));
    assert!(f.take().is_empty());
}

#[test]
fn ms17_05_queue_and_deadline() {
    let f = DispatchFixture::new();
    let mut st = DispatchState {
        minutes: 100,
        ..DispatchState::default()
    };
    st.search.state = 3;
    st.current.set_deadline(200);
    st.queue.extend([loc(1, false), loc(2, false)]);
    restore(&f, &st);
    assert!(f.d.add_destination_location(1, loc(3, false), 4));
    assert!(f.d.add_destination_location(1, loc(4, false), 1));
    let got = f.d.get_state(1);
    assert_eq!(got.queue.iter().map(|l| l.cell.0).collect::<Vec<_>>(), vec![4, 3]);
    assert_eq!(got.current.deadline(), 200);
    assert_eq!(got.search.state, 3);
    assert_eq!(f.d.go_to(1, loc(5, false), &Foreign::default()), TripOutcome::Accepted);
    let got = f.d.get_state(1);
    assert_eq!(
        (got.queue.len(), got.queue[0].cell.0, got.current.deadline()),
        (1, 5, 99)
    );
}

#[test]
fn ms17_06_commute_requests_dispatch_portion() {
    let f = DispatchFixture::new();
    for (hour, duration) in [(7, 5), (8, 540), (17, 0), (18, 5)] {
        assert_eq!(f.d.dispatch_commute(1, true, 2, hour), TripOutcome::Accepted);
        let got = f.d.get_state(1);
        assert_eq!(got.queue.len(), 1);
        assert!(got.queue[0].by_commute);
        assert_eq!(got.queue[0].duration(), duration);
    }
    let queue = f.d.get_state(1).queue;
    assert_eq!(f.d.dispatch_commute(1, false, 2, 8), TripOutcome::Rejected);
    f.d.dispatch_commute(1, true, 5, 8);
    f.d.dispatch_commute(1, true, 4, 8);
    assert_eq!(f.d.get_state(1).queue, queue);
}

#[test]
fn ms17_07_dispatch_record() {
    let mut st = DispatchState {
        auxiliary_flag: true,
        travel_type: 254,
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
    for k in [2, 1] {
        st.models.insert(
            k,
            (
                ResKey {
                    type_id: 11,
                    group: 22,
                    instance: k as u32,
                },
                -k,
            ),
        );
    }
    st.queue.extend([loc(1, false), loc(2, false)]);
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
    assert_eq!(&bytes[267..271], &1i32.to_le_bytes());
    assert_eq!(DispatchState::read(&mut ByteReader::new(bytes), &Refs).unwrap(), st);
}

#[test]
fn ms17_08_player_and_rng() {
    let f = DispatchFixture::new();
    let foreign = Foreign::default();
    let mut st = DispatchState::default();
    st.search.state = 3;
    st.search.home = loc(1, true);
    st.current = st.search.home;
    st.search.commute_destination = loc(2, true);
    restore(&f, &st);
    assert_eq!(f.d.enter_u_drive_it(1, &foreign), TripOutcome::Accepted);
    assert!(f.d.check_flags(1, 0x20000));
    f.take();
    f.d.exit_u_drive_it(1, &foreign);
    assert_eq!(f.take(), vec![(1, 0x4BB2D020, 0x20000, 0)]);
    st.search.state = 2;
    st.search.flags = 0x10000;
    st.queue.push_back(loc(3, false));
    restore(&f, &st);
    f.d.exit_u_drive_it(1, &foreign);
    let got = f.d.get_state(1);
    assert_eq!(
        (got.current, got.destination, got.travel_type, got.search.state),
        (st.search.home, st.search.home, 255, -1)
    );
    assert!(got.queue.is_empty());
    assert_eq!(
        f.take(),
        vec![(1, 0x4BB2CFC0, 2, u32::MAX), (1, 0x4BB2D020, 0x10000, 0)]
    );
    // These APIs accept no RNG/service capable of consuming residence draws.
    // The record's fixed 18-word header also has no RNG slot (§8).
}

#[test]
fn dispatch_record_preserves_all_opaque_words_and_foreign_references() {
    let f = DispatchFixture::new();
    let mut st = DispatchState {
        auxiliary_flag: true,
        opaque_28: 0xF1234567,
        path_indices: (123, 456),
        change_counter: 42,
        previous_change_counter: 41,
        tracking_value: u32::MAX,
        minutes: -123,
        travel_type: 0xABCDEF01,
        wait_timestamp: -456,
        neighbor_retries: 27,
        auxiliary_occupant: Some(500),
        auxiliary_value: 512,
        auxiliary_string: vec![255, 65],
        automata_occupant: Some(501),
        owner: Some(502),
        simulation_service: Some(503),
        route_marker: -77,
        current: loc(1, true),
        destination: loc(2, true),
        ..DispatchState::default()
    };
    st.search.home = loc(3, true);
    st.search.commute_destination = loc(4, true);
    st.search.state = 2;
    st.search.prev_state = -3;
    st.search.flags = 0x12345678;
    st.current.set_duration(-10);
    st.current.set_deadline(-20);
    st.search.home.in_city = false;
    st.search.home.neighbor_city = (-8, 19);
    st.queue.push_back(st.current);
    restore(&f, &st);
    let mut restored = f.d.get_state(1);
    restored.read_pending = false;
    restored.read_succeeded = false;
    assert_eq!(restored, st);
    // Non-null references without a registry fail the stream, not a fake null write.
    assert!(!st.write(&mut ByteWriter::new(), &Refs));
}
