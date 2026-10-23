// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent proposed public vectors, transcribed in local_grid_helper_vectors.md
//! before production implementation. Native execution remains UNEXECUTED.
use fc4_core::local_grid_helper::{
    Descriptor, Host, LocalGridHelper, NumericMode, Position, PositionOutcome, Step, UnsupportedReason,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

const MODE: NumericMode = NumericMode::QualifiedDyadicNearest;
const E: u32 = 71;
const F: u32 = 82;
const SERVICE: u32 = 19;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Factory(Vec<u8>, Option<u32>),
    Query(u32),
    Start(u32, u32),
    Stop(u32, u32),
    Release(u32, Option<u32>),
    Capture,
    Read(u8),
    Scalar(u32, u64),
    Floor(u64, u64),
    Transform(u32, Descriptor),
}

type Events = Rc<RefCell<Vec<Event>>>;

#[derive(Default)]
struct Fixture {
    events: Events,
    factory: VecDeque<(Option<u32>, u32)>,
    queries: VecDeque<u32>,
    scalars: VecDeque<f64>,
    floors: VecDeque<(f64, f64)>,
    capture_count: usize,
}

impl Host for Fixture {
    type Effect = u32;
    type Service = u32;

    fn raw_factory(&mut self, name: &[u8], output: &mut Option<u32>) -> u32 {
        self.events.borrow_mut().push(Event::Factory(name.to_vec(), *output));
        let (write, status) = self.factory.pop_front().expect("factory primitive reply");
        if let Some(effect) = write {
            *output = Some(effect);
        }
        status
    }

    fn active_query(&mut self, effect: u32) -> u32 {
        self.events.borrow_mut().push(Event::Query(effect));
        self.queries.pop_front().expect("active primitive reply")
    }

    fn start(&mut self, effect: u32, argument: u32) {
        self.events.borrow_mut().push(Event::Start(effect, argument));
    }

    fn stop(&mut self, effect: u32, argument: u32) {
        self.events.borrow_mut().push(Event::Stop(effect, argument));
    }

    fn release(&mut self, effect: u32, helper: &LocalGridHelper<u32>) {
        self.events
            .borrow_mut()
            .push(Event::Release(effect, helper.retained_effect()));
    }

    fn capture_service(&mut self) -> u32 {
        self.capture_count += 1;
        self.events.borrow_mut().push(Event::Capture);
        SERVICE
    }

    fn raw_scalar(&mut self, service: u32) -> f64 {
        assert_eq!(service, SERVICE);
        let reply = self.scalars.pop_front().expect("scalar primitive reply");
        self.events.borrow_mut().push(Event::Scalar(service, reply.to_bits()));
        reply
    }

    fn raw_floor(&mut self, input: f64) -> f64 {
        let (expected, reply) = self.floors.pop_front().expect("floor primitive reply");
        assert_eq!(input.to_bits(), expected.to_bits());
        self.events
            .borrow_mut()
            .push(Event::Floor(input.to_bits(), reply.to_bits()));
        reply
    }

    fn set_transform(&mut self, effect: u32, descriptor: &Descriptor) {
        self.events
            .borrow_mut()
            .push(Event::Transform(effect, descriptor.clone()));
    }
}

struct Input {
    words: [u32; 3],
    events: Events,
}

impl Position for Input {
    fn read_x(&mut self) -> u32 {
        self.events.borrow_mut().push(Event::Read(0));
        self.words[0]
    }
    fn read_y(&mut self) -> u32 {
        self.events.borrow_mut().push(Event::Read(1));
        self.words[1]
    }
    fn read_z(&mut self) -> u32 {
        self.events.borrow_mut().push(Event::Read(2));
        self.words[2]
    }
}

fn live() -> (LocalGridHelper<u32>, Fixture) {
    let mut host = Fixture::default();
    host.factory.push_back((Some(E), 1));
    let mut helper = LocalGridHelper::new();
    helper.init(&mut host);
    host.events.borrow_mut().clear();
    (helper, host)
}

fn commands(host: &Fixture, expected: &[Event]) {
    assert_eq!(&*host.events.borrow(), expected);
}

fn creation() -> Event {
    Event::Factory(b"local_grid\0".to_vec(), None)
}

