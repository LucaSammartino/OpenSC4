// SPDX-License-Identifier: GPL-3.0-or-later
// Independent literal fixtures from auto-game-pause.md §4; no native oracle.
use super::{sc4_game_pause, AutoGamePause, Host, Storage};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Store(usize, u8),
    Read(usize, u8),
    Framework(u32),
    Lookup(u32, u32, u32, u32, u32, u32, u32),
    Ref(u32, i32),
    Call(u32, u32, Vec<u32>),
    City(u32, u32, u32),
    Clock(u32, u32, u32),
    Release(u32, u32),
}

type Trace = Rc<RefCell<Vec<Event>>>;

#[derive(Clone)]
struct Reply {
    framework: u32,
    service: u32,
    iid: u32,
    // None deliberately leaves the output untouched, testing zero initialization.
    output: Option<u32>,
    status: u32,
    city: u32,
    clock: u32,
}

fn replies(epoch: u32) -> [Reply; 4] {
    // Every nonnull identity has AL=0, including full-width high-bit identities.
    let base = epoch * 0x10000;
    [
        Reply {
            framework: base + 0x100,
            service: 0xc416025c,
            iid: 0x0073283c,
            output: Some(base + 0x80000100),
            status: 1,
            city: 0,
            clock: 0,
        },
        Reply {
            framework: base + 0x200,
            service: 0x86ad10ec,
            iid: 0x86ad10ed,
            output: Some(base + 0x200),
            status: 1,
            city: 0,
            clock: 0,
        },
        Reply {
            framework: base + 0x300,
            service: 0x46956659,
            iid: 0x8695664e,
            output: Some(base + 0x300),
            status: 1,
            city: 0,
            clock: 0,
        },
        Reply {
            framework: base + 0x400,
            service: 0x66,
            iid: 0x26ce01c0,
            output: Some(base + 0x400),
            status: 1,
            city: base + 0x80000500,
            clock: base + 0x80000600,
        },
    ]
}

struct Script {
    pending: VecDeque<Reply>,
    active: Option<Reply>,
    trace: Trace,
    refs: BTreeMap<u32, i32>,
    observe: Option<(Rc<Cell<[u8; 4]>>, [u8; 4])>,
}

impl Script {
    fn new(replies: impl IntoIterator<Item = Reply>, trace: Trace) -> Self {
        Self {
            pending: replies.into_iter().collect(),
            active: None,
            trace,
            refs: BTreeMap::new(),
            observe: None,
        }
    }

    fn record(&self, event: Event) {
        self.trace.borrow_mut().push(event);
    }

    fn finish(&self) {
        assert!(self.pending.is_empty(), "missing framework acquisitions");
        assert!(self.refs.values().all(|&refs| refs == 0));
    }
}

impl Host for Script {
    fn framework(&mut self) -> u32 {
        if let Some((bytes, expected)) = &self.observe {
            assert_eq!(bytes.get(), *expected, "storage before first callback");
        }
        let reply = self.pending.pop_front().expect("extra framework acquisition");
        let result = reply.framework;
        self.active = Some(reply);
        self.record(Event::Framework(result));
        result
    }

    fn lookup(
        &mut self,
        framework: u32,
        slot: u32,
        service: u32,
        iid: u32,
        output: &mut u32,
    ) -> u32 {
        let reply = self.active.as_ref().unwrap().clone();
        assert_eq!((framework, service, iid), (reply.framework, reply.service, reply.iid));
        assert_eq!(*output, 0, "each selected lookup starts with null output");
        let initial = *output;
        if let Some(value) = reply.output {
            *output = value;
        }
        self.record(Event::Lookup(framework, slot, service, iid, initial, *output, reply.status));
        if *output != 0 {
            let balance = self.refs.entry(*output).or_default();
            *balance += 1;
            let balance = *balance;
            self.record(Event::Ref(*output, balance));
        }
        reply.status
    }

    fn call(&mut self, participant: u32, slot: u32, arguments: &[u32]) -> u32 {
        self.record(Event::Call(participant, slot, arguments.to_vec()));
        0 // A false/raw-zero reply never suppresses subsequent operations.
    }

    fn current_city(&mut self, application: u32, slot: u32) -> u32 {
        let city = self.active.as_ref().unwrap().city;
        self.record(Event::City(application, slot, city));
        city
    }

    fn clock(&mut self, city: u32, slot: u32) -> u32 {
        let clock = self.active.as_ref().unwrap().clock;
        self.record(Event::Clock(city, slot, clock));
        clock
    }

    fn release(&mut self, participant: u32, slot: u32) -> u32 {
        self.record(Event::Release(participant, slot));
        let balance = self.refs.get_mut(&participant).expect("unowned Release");
        *balance -= 1;
        assert!(*balance >= 0, "extra Release");
        let balance = *balance;
        self.record(Event::Ref(participant, balance));
        0
    }
}

