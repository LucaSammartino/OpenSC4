// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Independent public §6 vectors 19–24. Literal expectations precede calls;
//! execution is portable-only, never a native output oracle.
use super::*;
use crate::variant::Elem;
use retained::{Property, RawTuningSource, ReadEvent, RetainedTuning, RippleProcess, Scalar, Target, Unsupported};
use std::{collections::BTreeMap, sync::Mutex};
use storage::{Assignment, Buffer, Event as StorageEvent, Kind, Ledger, Shore, Strip, StripList};

struct Raw {
    tag: u16,
    scalar: Option<Elem>,
    bytes: Vec<u8>,
    count: usize,
}
struct Props {
    whole: bool,
    values: BTreeMap<u32, Raw>,
    calls: Mutex<Vec<u32>>,
    releases: AtomicUsize,
    converted: Option<Scalar>,
}
impl Props {
    fn valid() -> Self {
        let mut result = Self {
            whole: true,
            values: BTreeMap::new(),
            calls: Mutex::new(Vec::new()),
            releases: AtomicUsize::new(0),
            converted: None,
        };
        result.array(2, &[2., 3., 4., 5., 6., 99.]);
        result.array(11, &[60., 60., 60., 60., 60., 99.]);
        result
    }
    fn array(&mut self, suffix: u32, values: &[f32]) {
        self.values.insert(
            suffix,
            Raw {
                tag: 0x800A,
                scalar: None,
                bytes: values.iter().flat_map(|v| v.to_bits().to_le_bytes()).collect(),
                count: values.len(),
            },
        );
    }
    fn scalar(&mut self, suffix: u32, scalar: Elem) {
        self.values.insert(
            suffix,
            Raw {
                tag: scalar.mem_type(),
                scalar: Some(scalar),
                bytes: Vec::new(),
                count: 0,
            },
        );
    }
}
impl RawTuningSource for Props {
    fn acquire(&self, key: [u32; 3]) -> bool {
        assert_eq!(key, RIPPLE_EXEMPLAR);
        self.whole
    }
    fn property(&self, id: u32) -> Option<Property<'_>> {
        self.calls.lock().unwrap().push(id);
        self.values.get(&(id - 0x491C_9C00)).map(|v| Property {
            tag: v.tag,
            scalar: v.scalar,
            backing: &v.bytes,
            count: v.count,
        })
    }
    fn release(&self) {
        self.releases.fetch_add(1, Ordering::SeqCst);
    }
    fn foreign_scalar(&self, _: u32, _: Target) -> Result<Option<Scalar>, Unsupported> {
        Ok(self.converted)
    }
}

#[test]
fn scalar_literals_not_float_nonzero_or_saturating_unsigned() {
    let rows = [
        (Elem::F32(0.75), false, 2),
        (Elem::F32(-1.75), true, 0xFFFF_FFFF),
        (Elem::I32(-2), true, 0xFFFF_FFFE),
        (Elem::U64(0x0000_0001_0000_0000), true, 2),
        (Elem::U64(0x0000_0001_0000_0007), true, 7),
    ];
    for (input, boolean, minimum) in rows {
        let mut raw = Props::valid();
        raw.scalar(0, input);
        raw.scalar(1, input);
        let mut state = RetainedTuning::default();
        state.read(&raw, arithmetic(), &mut Vec::new()).unwrap();
        assert_eq!((state.enabled, state.minimum), (boolean, minimum));
        assert_eq!(raw.values[&0].scalar, Some(input));
        assert_eq!(raw.values[&1].scalar, Some(input));
    }
}