fn check_descriptor(descriptor: &Descriptor, words: [u32; 4], flags: u8) {
    assert_eq!(descriptor.translation_words(), words[..3]);
    assert_eq!(descriptor.scale_word(), words[3]);
    assert_eq!(descriptor.flags(), flags);
    let mut backing = [0xa5; 56];
    descriptor.project_into(&mut backing);
    assert_eq!(&backing[..4], &[2, flags, 0xa5, 0xa5]);
    let identity = [0x3f80_0000_u32, 0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000];
    for (i, word) in identity.into_iter().chain(words).enumerate() {
        assert_eq!(&backing[4 + i * 4..8 + i * 4], &word.to_le_bytes());
    }
    assert_eq!(Descriptor::DEFINED_BYTES.iter().filter(|&&b| b).count(), 54);
    for (i, &defined) in Descriptor::DEFINED_BYTES.iter().enumerate() {
        assert_eq!(defined, i != 2 && i != 3);
    }
    let mut other = [0x5a; 56];
    descriptor.project_into(&mut other);
    for (i, &defined) in Descriptor::DEFINED_BYTES.iter().enumerate() {
        if defined {
            assert_eq!(backing[i], other[i]);
        }
    }
}

#[derive(Clone, Copy)]
struct Vector {
    position: [u32; 3],
    scalars: [f64; 5],
    floors: [(f64, f64); 2],
    expected: [u32; 4],
    flags: u8,
}

const G1: Vector = Vector {
    position: [0x418c_0000, 0xc050_0000, 0xbf00_0000],
    scalars: [16.0; 5],
    floors: [(35.0 / 32.0, 1.0), (-1.0 / 32.0, -1.0)],
    expected: [0x4188_0000, 0xc050_0000, 0xc170_0000, 0x4380_0000],
    flags: 3,
};

fn geometry(helper: &LocalGridHelper<u32>, host: &mut Fixture, v: Vector) {
    host.events.borrow_mut().clear();
    let captures_before = host.capture_count;
    host.scalars.extend(v.scalars);
    host.floors.extend(v.floors);
    let mut input = Input {
        words: v.position,
        events: host.events.clone(),
    };
    assert_eq!(
        helper.set_position(host, &mut input, MODE),
        Ok(PositionOutcome::Dispatched)
    );
    assert_eq!(host.capture_count, captures_before + 1);
    assert!(host.scalars.is_empty());
    assert!(host.floors.is_empty());
    let events = host.events.borrow();
    let scalar = |i: usize| Event::Scalar(SERVICE, v.scalars[i].to_bits());
    let floor = |i: usize| Event::Floor(v.floors[i].0.to_bits(), v.floors[i].1.to_bits());
    assert_eq!(
        &events[..11],
        &[
            Event::Capture,
            Event::Read(0),
            scalar(0),
            floor(0),
            scalar(1),
            Event::Read(1),
            Event::Read(2),
            scalar(2),
            floor(1),
            scalar(3),
            scalar(4),
        ]
    );
    assert_eq!(events.len(), 12); // eight foreign calls plus capture/three loads
    match &events[11] {
        Event::Transform(effect, descriptor) => {
            assert_eq!(*effect, E);
            check_descriptor(descriptor, v.expected, v.flags);
        }
        other => panic!("expected synchronous transform, got {other:?}"),
    }
}

macro_rules! vector_test {
    ($name:ident, $vector:expr) => {
        #[test]
        fn $name() {
            let (helper, mut host) = live();
            geometry(&helper, &mut host, $vector);
        }
    };
}