fn lookup_trace(reply: &Reply) -> Vec<Event> {
    let output = reply.output.unwrap_or(0);
    let mut events = vec![
        Event::Framework(reply.framework),
        Event::Lookup(reply.framework, 0x14, reply.service, reply.iid, 0, output, reply.status),
    ];
    if output != 0 {
        events.push(Event::Ref(output, 1));
    }
    events
}

fn cleanup(reply: &Reply) -> Vec<Event> {
    let output = reply.output.unwrap();
    vec![Event::Release(output, 8), Event::Ref(output, 0)]
}

// Literal P/A/S/C operation vectors, independent of production selectors/slots.
// No model of participant pause state or aggregate result is used.
fn literal_stages(replies: &[Reply; 4], entering: bool) -> [Vec<Event>; 4] {
    let p = replies[0].output.unwrap();
    let a = replies[1].output.unwrap();
    let s = replies[2].output.unwrap();
    let u = replies[3].output.unwrap();
    let c = replies[3].city;
    let k = replies[3].clock;
    let operations = if entering {
        [
            vec![Event::Call(p, 0x48, vec![])],
            vec![Event::Call(a, 0x30, vec![1])],
            vec![Event::Call(s, 0x28, vec![])],
            vec![Event::City(u, 0x94, c), Event::Clock(c, 0x158, k), Event::Call(k, 0x68, vec![1])],
        ]
    } else {
        [
            vec![Event::Call(p, 0x4c, vec![])],
            vec![Event::Call(a, 0x30, vec![0])],
            vec![Event::Call(s, 0x34, vec![])],
            vec![Event::City(u, 0x94, c), Event::Clock(c, 0x158, k), Event::Call(k, 0x68, vec![0])],
        ]
    };
    std::array::from_fn(|i| {
        let mut trace = lookup_trace(&replies[i]);
        trace.extend(operations[i].clone());
        trace.extend(cleanup(&replies[i]));
        trace
    })
}

// All 16 public truth-table rows: explicit flags and literal selected stage order.
const SELECTIONS: [([u32; 4], &[usize]); 16] = [
    ([0, 0, 0, 0], &[]),
    ([0, 0, 0, 1], &[3]),
    ([0, 0, 1, 0], &[2]),
    ([0, 0, 1, 1], &[2, 3]),
    ([0, 1, 0, 0], &[1]),
    ([0, 1, 0, 1], &[1, 3]),
    ([0, 1, 1, 0], &[1, 2]),
    ([0, 1, 1, 1], &[1, 2, 3]),
    ([1, 0, 0, 0], &[0]),
    ([1, 0, 0, 1], &[0, 3]),
    ([1, 0, 1, 0], &[0, 2]),
    ([1, 0, 1, 1], &[0, 2, 3]),
    ([1, 1, 0, 0], &[0, 1]),
    ([1, 1, 0, 1], &[0, 1, 3]),
    ([1, 1, 1, 0], &[0, 1, 2]),
    ([1, 1, 1, 1], &[0, 1, 2, 3]),
];

fn selected_trace(stages: &[Vec<Event>; 4], selected: &[usize]) -> Vec<Event> {
    selected.iter().flat_map(|&i| stages[i].clone()).collect()
}

#[test]
fn all_sixteen_selections_in_both_directions_exact_callbacks_and_refs() {
    for direction in [0, 1] {
        for (flags, selected) in SELECTIONS {
            let replies = replies(0);
            let expected = selected_trace(&literal_stages(&replies, direction == 1), selected);
            let trace = Trace::default();
            let mut host = Script::new(selected.iter().map(|&i| replies[i].clone()), trace.clone());
            sc4_game_pause(&mut host, [direction, flags[0], flags[1], flags[2], flags[3]]);
            host.finish();
            assert_eq!(*trace.borrow(), expected, "D={direction}, F={flags:?}");
        }
    }
}

#[test]
fn raw_direction_and_each_raw_flag_use_only_low_byte() {
    for (raw, selected) in [(2, true), (0x80, true), (u32::MAX, true), (0x100, false), (0xabcd1200, false), (0xabcd1202, true)] {
        for direction in [0, 1, 2, 0x80, u32::MAX, 0x100, 0x80000000] {
            for stage in 0..4 {
                let replies = replies(0);
                let (pending, expected) = if selected {
                    let entering = matches!(direction, 1 | 2 | 0x80 | u32::MAX);
                    (vec![replies[stage].clone()], literal_stages(&replies, entering)[stage].clone())
                } else {
                    (vec![], vec![])
                };
                let trace = Trace::default();
                let mut host = Script::new(pending, trace.clone());
                let mut operands = [direction, 0, 0, 0, 0];
                operands[stage + 1] = raw;
                sc4_game_pause(&mut host, operands);
                host.finish();
                assert_eq!(*trace.borrow(), expected);
            }
        }
    }
}

