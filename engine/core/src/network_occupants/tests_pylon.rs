use super::pylon::*;
use super::xform::{Xform, XFORM_SCALE, XFORM_TRANSLATION};
use crate::automata::OccupantBase;
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue, SaveRegistry};
use std::{cell::RefCell, rc::Rc};

fn record() -> PylonRecord {
    PylonRecord {
        base: OccupantBase::new(0xA9B6_D7DE),
        transform_5c: Xform {
            mask: XFORM_TRANSLATION,
            translation: [1.0, -2.0, 3.0],
            ..Xform::default()
        },
        bytes_60_64_68: [0x12, 0x34, 0x56],
        byte_198: 0x78,
        field_140: 0x8765_4321,
        source_6c: RefValue::Host {
            token: 11,
            class_id: 0xC9C0_5C6E,
        },
        transform_70: Xform {
            mask: XFORM_SCALE,
            scale: 2.5,
            ..Xform::default()
        },
        byte_199: 0x9A,
        field_19c: 0xDEAD_BEEF,
        field_1a0: -0x0123_4567_89AB_CDEF,
        reference_1b4: RefValue::Host {
            token: 22,
            class_id: 0x49B6_D69B,
        },
    }
}

struct Resolver(Vec<(u32, u32)>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class: u32, id: u32) -> Option<RefValue> {
        self.0.push((class, id));
        Some(RefValue::Host {
            token: u64::from(id),
            class_id: class,
        })
    }
}