vector_test!(g1, G1);
vector_test!(
    g2,
    Vector {
        scalars: [8.0; 5],
        floors: [(35.0 / 16.0, 2.0), (-1.0 / 16.0, -1.0)],
        expected: [0x4184_0000, 0xc050_0000, 0xc0f0_0000, 0x4300_0000],
        ..G1
    }
);
vector_test!(
    g3_asymmetric_fresh_replies,
    Vector {
        scalars: [8.0, 32.0, 4.0, 64.0, 2.0],
        floors: [(35.0 / 16.0, 2.0), (-1.0 / 8.0, -1.0)],
        expected: [0x4190_0000, 0xc050_0000, 0, 0x4200_0000],
        ..G1
    }
);
vector_test!(
    g4_unity,
    Vector {
        position: [0x3dc0_0000, 0xc050_0000, 0xbd00_0000],
        scalars: [1.0 / 16.0; 5],
        floors: [(1.5, 1.0), (-0.5, -1.0)],
        expected: [0x3d88_0000, 0xc050_0000, 0xbd70_0000, 0x3f80_0000],
        flags: 1,
    }
);
vector_test!(
    g5_signed_y_zero,
    Vector {
        position: [0, 0x8000_0000, 0],
        floors: [(0.0, 0.0), (0.0, 0.0)],
        expected: [0x3f80_0000, 0x8000_0000, 0x3f80_0000, 0x4380_0000],
        ..G1
    }
);
vector_test!(
    g6_fractional_scale,
    Vector {
        scalars: [16.0, 16.0, 16.0, 16.0, 3.0 / 32.0],
        expected: [0x4188_0000, 0xc050_0000, 0xc170_0000, 0x3fc0_0000],
        ..G1
    }
);
vector_test!(
    g7_held_nonunity_stored_unity,
    Vector {
        scalars: [16.0, 16.0, 16.0, 16.0, 1.0 / 16.0 + 1.0 / 1073741824.0],
        expected: [0x4188_0000, 0xc050_0000, 0xc170_0000, 0x3f80_0000],
        ..G1
    }
);

#[test]
fn b_seven_boundary_cases() {
    let (helper, mut host) = live();
    for (x, q, floor, tx) in [
        (0xc180_0000, -1.0, -1.0, 0xc170_0000),
        (0xbe80_0000, -1.0 / 64.0, -1.0, 0xc170_0000),
        (0x8000_0000, -0.0, -0.0, 0x3f80_0000),
        (0, 0.0, 0.0, 0x3f80_0000),
        (0x417c_0000, 63.0 / 64.0, 0.0, 0x3f80_0000),
        (0x417f_ffff, 1.0 - 1.0 / 16777216.0, 0.0, 0x3f80_0000),
        (0x4180_0000, 1.0, 1.0, 0x4188_0000),
    ] {
        geometry(
            &helper,
            &mut host,
            Vector {
                position: [x, G1.position[1], G1.position[2]],
                floors: [(q, floor), G1.floors[1]],
                expected: [tx, 0xc050_0000, 0xc170_0000, 0x4380_0000],
                ..G1
            },
        );
    }
}

#[test]
fn l1_construction_and_init() {
    let mut host = Fixture::default();
    let mut helper = LocalGridHelper::<u32>::new();
    assert_eq!(helper.retained_effect(), None);
    commands(&host, &[]);
    host.factory.push_back((Some(E), 0xaabb_ccdd));
    helper.init(&mut host);
    commands(&host, &[creation()]);
    assert_eq!(helper.retained_effect(), Some(E));
}

#[test]
fn l2_repeated_init() {
    let (mut helper, mut host) = live();
    helper.init(&mut host);
    commands(&host, &[]);
    assert_eq!(helper.retained_effect(), Some(E));
}

#[test]
fn l3_untouched_failure_retries_only_next_init() {
    let mut helper = LocalGridHelper::<u32>::new();
    let mut host = Fixture::default();
    host.factory.extend([(None, 0), (None, 0)]);
    helper.init(&mut host);
    assert_eq!(helper.retained_effect(), None);
    commands(&host, &[creation()]);
    helper.init(&mut host);
    assert_eq!(helper.retained_effect(), None);
    commands(&host, &[creation(), creation()]);
}

#[test]
fn l4_failure_with_effect_retains_identity() {
    let mut helper = LocalGridHelper::<u32>::new();
    let mut host = Fixture::default();
    host.factory.push_back((Some(E), 0));
    helper.init(&mut host);
    helper.init(&mut host);
    commands(&host, &[creation()]);
    assert_eq!(helper.retained_effect(), Some(E));
}

struct Unreadable;
impl Position for Unreadable {
    fn read_x(&mut self) -> u32 {
        panic!("null H read x")
    }
    fn read_y(&mut self) -> u32 {
        panic!("null H read y")
    }
    fn read_z(&mut self) -> u32 {
        panic!("null H read z")
    }
}

#[test]
fn l5_null_guard_precedes_all_inputs() {
    let mut helper = LocalGridHelper::<u32>::new();
    let mut host = Fixture::default();
    helper.show_effect(&mut host, 1);
    helper.show_effect(&mut host, 0);
    assert_eq!(
        helper.set_position(&mut host, &mut Unreadable, MODE),
        Ok(PositionOutcome::NullEffect)
    );
    helper.shutdown(&mut host);
    helper.destroy(&mut host);
    commands(&host, &[]);
}