#[test]
fn every_framework_and_output_absence_still_allows_following_stages() {
    for direction in [0, 1] {
        for missing in 0..4 {
            for framework_absent in [true, false] {
                let mut replies = replies(0);
                let mut expected = literal_stages(&replies, direction == 1);
                if framework_absent {
                    replies[missing].framework = 0;
                    expected[missing] = vec![Event::Framework(0)];
                } else {
                    replies[missing].output = None;
                    expected[missing] = lookup_trace(&replies[missing]);
                }
                let trace = Trace::default();
                let mut host = Script::new(replies, trace.clone());
                sc4_game_pause(&mut host, [direction, 1, 1, 1, 1]);
                host.finish();
                assert_eq!(*trace.borrow(), selected_trace(&expected, &[0, 1, 2, 3]));
            }
        }
    }
}

#[test]
fn false_lookup_status_and_zero_operation_release_replies_are_ignored() {
    for direction in [0, 1] {
        let mut replies = replies(0);
        for reply in &mut replies {
            reply.status = 0;
        }
        let expected = selected_trace(&literal_stages(&replies, direction == 1), &[0, 1, 2, 3]);
        let trace = Trace::default();
        let mut host = Script::new(replies, trace.clone());
        sc4_game_pause(&mut host, [direction, 1, 1, 1, 1]);
        host.finish();
        assert_eq!(*trace.borrow(), expected);
    }
}

#[test]
fn null_city_or_clock_releases_app_not_borrowed_pointers_in_both_directions() {
    for direction in [0, 1] {
        for city_absent in [true, false] {
            let mut reply = replies(0)[3].clone();
            if city_absent {
                reply.city = 0;
            } else {
                reply.clock = 0;
            }
            let mut expected = lookup_trace(&reply);
            let app = reply.output.unwrap();
            expected.push(Event::City(app, 0x94, reply.city));
            if !city_absent {
                expected.push(Event::Clock(reply.city, 0x158, 0));
            }
            expected.extend(cleanup(&reply));
            let trace = Trace::default();
            let mut host = Script::new([reply], trace.clone());
            sc4_game_pause(&mut host, [direction, 0, 0, 0, 1]);
            host.finish();
            assert_eq!(*trace.borrow(), expected);
        }
    }
}

struct ObservedStorage {
    bytes: Rc<Cell<[u8; 4]>>,
    trace: Trace,
}

impl Storage for ObservedStorage {
    fn read_byte(&self, offset: usize) -> u8 {
        let value = self.bytes.get()[offset];
        self.trace.borrow_mut().push(Event::Read(offset, value));
        value
    }

    fn write_byte(&mut self, offset: usize, value: u8) {
        let mut bytes = self.bytes.get();
        bytes[offset] = value;
        self.bytes.set(bytes);
        self.trace.borrow_mut().push(Event::Store(offset, value));
    }
}

#[test]
fn class_storage_boundary_preserves_original_entry_and_zero_extends_current_exit() {
    let trace = Trace::default();
    let bytes = Rc::new(Cell::new([0xa5; 4]));
    let mut owner = AutoGamePause::from_storage(ObservedStorage { bytes: bytes.clone(), trace: trace.clone() });
    assert_eq!(owner.store_entry([2, 0x80, u32::MAX, 0x100]), [1, 2, 0x80, u32::MAX, 0x100]);
    assert_eq!(bytes.get(), [2, 0x80, 0xff, 0]);
    assert_eq!(owner.read_exit(), [0, 2, 128, 255, 0]);
    assert_eq!(*trace.borrow(), vec![
        Event::Store(0, 2), Event::Store(1, 0x80), Event::Store(2, 0xff), Event::Store(3, 0),
        Event::Read(3, 0), Event::Read(2, 0xff), Event::Read(1, 0x80), Event::Read(0, 2),
    ]);
    // Isolated read boundary, not an assertion of mutated-lifetime support.
    bytes.set([0x80, 0xff, 2, 1]);
    assert_eq!(owner.read_exit(), [0, 128, 255, 2, 1]);
    assert_eq!(bytes.get(), [0x80, 0xff, 2, 1]);
}