#[test]
fn all_numeric_tags_have_the_bound_source_gates() {
    let rows = [
        (Elem::Bool(true), 1, 1.),
        (Elem::U8(255), 255, 255.),
        (Elem::I8(-1), u32::MAX, -1.),
        (Elem::U16(65535), 65535, 65535.),
        (Elem::I16(-2), 0xFFFF_FFFE, -2.),
        (Elem::U32(0x8000_0001), 0x8000_0001, 2147483648.),
        (Elem::I32(-3), 0xFFFF_FFFD, -3.),
        (Elem::U64(16777217), 16777217, 16777216.),
        (Elem::I64(16777217), 16777217, 16777216.),
        (Elem::F32(-1.75), u32::MAX, -1.75),
        (Elem::F64(-1.75), u32::MAX, -1.75),
        (Elem::Char(0xE9), 0xFFFF_FFE9, -23.),
        (Elem::UnicodeChar(0xFFE9), 65513, 65513.),
        (Elem::RzChar(0xE9), 0xFFFF_FFE9, -23.),
    ];
    for (input, integer, float) in rows {
        let mut raw = Props::valid();
        raw.scalar(12, input);
        raw.scalar(3, input);
        let mut state = RetainedTuning::default();
        state.read(&raw, arithmetic(), &mut Vec::new()).unwrap();
        assert_eq!(state.edit_area, integer);
        assert_eq!(state.height, float);
    }
}