#[test]
fn l6_show_fresh_queries() {
    let (helper, mut host) = live();
    host.queries.extend([0, 1]);
    helper.show_effect(&mut host, 1);
    helper.show_effect(&mut host, 1);
    commands(&host, &[Event::Query(E), Event::Start(E, 0), Event::Query(E)]);
    assert_eq!(helper.retained_effect(), Some(E));
}

#[test]
fn l7_hide_fresh_queries() {
    let (helper, mut host) = live();
    host.queries.extend([1, 0]);
    helper.show_effect(&mut host, 0);
    helper.show_effect(&mut host, 0);
    commands(&host, &[Event::Query(E), Event::Stop(E, 0), Event::Query(E)]);
}

#[test]
fn l8_repeated_start_uses_reply_not_cache() {
    let (helper, mut host) = live();
    host.queries.extend([0, 0]);
    helper.show_effect(&mut host, 1);
    helper.show_effect(&mut host, 1);
    commands(
        &host,
        &[Event::Query(E), Event::Start(E, 0), Event::Query(E), Event::Start(E, 0)],
    );
}

#[test]
fn l9_request_low_byte_and_full_zero_argument() {
    let (helper, mut host) = live();
    host.queries.extend([1, 0x100]); // AL=0 despite high bits in second reply
    helper.show_effect(&mut host, 0x100);
    helper.show_effect(&mut host, 2);
    commands(
        &host,
        &[Event::Query(E), Event::Stop(E, 0), Event::Query(E), Event::Start(E, 0)],
    );
}

#[test]
fn l10_hidden_and_unchanged_cell_still_send_every_call() {
    let (helper, mut host) = live();
    host.queries.push_back(0);
    helper.show_effect(&mut host, 0);
    commands(&host, &[Event::Query(E)]);
    geometry(&helper, &mut host, G1);
    geometry(&helper, &mut host, G1);
    geometry(
        &helper,
        &mut host,
        Vector {
            position: [0x418e_0000, G1.position[1], G1.position[2]],
            floors: [(71.0 / 64.0, 1.0), G1.floors[1]],
            ..G1
        },
    );
    geometry(
        &helper,
        &mut host,
        Vector {
            position: [0x4200_0000, G1.position[1], G1.position[2]],
            floors: [(2.0, 2.0), G1.floors[1]],
            expected: [0x4204_0000, 0xc050_0000, 0xc170_0000, 0x4380_0000],
            ..G1
        },
    );
}

#[test]
fn l11_shown_move_same_geometry() {
    let (helper, mut host) = live();
    host.queries.push_back(0);
    helper.show_effect(&mut host, 1);
    commands(&host, &[Event::Query(E), Event::Start(E, 0)]);
    geometry(&helper, &mut host, G1);
}

#[test]
fn l12_shutdown_clear_before_release_then_no_calls() {
    let (mut helper, mut host) = live();
    host.queries.push_back(1);
    helper.shutdown(&mut host);
    assert_eq!(helper.retained_effect(), None);
    helper.shutdown(&mut host);
    helper.destroy(&mut host);
    commands(&host, &[Event::Query(E), Event::Stop(E, 0), Event::Release(E, None)]);
}

#[test]
fn l13_inactive_shutdown_then_new_identity() {
    let (mut helper, mut host) = live();
    host.queries.push_back(0x100); // only AL is used
    helper.shutdown(&mut host);
    host.factory.push_back((Some(F), 1));
    helper.init(&mut host);
    assert_eq!(helper.retained_effect(), Some(F));
    commands(&host, &[Event::Query(E), Event::Release(E, None), creation()]);
}

#[test]
fn l14_destroy_releases_only_without_clearing_field() {
    let (helper, mut host) = live();
    helper.destroy(&mut host);
    commands(&host, &[Event::Release(E, Some(E))]);
}

#[test]
fn unsupported_mode_has_empty_prefix_and_null_guard_still_wins() {
    let (helper, mut host) = live();
    let error = helper
        .set_position(&mut host, &mut Unreadable, NumericMode::Unqualified)
        .unwrap_err();
    assert_eq!(error.reason, UnsupportedReason::NumericMode);
    assert!(error.reached.is_empty());
    commands(&host, &[]);
    let null = LocalGridHelper::<u32>::new();
    assert_eq!(
        null.set_position(&mut host, &mut Unreadable, NumericMode::Unqualified),
        Ok(PositionOutcome::NullEffect)
    );
}