#[test]
fn v5_record_order_both_transforms_and_references() {
    let source = record();
    let mut out = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
    assert!(source.write(&mut out));
    let bytes = out.into_bytes();
    let mut reader = ByteReader::new(&bytes);
    assert_eq!(reader.get_u16(), Some(5));
    OccupantBase::read(&mut reader).unwrap();
    let tail = reader.position();
    // Independently inspect the written field boundaries, including two refs.
    assert_eq!(&bytes[tail..tail + 1], &[1]);
    assert_eq!(reader.get_u8(), Some(1));
    assert_eq!(reader.get_f32(), Some(1.0));
    assert_eq!(reader.get_f32(), Some(-2.0));
    assert_eq!(reader.get_f32(), Some(3.0));
    assert_eq!(reader.get_bytes(4).unwrap(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(reader.get_u32(), Some(0x8765_4321));
    assert_eq!(reader.get_u32(), Some(1));
    assert_eq!(reader.get_u32(), Some(0xC9C0_5C6E));
    assert_eq!(reader.get_u8(), Some(2));
    assert_eq!(reader.get_f32(), Some(2.5));
    assert_eq!(reader.get_u8(), Some(0x9A));
    assert_eq!(reader.get_u32(), Some(0xDEAD_BEEF));
    assert_eq!(reader.get_i64(), Some(-0x0123_4567_89AB_CDEF));
    assert_eq!(reader.get_u32(), Some(2));
    assert_eq!(reader.get_u32(), Some(0x49B6_D69B));
    assert_eq!(reader.remaining(), 0);
    let mut resolver = Resolver(vec![]);
    let mut input = ByteReader::with_resolver(&bytes, &mut resolver);
    let mut back = record();
    back.transform_5c = Xform::default();
    back.transform_70 = Xform::default();
    back.field_140 = 0;
    back.field_1a0 = 0;
    assert!(back.read_into(&mut input).is_some());
    assert_eq!(input.remaining(), 0);
    assert_eq!(back.transform_5c.translation, [1.0, -2.0, 3.0]);
    assert_eq!(back.transform_70.scale, 2.5);
    assert_eq!(back.field_140, source.field_140);
    assert_eq!(back.field_1a0, source.field_1a0);
    assert_eq!(resolver.0, [(0xC9C0_5C6E, 1), (0x49B6_D69B, 2)]);
}

fn fixture(version: u16) -> Vec<u8> {
    let mut out = ByteWriter::new();
    out.put_u16(version);
    OccupantBase::new(0xA9B6_D7DE).write(&mut out);
    let transform = |out: &mut ByteWriter, translation: [f32; 3], scale: f32| {
        if version < 4 {
            for value in super::xform::IDENTITY_MATRIX {
                out.put_f32(value);
            }
            for value in translation {
                out.put_f32(value);
            }
            out.put_f32(scale);
            out.put_u32(0x1234_5678);
        } else {
            // Current transform, encoded independently of Xform::write.
            out.put_u8(3);
            for value in translation {
                out.put_f32(value);
            }
            out.put_f32(scale);
        }
    };
    transform(&mut out, [10.0, 20.0, 30.0], 4.0);
    out.put_bytes(&[1, 2, 3, 4]);
    out.put_u32(5);
    out.put_u32(0); // source null
    transform(&mut out, [-10.0, -20.0, -30.0], 6.0);
    out.put_u8(7);
    out.into_bytes()
}

#[test]
fn legacy_v3_and_current_v4_preserve_absent_tail() {
    for version in [3, 4] {
        let bytes = fixture(version);
        let mut input = ByteReader::new(&bytes);
        let mut back = record();
        assert!(back.read_into(&mut input).is_some());
        assert_eq!(input.remaining(), 0);
        assert_eq!(back.transform_5c.translation, [10.0, 20.0, 30.0]);
        assert_eq!(back.transform_70.translation, [-10.0, -20.0, -30.0]);
        assert_eq!(back.transform_5c.scale, 4.0);
        assert_eq!(back.transform_70.scale, 6.0);
        if version == 3 {
            assert_eq!(back.transform_5c.counter, 0x78);
            assert_eq!(back.transform_70.counter, 0x78);
        }
        assert_eq!(back.field_19c, 0xDEAD_BEEF);
        assert_eq!(back.field_1a0, -0x0123_4567_89AB_CDEF);
        assert!(back.source_6c.is_null());
        assert!(matches!(back.reference_1b4, RefValue::Host { token: 22, .. }));
    }
    let bytes = [2, 0, 99, 99];
    let mut input = ByteReader::new(&bytes);
    assert!(record().read_into(&mut input).is_none());
    assert_eq!(input.position(), 2);
}

#[derive(Default)]
struct Host {
    events: Vec<u32>,
    initialized: bool,
}

impl PylonLifecycleHost for Host {
    fn is_initialized(&mut self, _: &OccupantBase) -> bool {
        self.events.push(1);
        self.initialized
    }
    fn occupant_init(&mut self, _: &mut OccupantBase) {
        self.events.push(2);
        self.initialized = true;
    }
    fn subscribe_message(&mut self, message: u32) {
        self.events.push(message);
    }
    fn subscribe_animation(&mut self, class: u32, iid: u32, tick: u32) {
        assert_eq!((class, iid, tick), (0x86F9_FB4D, 0x86F9_FB4C, 0x80));
        self.events.push(tick);
    }
    fn occupant_shutdown(&mut self, _: &mut OccupantBase) {
        self.events.push(3);
    }
    fn city_handle(&mut self) -> u32 {
        self.events.push(4);
        123
    }
    fn remove_occupant(&mut self, handle: u32) {
        assert_eq!(handle, 123);
        self.events.push(5);
    }
    fn unsubscribe_message(&mut self, message: u32) {
        self.events.push(message);
    }
}

#[test]
fn lifecycle_order_inserted_and_uninserted() {
    for inserted in [false, true] {
        let mut pylon = Pylon::from_state(record(), 11, 22, u8::from(inserted), RefValue::Null);
        let mut host = Host::default();
        let mut globals = PylonGlobals::default();
        pylon.init_published_lifecycle(&mut globals, &mut host);
        assert!(globals.field_b23b78);
        assert_eq!(host.events, [1, 2, 0xA6B7_9602, 0x06CD_B65B, 0x26D3_1EC1, 0x80]);
        host.events.clear();
        pylon.init_published_lifecycle(&mut globals, &mut host);
        assert_eq!(host.events, [1]);
        host.events.clear();
        pylon.shutdown_published_lifecycle(&mut host);
        let mut expected = vec![3];
        if inserted {
            expected.extend([4, 5]);
        }
        expected.extend(PYLON_MESSAGES);
        assert_eq!(host.events, expected);
        assert!(pylon.reference_78.is_null());
        // Do not conflate the contradictory source offsets; NO37.
        assert!(!pylon.record.source_6c.is_null());
    }
}

#[test]
fn type_and_call_drag_state() {
    let mut rec = record();
    rec.base.occupant_type = 0x7475_8925;
    let mut pylon = Pylon::from_state(rec, 11, 22, 0, RefValue::Null);
    assert_eq!(pylon.record.base.occupant_type, 0xA9B6_D7DE);
    assert_eq!(pylon.get_call_number(), 11);
    assert_eq!(pylon.get_drag_state(), 22);
    pylon.set_call_number(0xFFFF_FFFE);
    pylon.set_drag_state(0xFFFF_FFFD);
    assert_eq!(pylon.get_call_number(), 0xFFFF_FFFE);
    assert_eq!(pylon.get_drag_state(), 0xFFFF_FFFD);
}

#[test]
fn truncated_records_fail_at_every_boundary() {
    let mut rec = record();
    rec.source_6c = RefValue::Null;
    rec.reference_1b4 = RefValue::Null;
    let mut out = ByteWriter::new();
    assert!(rec.write(&mut out));
    for end in 0..out.len() {
        let mut back = record();
        assert!(back.read_into(&mut ByteReader::new(&out.bytes()[..end])).is_none());
    }
}

#[test]
fn failed_reference_read_has_already_released_old_source() {
    let bytes = fixture(4);
    let mut reader = ByteReader::new(&bytes);
    reader.get_u16().unwrap();
    OccupantBase::read(&mut reader).unwrap();
    // Current-form transform: mask 3, translation and scale; then 4 bytes/u32.
    assert!(reader.skip(17 + 4 + 4));
    let end = reader.position();
    let mut back = record();
    assert!(back.read_into(&mut ByteReader::new(&bytes[..end])).is_none());
    assert!(back.source_6c.is_null());
    assert!(!back.reference_1b4.is_null());
}