#[test]
fn absent_minimum_clamps_and_missing_whole_exemplar_does_nothing() {
    for (old, expected) in [(1, 2), (50, 50)] {
        let mut state = RetainedTuning {
            minimum: old,
            ..Default::default()
        };
        let mut raw = Props::valid();
        raw.whole = false;
        let before = state.clone();
        let mut events = Vec::new();
        state.read(&raw, arithmetic(), &mut events).unwrap();
        assert_eq!(state, before);
        assert_eq!(events, [ReadEvent::Acquire(false)]);
        assert_eq!(raw.releases.load(Ordering::SeqCst), 0);
        raw.whole = true;
        state.read(&raw, arithmetic(), &mut Vec::new()).unwrap();
        assert_eq!(state.minimum, expected);
        assert_eq!(
            *raw.calls.lock().unwrap(),
            (0..14).map(|s| 0x491C_9C00 + s).collect::<Vec<_>>()
        );
        assert_eq!(raw.releases.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn sixth_element_and_source_bits_survive_repeated_absence_transform() {
    let first_squared = vec![4., 9., 16., 25., 36., 99.];
    let first_step = [1. / 32., 3. / 64., 1. / 16., 5. / 64., 3. / 32.];
    let second_squared = vec![16., 81., 256., 625., 1296., 99.];
    let second_step = [1. / 16., 9. / 64., 1. / 4., 25. / 64., 9. / 16.];
    let mut raw = Props::valid();
    let source_before = raw.values[&2].bytes.clone();
    let mut process = RippleProcess::default();
    process.eligible_init(&raw, arithmetic()).unwrap();
    assert_eq!(process.tuning.floats[0], first_squared);
    assert_eq!(process.tuning.step, first_step);
    assert_eq!(process.tuning.floats[5], vec![30., 30., 30., 30., 30., 99.]);
    assert_eq!(raw.values[&2].bytes, source_before);
    let before = process.tuning.clone();
    process.reset();
    assert_eq!(process.tuning, before);
    raw.values.remove(&2);
    raw.values.remove(&11);
    process.eligible_init(&raw, arithmetic()).unwrap();
    assert_eq!(process.tuning.floats[0], second_squared);
    assert_eq!(process.tuning.step, second_step);
    assert_eq!(process.tuning.floats[5], vec![15., 15., 15., 15., 15., 99.]);
    raw.whole = false;
    process.reset();
    let before = process.tuning.clone();
    process.eligible_init(&raw, arithmetic()).unwrap();
    assert_eq!(process.tuning, before);
}

#[test]
fn present_empty_and_wrong_width_are_unsupported_after_earlier_mutations() {
    for bytes in [Vec::new(), vec![0; 3]] {
        let mut raw = Props::valid();
        raw.scalar(0, Elem::Bool(false));
        raw.values.insert(
            2,
            Raw {
                tag: 0x8002,
                scalar: None,
                count: if bytes.is_empty() { 0 } else { 5 },
                bytes,
            },
        );
        let mut process = RippleProcess::default();
        let expected = if raw.values[&2].count == 0 {
            Unsupported::PostTransformIndices
        } else {
            Unsupported::InvalidBacking
        };
        assert_eq!(process.eligible_init(&raw, arithmetic()), Err(expected));
        assert!(process.was_read());
        assert!(!process.tuning.enabled);
        assert_eq!(raw.releases.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn raw_array_copy_has_no_tag_conversion_and_copies_bytes() {
    let mut raw = Props::valid();
    raw.array(4, &[f32::from_bits(0x7FC0_1234), -0.0, 9., 8., 7., 6.]);
    raw.values.get_mut(&4).unwrap().tag = 0x8002; // valid raw backing despite tag
    raw.values.insert(
        8,
        Raw {
            tag: 0x800A,
            scalar: None,
            bytes: vec![1, 2, 3, 4, 5, 6],
            count: 6,
        },
    );
    let mut state = RetainedTuning::default();
    state.read(&raw, arithmetic(), &mut Vec::new()).unwrap();
    assert_eq!(
        state.floats[1].iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [
            0x7FC0_1234,
            0x8000_0000,
            0x4110_0000,
            0x4100_0000,
            0x40E0_0000,
            0x40C0_0000
        ]
    );
    assert_eq!(state.bytes[0], [1, 2, 3, 4, 5, 6]);
}

#[test]
fn temporary_destroyed_on_success_and_foreign_false_output_retained() {
    for success in [false, true] {
        let mut raw = Props::valid();
        raw.values.insert(
            0,
            Raw {
                tag: 0x8010,
                scalar: None,
                bytes: vec![],
                count: 0,
            },
        );
        raw.converted = success.then_some(Scalar::Bool(false));
        let mut events = Vec::new();
        let mut state = RetainedTuning::default();
        state.read(&raw, arithmetic(), &mut events).unwrap();
        assert_eq!(state.enabled, !success);
        assert_eq!(
            &events[2..4],
            &[
                ReadEvent::Temporary(Target::Bool),
                ReadEvent::DestroyTemporary(Target::Bool)
            ]
        );
        assert_eq!(events.last(), Some(&ReadEvent::Release));
    }
}

#[test]
fn process_shared_between_instances_and_reset_obeys_initialized_gate() {
    let shared = process();
    let mut raw = Props::valid();
    let ground = Arc::new(Ground::new([1, 1]));
    let factory = Arc::new(Factory::default());
    let mut a = WaterRipple::default();
    let mut b = WaterRipple::default();
    assert_eq!(
        a.init(ground.clone(), factory.clone(), &raw, shared.clone(), arithmetic()),
        Ok(true)
    );
    shared.lock().unwrap().reset();
    assert_eq!(
        a.init(ground.clone(), factory.clone(), &raw, shared.clone(), arithmetic()),
        Ok(false)
    );
    assert!(!shared.lock().unwrap().was_read());
    raw.values.remove(&2);
    raw.values.remove(&11);
    b.init(ground.clone(), factory.clone(), &raw, shared.clone(), arithmetic())
        .unwrap();
    assert_eq!(a.tuning().unwrap().floats[0], vec![16., 81., 256., 625., 1296., 99.]);
    let lookups = raw.calls.lock().unwrap().len();
    a.shutdown();
    a.init(ground, factory, &raw, shared.clone(), arithmetic()).unwrap();
    assert_eq!(raw.calls.lock().unwrap().len(), lookups);
    let events = &shared.lock().unwrap().events;
    assert_eq!(
        &events[..3],
        &[ReadEvent::SetFlag, ReadEvent::RegisterReset, ReadEvent::Acquire(true)]
    );
}

#[test]
fn logical_reference_deletion_does_not_manufacture_shutdown() {
    use lifecycle::{Interface, References, Release};
    let mut r = References::default();
    assert_eq!(r.remove_ref(), 0);
    assert_eq!(r.release(), Release::Delete);
    let mut r = References::default();
    assert_eq!(r.add_ref(), 1);
    assert_eq!(r.release(), Release::Delete);
    let mut r = References::default();
    assert_eq!(r.add_ref(), 1);
    let mut output = Some(Interface::Message);
    assert!(!r.query(true, 99, &mut output));
    assert_eq!(output, Some(Interface::Message));
    assert!(r.query(true, 1, &mut output));
    assert_eq!(output, Some(Interface::Reference));
    assert!(r.query(true, 0x4522_94AA, &mut output));
    assert_eq!(output, Some(Interface::Message));
    assert_eq!(r.count(), 3);
    assert_eq!(r.release(), Release::Count(2));
}

fn payload(word: u32, ledger: &mut Ledger) -> Strip {
    Strip {
        vertices: Buffer::new(Kind::Vertices, &[[word; 6]; 2], ledger),
        rectangles: Buffer::new(Kind::Rectangles, &[[word; 4]; 2], ledger),
        words: [word; 13],
    }
}
fn shore(words: &[u32], ledger: &mut Ledger) -> Shore {
    let points = Buffer::new(Kind::Points, &[[1, 2], [3, 4]], ledger);
    let mut strips = StripList::new(ledger);
    for &word in words {
        // Fixture construction uses allocator identity through a public test-only helper.
        strips.push_owned(payload(word, ledger), ledger);
    }
    Shore {
        points,
        strips,
        words: [9; 10],
    }
}

#[test]
fn deep_copy_is_independent_through_edit_and_source_destruction() {
    let mut ledger = Ledger::default();
    let mut source = shore(&[10, 20], &mut ledger);
    let destination = source.copy(&mut ledger);
    assert_ne!(source.points.id, destination.points.id);
    assert_ne!(source.strips.sentinel, destination.strips.sentinel);
    for (a, b) in source.strips.nodes.iter().zip(&destination.strips.nodes) {
        assert_ne!(a.id, b.id);
        assert_ne!(a.strip.vertices.id, b.strip.vertices.id);
        assert_ne!(a.strip.rectangles.id, b.strip.rectangles.id);
        assert_eq!(a.strip.vertices.records, b.strip.vertices.records);
        assert_eq!(a.strip.words, b.strip.words);
    }
    source.points.records[0] = [99, 99];
    source.strips.nodes[0].strip.vertices.records[0] = [99; 6];
    source.strips.nodes[0].strip.rectangles.records[0] = [99; 4];
    let a = &source.strips.nodes[0];
    let b = &source.strips.nodes[1];
    let expected = [
        a.strip.rectangles.id.unwrap(),
        a.strip.vertices.id.unwrap(),
        a.id,
        b.strip.rectangles.id.unwrap(),
        b.strip.vertices.id.unwrap(),
        b.id,
        source.strips.sentinel,
        source.points.id.unwrap(),
    ];
    let start = ledger.events.len();
    source.destroy(&mut ledger);
    assert_eq!(ledger.events[start..], expected.map(StorageEvent::Free));
    assert_eq!(destination.points.records, [[1, 2], [3, 4]]);
    assert_eq!(destination.strips.nodes[0].strip.vertices.records, [[10; 6]; 2]);
    assert_eq!(destination.strips.nodes[0].strip.rectangles.records, [[10; 4]; 2]);
}

#[test]
fn vertex_assignment_reuses_shrinks_grows_before_free_and_self_is_noop() {
    let mut ledger = Ledger::default();
    let mut dest = Buffer::new(Kind::Vertices, &[[1; 6]; 4], &mut ledger);
    dest.records.truncate(3);
    let id = dest.id;
    let small = Buffer::new(Kind::Vertices, &[[2; 6]; 2], &mut ledger);
    dest.assign(Assignment::Other(&small), &mut ledger);
    assert_eq!((dest.id, dest.records.len(), dest.capacity), (id, 2, 4));
    let count = ledger.events.len();
    dest.assign(Assignment::This, &mut ledger);
    assert_eq!(ledger.events.len(), count);
    assert_eq!((dest.id, dest.capacity), (id, 4));
    let large = Buffer::new(Kind::Vertices, &[[3; 6]; 5], &mut ledger);
    let start = ledger.events.len();
    dest.assign(Assignment::Other(&large), &mut ledger);
    assert_eq!(dest.capacity, 5);
    assert_eq!(dest.records, [[3; 6]; 5]);
    assert_eq!(
        ledger.events[start..],
        [
            StorageEvent::Allocate(dest.id.unwrap(), Kind::Vertices, 120),
            StorageEvent::Free(id.unwrap())
        ]
    );
}

#[test]
fn list_assignment_reuses_prefix_and_erases_rectangle_vertex_node_order() {
    let mut ledger = Ledger::default();
    let mut dest = shore(&[1, 2, 3], &mut ledger);
    let source = shore(&[10, 20], &mut ledger);
    let sentinel = dest.strips.sentinel;
    let ids: Vec<_> = dest.strips.nodes.iter().map(|n| n.id).collect();
    let tail = &dest.strips.nodes[2];
    let expected = [
        tail.strip.rectangles.id.unwrap(),
        tail.strip.vertices.id.unwrap(),
        tail.id,
    ];
    let start = ledger.events.len();
    dest.strips.assign(Assignment::Other(&source.strips), &mut ledger);
    assert_eq!(ledger.events[start..], expected.map(StorageEvent::Free));
    assert_eq!(dest.strips.sentinel, sentinel);
    assert_eq!(dest.strips.nodes.iter().map(|n| n.id).collect::<Vec<_>>(), ids[..2]);
    assert_eq!(
        dest.strips.nodes.iter().map(|n| n.strip.words[0]).collect::<Vec<_>>(),
        [10, 20]
    );
    let count = ledger.events.len();
    dest.strips.assign(Assignment::This, &mut ledger);
    assert_eq!(ledger.events.len(), count);
    let big = shore(&[40, 50, 60], &mut ledger);
    dest.strips.assign(Assignment::Other(&big.strips), &mut ledger);
    assert_eq!(dest.strips.nodes[0].id, ids[0]);
    assert_eq!(dest.strips.nodes[1].id, ids[1]);
    assert_eq!(
        dest.strips.nodes.iter().map(|n| n.strip.words[0]).collect::<Vec<_>>(),
        [40, 50, 60]
    );
}

#[test]
fn insertion_erase_iterator_clear_and_nonempty_pop_contracts() {
    let mut ledger = Ledger::default();
    let mut dest = shore(&[1, 4], &mut ledger);
    let source = shore(&[2, 3], &mut ledger);
    dest.strips.insert_range(1, &source.strips, 0..2, &mut ledger).unwrap();
    assert_eq!(
        dest.strips.nodes.iter().map(|n| n.strip.words[0]).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let mut output = 0;
    let end = dest.strips.nodes[3].id;
    dest.strips.erase(1..3, &mut output, &mut ledger).unwrap();
    assert_eq!(output, end);
    output = 0;
    dest.strips.erase(1..1, &mut output, &mut ledger).unwrap();
    assert_eq!(output, end);
    let sentinel = dest.strips.sentinel;
    dest.strips.remove_last(&mut ledger).unwrap();
    dest.strips.clear(&mut ledger);
    assert_eq!(dest.strips.sentinel, sentinel);
    assert!(dest.strips.nodes.is_empty());
    assert_eq!(
        dest.strips.remove_last(&mut ledger),
        Err(storage::Unsupported::EmptyPop)
    );
    output = 0;
    dest.strips.erase(0..0, &mut output, &mut ledger).unwrap();
    assert_eq!(output, sentinel);
}

#[test]
fn shore_range_assignment_advances_destination_without_replacing_outer_storage() {
    let mut ledger = Ledger::default();
    let source = [shore(&[10], &mut ledger), shore(&[20], &mut ledger)];
    let mut dest = [
        shore(&[1], &mut ledger),
        shore(&[2], &mut ledger),
        shore(&[3], &mut ledger),
    ];
    let sentinel = dest[1].strips.sentinel;
    assert_eq!(storage::assign_shores(&[], &mut dest, 1, &mut ledger), Ok(1));
    assert_eq!(storage::assign_shores(&source, &mut dest, 1, &mut ledger), Ok(3));
    assert_eq!(dest[0].strips.nodes[0].strip.words[0], 1);
    assert_eq!(dest[1].strips.nodes[0].strip.words[0], 10);
    assert_eq!(dest[2].strips.nodes[0].strip.words[0], 20);
    assert_eq!(dest[1].strips.sentinel, sentinel);
}
