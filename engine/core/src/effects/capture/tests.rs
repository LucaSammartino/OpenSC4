// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects::{ComponentRecord, ParamBinding};

#[derive(Clone)]
struct Image(Vec<u8>);
impl Memory for Image {
    fn read(&mut self, address: u32, bytes: &mut [u8]) -> bool {
        let Some(source) = self.0.get(address as usize..address as usize + bytes.len()) else {
            return false;
        };
        bytes.copy_from_slice(source);
        true
    }
}
impl Image {
    fn put(&mut self, address: usize, value: u32) {
        self.0[address..address + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn vector(&mut self, address: usize, begin: u32, length: u32) {
        self.put(address, begin);
        self.put(address + 4, begin + length);
        self.put(address + 8, begin + length + 16);
    }
    fn string(&mut self, address: usize, begin: u32, value: &[u8]) {
        self.vector(address, begin, value.len() as u32);
        self.0[begin as usize..begin as usize + value.len()].copy_from_slice(value);
    }
}

fn fixture() -> (Image, EffectDirectory) {
    let mut d = crate::effects::shipped::volcano_directory();
    let mut component = ComponentRecord::new(3, 7);
    component.name = "brush".into();
    component.render[0] = f32::from_bits(0x7fc01234);
    let mut e = EffectDescription::new(3, vec![component]);
    e.text = "payload".into();
    e.bindings = vec![ParamBinding {
        index: 1,
        name: "shake".into(),
        value: -0.0,
        extra: 5,
    }];
    d.effects = vec![e];
    d.names = vec![("duplicate".into(), 0), ("duplicate".into(), 0)];
    let e = &d.effects[0];
    let mut m = Image(vec![0; 4096]);
    m.vector(0x118, 0x200, 56);
    m.put(0x200, e.flags);
    m.put(0x204, e.priority);
    m.vector(0x208, 0x300, 108);
    m.vector(0x214, 0x400, 24);
    for (i, value) in e.extra.iter().enumerate() {
        m.put(0x220 + i * 4, *value);
    }
    m.string(0x22c, 0x500, b"payload");
    let c = &e.components[0];
    m.string(0x300, 0x520, b"brush");
    m.0[0x30c] = c.kind;
    m.put(0x310, c.flags);
    let p = Placement::decode(&c.placement);
    m.0[0x314] = p.revision;
    m.0[0x315] = p.flags;
    m.0[0x318..0x34c].copy_from_slice(&c.placement[..52]);
    m.0[0x34c] = c.lod_min;
    m.0[0x34d] = c.lod_max;
    m.0[0x34e..0x350].copy_from_slice(&c.copies.to_le_bytes());
    m.0[0x350..0x352].copy_from_slice(&c.spacing.to_le_bytes());
    for (i, value) in c.render.iter().enumerate() {
        m.put(0x354 + i * 4, value.to_bits());
    }
    m.put(0x368, c.index);
    m.put(0x400, 1);
    m.string(0x404, 0x540, b"shake");
    m.put(0x410, (-0.0f32).to_bits());
    m.put(0x414, 5);
    m.vector(0x108, 0x600, 8);
    m.put(0x114, 2);
    m.put(0x600, 0x700);
    m.put(0x604, 0x720);
    m.string(0x704, 0x800, b"duplicate");
    m.string(0x724, 0x820, b"duplicate");
    (m, d)
}

#[test]
fn payload_bits_duplicates_and_allocation_details() {
    let (mut m, d) = fixture();
    let exact = compare_visuals(m.clone(), 0x100, &d).unwrap();
    assert_eq!(exact.records, 1);
    assert_eq!(exact.differences, 0);
    assert!(exact.fields > 30);
    // Padding, spare capacity and bucket order do not change values.
    m.0[0x316] = 123;
    m.0[0x352] = 99;
    m.put(0x600, 0x720);
    m.put(0x604, 0x700);
    assert_eq!(compare_visuals(m.clone(), 0x100, &d).unwrap().differences, 0);
    m.put(0x410, 0); // Signed zero matters, as does the NaN payload above.
    assert_eq!(compare_visuals(m, 0x100, &d).unwrap().differences, 1);
}

#[test]
fn malformed_capture_is_never_exact() {
    let (m, d) = fixture();
    for (address, value) in [
        (0x700, 0x700),
        (0x710, 1),
        (0x114, 1),
        (0x11c, 0x237),
        (0x208, 0xfffffff0),
    ] {
        let mut bad = m.clone();
        bad.put(address, value);
        assert!(compare_visuals(bad, 0x100, &d).is_none(), "address {address:x}");
    }
}

#[test]
fn empty_vector_spare_capacity_and_missing_records() {
    let (mut m, mut d) = fixture();
    m.vector(0x118, 0x200, 0);
    m.vector(0x108, 0x600, 0);
    m.put(0x114, 0);
    assert!(compare_visuals(m.clone(), 0x100, &d).unwrap().differences > 0);
    d.effects.clear();
    d.names.clear();
    assert_eq!(compare_visuals(m, 0x100, &d).unwrap().differences, 0);
}

#[test]
fn ef21_pointer_null_misalignment_and_unreadable_nested_payload_refuse() {
    use super::payload::copy_builtins;
    let mut m = Image(vec![0; 0x4000]);
    m.vector(0x138, 0x1000, 4);
    m.put(0x1000, 0x2000);
    assert!(copy_builtins(m.clone(), 0x100).is_some());
    for (at, value) in [
        (0x1000, 0),
        (0x13c, 0x1003),
        (0x1000, 0xfffffff0),
        (0x2088, 0x3fff),
        (0x208c, 0x4003),
    ] {
        let mut bad = m.clone();
        bad.put(at, value);
        assert!(copy_builtins(bad, 0x100).is_none(), "at={at:x}");
    }
    assert!(copy_builtins(m, u32::MAX - 3).is_none());
}

#[test]
fn ef21_capture_has_aggregate_byte_budget() {
    let mut r = Reader::new(Image(vec![0; 16]));
    r.remaining = 7;
    assert!(r.bytes(1, 4).is_some());
    assert!(r.bytes(1, 4).is_none());
}

fn sequence_fixture(flags: u32, named: bool) -> Image {
    // Exactly the EF23 object extent is readable: no byte at D+1C exists.
    let mut m = Image(vec![0; 0x201c]);
    m.vector(0x1c4, 0x1000, 4 * 0x20);
    m.vector(0x1074, 0x1800, 4);
    m.put(0x1800, 0x2000);
    m.put(0x2018, flags);
    if named {
        m.vector(0x1064, 0x1900, 4);
        m.put(0x1070, 1);
        m.put(0x1900, 0x1920);
        m.string(0x1924, 0x1940, b"sequence");
    }
    m
}

#[test]
fn ef23_complete_named_and_anonymous_sequence_flags() {
    use super::payload::{copy_builtins, Kind, Value};
    for named in [false, true] {
        for flags in [0, 7, 0x80000005] {
            let capture = copy_builtins(sequence_fixture(flags, named), 0x100).unwrap();
            assert!(capture.supported());
            let sequence = capture.families.iter().find(|f| f.kind == Kind::Component(6)).unwrap();
            assert_eq!(sequence.names.len(), usize::from(named));
            let payload = sequence.slots[0].payload.as_ref().unwrap();
            assert_eq!(payload.get(&0x18), Some(&Value::Word(flags)));
            assert_eq!(payload.get(&0xc), Some(&Value::Rows(vec![])));
        }
    }
    let full = copy_builtins(sequence_fixture(0x80000005, false), 0x100).unwrap();
    let masked = copy_builtins(sequence_fixture(5, false), 0x100).unwrap();
    assert_ne!(full, masked);
}

#[test]
fn ef23_vector_only_sequence_is_not_complete() {
    use super::payload::copy_builtins;
    for readable in [0x18, 0x1b] {
        let mut m = sequence_fixture(7, false);
        m.0.truncate(0x2000 + readable);
        assert!(copy_builtins(m, 0x100).is_none());
    }
}

#[test]
fn ef21_resource_repeated_leases_and_released_publication_refuse() {
    use super::observation::{Event, Observation};
    let mut o = Observation::attach(1, true);
    let request = Event::Request {
        key: [1, 2, 3],
        iid: 4,
        success: true,
        resource: Some(100),
    };
    o.record(1, request.clone());
    o.record(1, request);
    o.record(1, Event::Release(100));
    assert!(o.borrowed(100));
    o.record(1, Event::Release(100));
    assert!(!o.borrowed(100));
    o.record(1, Event::PublishBefore(100));
    assert!(!o.history_available());
    assert!(!o.retained_available(1, true));
}