#[test]
fn constructor_stores_before_callbacks_returns_same_object_destructor_leaves_bytes() {
    for (flags, bytes, selected) in [
        ([0, 0, 0, 0], [0, 0, 0, 0], &[][..]),
        ([2, 0x80, u32::MAX, 0x100], [2, 0x80, 0xff, 0], &[0, 1, 2][..]),
    ] {
        let trace = Trace::default();
        let storage = Rc::new(Cell::new([0xa5; 4]));
        let mut owner = AutoGamePause::from_storage(ObservedStorage { bytes: storage.clone(), trace: trace.clone() });
        let entry = replies(0);
        let exit = replies(1);
        let pending = selected.iter().map(|&i| entry[i].clone()).chain(selected.iter().map(|&i| exit[i].clone()));
        let mut host = Script::new(pending, trace.clone());
        host.observe = Some((storage.clone(), bytes));
        let identity = &owner as *const _;
        assert_eq!(owner.construct(&mut host, flags) as *const _, identity);
        owner.destruct(&mut host);
        host.finish();
        assert_eq!(storage.get(), bytes);
        let mut expected = vec![Event::Store(0, bytes[0]), Event::Store(1, bytes[1]), Event::Store(2, bytes[2]), Event::Store(3, bytes[3])];
        expected.extend(selected_trace(&literal_stages(&entry, true), selected));
        expected.extend([Event::Read(3, bytes[3]), Event::Read(2, bytes[2]), Event::Read(1, bytes[1]), Event::Read(0, bytes[0])]);
        expected.extend(selected_trace(&literal_stages(&exit, false), selected));
        assert_eq!(*trace.borrow(), expected);
    }
}

#[test]
fn serial_scopes_exit_in_interrupted_order_without_saved_binding_or_suppression() {
    let trace = Trace::default();
    let a_entry = replies(0);
    let b_entry = replies(1);
    let a_exit = replies(2);
    let b_exit = replies(3);
    let pending = [a_entry[2].clone(), a_entry[3].clone(), b_entry[2].clone(), a_exit[2].clone(), a_exit[3].clone(), b_exit[2].clone()];
    let mut host = Script::new(pending, trace.clone());
    let mut a = AutoGamePause::from_storage([0xa5; 4]);
    let mut b = AutoGamePause::from_storage([0xa5; 4]);
    a.construct(&mut host, [0, 0, 1, 1]);
    b.construct(&mut host, [0, 0, 1, 0]);
    a.destruct(&mut host);
    b.destruct(&mut host);
    host.finish();
    let mut expected = selected_trace(&literal_stages(&a_entry, true), &[2, 3]);
    expected.extend(selected_trace(&literal_stages(&b_entry, true), &[2]));
    expected.extend(selected_trace(&literal_stages(&a_exit, false), &[2, 3]));
    expected.extend(selected_trace(&literal_stages(&b_exit, false), &[2]));
    assert_eq!(*trace.borrow(), expected);
    assert_eq!(a.storage, [0, 0, 1, 1]);
    assert_eq!(b.storage, [0, 0, 1, 0]);
}

#[test]
fn repeated_direct_calls_reacquire_all_frameworks_services_city_and_clock() {
    let trace = Trace::default();
    let first = replies(0);
    let second = replies(1);
    let third = replies(2);
    let mut host = Script::new(first.clone().into_iter().chain(second.clone()).chain(third.clone()), trace.clone());
    sc4_game_pause(&mut host, [1, 1, 1, 1, 1]);
    sc4_game_pause(&mut host, [1, 1, 1, 1, 1]);
    sc4_game_pause(&mut host, [0, 1, 1, 1, 1]);
    host.finish();
    let mut expected = selected_trace(&literal_stages(&first, true), &[0, 1, 2, 3]);
    expected.extend(selected_trace(&literal_stages(&second, true), &[0, 1, 2, 3]));
    expected.extend(selected_trace(&literal_stages(&third, false), &[0, 1, 2, 3]));
    assert_eq!(*trace.borrow(), expected);
}

#[test]
fn entry_missing_clock_exit_present_and_entry_present_exit_missing_simulator() {
    let trace = Trace::default();
    let mut entry = replies(0);
    entry[3].clock = 0;
    let mut exit = replies(1);
    exit[2].framework = 0;
    let pending = [entry[2].clone(), entry[3].clone(), exit[2].clone(), exit[3].clone()];
    let mut host = Script::new(pending, trace.clone());
    let mut owner = AutoGamePause::from_storage([0xa5; 4]);
    owner.construct(&mut host, [0, 0, 1, 1]);
    owner.destruct(&mut host);
    host.finish();
    let mut expected = literal_stages(&entry, true)[2].clone();
    expected.extend(lookup_trace(&entry[3]));
    expected.extend([Event::City(0x400, 0x94, 0x80000500), Event::Clock(0x80000500, 0x158, 0)]);
    expected.extend(cleanup(&entry[3]));
    expected.push(Event::Framework(0));
    expected.extend(literal_stages(&exit, false)[3].clone());
    assert_eq!(*trace.borrow(), expected);
}