#[test]
fn unsupported_scalar_at_each_request_preserves_reached_prefix() {
    let prefix = [
        Step::CaptureService,
        Step::ReadX,
        Step::Scalar(1),
        Step::Floor(1),
        Step::Scalar(2),
        Step::ReadY,
        Step::ReadZ,
        Step::Scalar(3),
        Step::Floor(2),
        Step::Scalar(4),
        Step::Scalar(5),
    ];
    for (index, end) in [(1, 3), (2, 5), (3, 8), (4, 10), (5, 11)] {
        for invalid in [f64::NAN, f64::INFINITY, 65537.0, 1.0 / 2147483648.0] {
            let (helper, mut host) = live();
            let mut values = G1.scalars;
            values[index - 1] = invalid;
            host.scalars.extend(values);
            host.floors.extend(G1.floors);
            let mut position = G1.position;
            let error = helper.set_position(&mut host, &mut position, MODE).unwrap_err();
            assert_eq!(error.reason, UnsupportedReason::Scalar(index as u8));
            assert_eq!(error.reached, prefix[..end]);
            assert!(!host.events.borrow().iter().any(|e| matches!(e, Event::Transform(..))));
            assert_eq!(helper.retained_effect(), Some(E));
        }
    }
}

#[test]
fn unsupported_nonpower_divisor_and_invalid_floor_are_not_successful_noops() {
    let (helper, mut host) = live();
    host.scalars.push_back(3.0);
    let mut position = G1.position;
    let error = helper.set_position(&mut host, &mut position, MODE).unwrap_err();
    assert_eq!(error.reason, UnsupportedReason::Scalar(1));
    assert_eq!(error.reached, [Step::CaptureService, Step::ReadX, Step::Scalar(1)]);
    for reply in [2.0, 0.5, f64::NEG_INFINITY] {
        let (helper, mut host) = live();
        host.scalars.push_back(16.0);
        host.floors.push_back((35.0 / 32.0, reply));
        let mut position = G1.position;
        let error = helper.set_position(&mut host, &mut position, MODE).unwrap_err();
        assert_eq!(error.reason, UnsupportedReason::FloorReply(1));
        assert_eq!(
            error.reached,
            [Step::CaptureService, Step::ReadX, Step::Scalar(1), Step::Floor(1)]
        );
    }
}

#[test]
fn unsupported_positions_stop_at_the_actual_load_prefix() {
    for x in [0x4780_0080, 0x7f80_0000, 0x7fc0_0001, 1] {
        let (helper, mut host) = live();
        let mut position = [x, G1.position[1], G1.position[2]];
        let error = helper.set_position(&mut host, &mut position, MODE).unwrap_err();
        assert_eq!(error.reason, UnsupportedReason::Position);
        assert_eq!(error.reached, [Step::CaptureService, Step::ReadX]);
    }
    for (y, z) in [(0x7fc0_0000, G1.position[2]), (0, 1), (0, 0xff80_0000)] {
        let (helper, mut host) = live();
        host.scalars.extend([16.0, 16.0]);
        host.floors.push_back(G1.floors[0]);
        let mut position = [G1.position[0], y, z];
        let error = helper.set_position(&mut host, &mut position, MODE).unwrap_err();
        assert_eq!(error.reason, UnsupportedReason::Position);
        assert_eq!(
            error.reached,
            [
                Step::CaptureService,
                Step::ReadX,
                Step::Scalar(1),
                Step::Floor(1),
                Step::Scalar(2),
                Step::ReadY,
                Step::ReadZ
            ]
        );
    }
}

#[test]
fn qualified_extreme_divisors_and_finest_offsets() {
    let (helper, mut host) = live();
    geometry(
        &helper,
        &mut host,
        Vector {
            position: [0xc780_0000, 0x0080_0001, 0x4780_0000],
            scalars: [
                1.0 / 65536.0,
                1.0 / 1073741824.0,
                65536.0,
                -1.0 / 1073741824.0,
                -65536.0,
            ],
            floors: [(-4294967296.0, -4294967296.0), (1.0, 1.0)],
            expected: [0xc780_0000, 0x0080_0001, 0x4780_0000, 0xc980_0000],
            flags: 3,
        },
    );
}
