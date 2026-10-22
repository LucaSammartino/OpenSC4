// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent literal vectors, recorded before runtime implementation.
use super::*;

#[derive(Default)]
struct Bytes {
    bytes: Vec<u8>,
    position: usize,
    reads: Vec<usize>,
    writes: Vec<usize>,
    fail_read: bool,
    fail_write: bool,
    closes: usize,
}
impl Bytes {
    fn input(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            ..Self::default()
        }
    }
}
impl RawStream for Bytes {
    fn read(&mut self, out: &mut [u8]) -> bool {
        self.reads.push(out.len());
        // This fixture specifies its own atomic short-read behaviour. The
        // runtime may not generalise it to arbitrary native streams.
        if self.fail_read || self.position + out.len() > self.bytes.len() {
            return false;
        }
        out.copy_from_slice(&self.bytes[self.position..self.position + out.len()]);
        self.position += out.len();
        true
    }
    fn write(&mut self, data: &[u8]) -> bool {
        self.writes.push(data.len());
        if self.fail_write {
            return false;
        }
        self.bytes.resize(self.bytes.len().max(self.position + data.len()), 0);
        self.bytes[self.position..self.position + data.len()].copy_from_slice(data);
        self.position += data.len();
        true
    }
    fn position(&self) -> u64 {
        self.position as u64
    }
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn seek(&mut self, position: u64) -> bool {
        self.position = position as usize;
        true
    }
    fn close(&mut self) -> bool {
        self.closes += 1;
        false
    }
}

#[test]
fn independent_integer_widths_signed_bits_and_array_order() {
    for (native_order, expected) in [
        (
            true,
            vec![
                0xfe, 0x80, 0xfe, 0xff, 0x34, 0x12, 0xfe, 0xff, 0xff, 0xff, 0x78, 0x56, 0x34, 0x12, 0xfe, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 8, 7, 6, 5, 4, 3, 2, 1,
            ],
        ),
        (
            false,
            vec![
                0xfe, 0x80, 0xff, 0xfe, 0x12, 0x34, 0xff, 0xff, 0xff, 0xfe, 0x12, 0x34, 0x56, 0x78, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xfe, 1, 2, 3, 4, 5, 6, 7, 8,
            ],
        ),
    ] {
        let mut serial = SerialRecord::new(Bytes::default(), native_order);
        assert!(serial.set(&-2i8));
        assert!(serial.set(&0x80u8));
        assert!(serial.set(&-2i16));
        assert!(serial.set(&0x1234u16));
        assert!(serial.set(&-2i32));
        assert!(serial.set(&0x12345678u32));
        assert!(serial.set(&-2i64));
        assert!(serial.set(&0x0102030405060708u64));
        assert_eq!(serial.raw().bytes, expected);
        let values = [0x1234u16, 0x9876];
        let mut array = SerialRecord::new(Bytes::default(), native_order);
        assert!(array.set_array(&values));
        assert_eq!(values, [0x1234, 0x9876]);
        assert_eq!(
            array.raw().bytes,
            if native_order {
                vec![0x34, 0x12, 0x76, 0x98]
            } else {
                vec![0x12, 0x34, 0x98, 0x76]
            }
        );
        assert_eq!(array.raw().writes, [4]);
    }
}

#[test]
fn float_bytes_are_never_swapped_and_failed_get_still_swaps() {
    let mut serial = SerialRecord::new(Bytes::default(), false);
    assert!(serial.set_array(&[1.0f32, -2.0]));
    assert!(serial.set(&1.0f64));
    assert_eq!(
        serial.raw().bytes,
        [0, 0, 0x80, 0x3f, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0xf0, 0x3f]
    );
    serial.raw_mut().fail_read = true;
    let mut values = [0x1234u16, 0xabcd];
    assert!(!serial.get_array(&mut values));
    assert_eq!(values, [0x3412, 0xcdab]);
    assert!(serial.set_array::<u32>(&[]));
    assert_eq!(serial.raw().writes.last(), Some(&0));
}

#[test]
fn string_exact_lengths_empty_short_header_and_failure_channels() {
    let mut serial = SerialRecord::new(Bytes::default(), false);
    assert!(serial.set_string(b"cat"));
    assert!(serial.set_string(b""));
    assert_eq!(serial.raw().bytes, [0, 0, 0, 3, b'c', b'a', b't', 0, 0, 0, 0]);
    let mut serial = SerialRecord::new(Bytes::input(b"xy"), true);
    let mut out = b"old".to_vec();
    assert!(serial.get_string(&mut out));
    assert_eq!(out, b"xy");
    let mut serial = SerialRecord::new(Bytes::default(), true);
    serial.raw_mut().fail_write = true;
    assert!(!serial.set_string(b"cat"));
    assert_eq!(serial.raw().writes, [4]);
    let mut serial = SerialRecord::new(Bytes::input(&[3, 0, 0, 0, b'a']), true);
    assert!(!serial.get_string(&mut out)); // no assertion about unspecified out state
    let mut serial = SerialRecord::new(Bytes::input(&[2, 0, 0, 0x10, 0x41, 0, 0xa9, 3]), true);
    assert!(serial.get_string(&mut out));
    assert_eq!(out, [b'A', 0xce, 0xa9]);
}

// v1 and v2 fixtures use literal little-endian headers and two 16-bit pixels.
const V1: &[u8] = &[
    1, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0x34, 0x12, 0xcd, 0xab,
];
const V2: &[u8] = &[
    2, 0, 0, 0, 15, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 15, 0, 0, 0, 0x10, 0xfb, 0, 0, 4, 0xe0, 0x34, 0x12,
    0xcd, 0xab, 0xfc,
];

use std::sync::{Arc, Mutex};
struct NoCodec;
impl ImageCodec for NoCodec {
    fn decompress(&self, _: &[u8]) -> Option<Vec<u8>> {
        None
    }
    fn compress(&self, _: &[u8]) -> Option<Vec<u8>> {
        None
    }
}
// Explicit synthetic conversion, with no claim about native RGB semantics.
struct Synthetic;
impl PixelConversion for Synthetic {
    fn pixel32(&self, value: u32, _: Format, out: &mut [u8]) -> bool {
        out.copy_from_slice(&value.to_le_bytes()[..out.len()]);
        true
    }
    fn rgb(&self, rgb: [u8; 3], _: Format, out: &mut [u8]) -> bool {
        let bytes = [rgb[0], rgb[1], rgb[2], 0xee];
        out.copy_from_slice(&bytes[..out.len()]);
        true
    }
    fn rgb16(&self, value: u16, _: Format) -> Option<[u8; 3]> {
        Some([value as u8, (value >> 8) as u8, 0x7f])
    }
    fn convert(&self, _: &[u8], _: Format, _: Format) -> Option<Vec<u8>> {
        None
    }
    fn index8(&self, index: u8, _: [u8; 3], out: &mut [u8]) -> bool {
        out[0] = index;
        true
    }
}
fn owned() -> Arc<Mutex<OwnedBuffer<Synthetic>>> {
    Arc::new(Mutex::new(OwnedBuffer::new(Synthetic, 1024)))
}

#[test]
fn record_close_failure_is_once_and_destruction_only_releases() {
    struct Owner(Mutex<Vec<&'static str>>);
    impl Segment for Owner {
        fn add_ref(&self) {
            self.0.lock().unwrap().push("add");
        }
        fn release(&self) {
            self.0.lock().unwrap().push("release");
        }
        fn close_record(&self, identity: u64) -> bool {
            assert_eq!(identity, 17);
            self.0.lock().unwrap().push("close");
            false
        }
    }
    let segment = Arc::new(Owner(Mutex::new(Vec::new())));
    let mut record = DbRecord::new(Key::new(1, 2, 3), 1, 17, Some(segment.clone()));
    assert_eq!(record.access_flags(), 0);
    record.set_access_flags(0xff03);
    record.set_key(Key::new(4, 5, 6));
    assert_eq!((record.key(), record.access_flags()), (Key::new(4, 5, 6), 0xff03));
    assert!(!record.close());
    assert!(!record.close());
    assert_eq!(*segment.0.lock().unwrap(), ["add", "close", "release"]);
    assert_eq!(record.release(), 0);
    assert_eq!(record.reference_count(), 1); // record destroy branch never decrements to zero
    drop(record);
    let record = DbRecord::new(Key::default(), 2, 17, Some(segment.clone()));
    drop(record);
    assert_eq!(
        *segment.0.lock().unwrap(),
        ["add", "close", "release", "add", "release"]
    );
    let resource = Resource::new(9);
    assert_eq!(resource.reference_count(), 0);
    assert!(resource.query_interface(RESOURCE_IID));
    assert_eq!(resource.release(), 0);
    assert_eq!(resource.reference_count(), 0); // interlocked decrement, unlike record
    let factory = ResourceFactory::new(0);
    assert_eq!(factory.release(), 0); // entry < 2, unlike resource decrement
}

#[test]
fn views_share_position_and_resource_buffer_replacement_balances_references() {
    let mut serial = SerialRecord::new(Bytes::default(), true);
    assert_eq!(serial.query_interface(INPUT_IID), Some(RecordInterface::Input));
    assert_eq!(serial.query_interface(OUTPUT_IID), Some(RecordInterface::Output));
    assert_eq!(serial.record.reference_count(), 3);
    assert!(serial.adapter().set_rz_char_str(b"hi\0ignored"));
    assert_eq!(serial.raw().bytes, [2, 0, 0, 0, b'h', b'i']);
    assert!(serial.raw_mut().seek(4));
    let mut out = [0; 2];
    assert!(serial.adapter().get_void(&mut out));
    assert_eq!(out, *b"hi");
    let a = owned();
    let b = owned();
    let mut resource = BufferResource::new(Some(a.clone()));
    resource.set_buffer(Some(a.clone())); // AddRef before old Release, same pointer
    assert_eq!(a.lock().unwrap().references, 1);
    resource.set_buffer(Some(b.clone()));
    assert_eq!(a.lock().unwrap().references, 0);
    assert_eq!(b.lock().unwrap().references, 2);
    let view = resource.query_interface(UNKNOWN_IID).unwrap();
    assert_eq!(view.0, ResourceInterface::Buffer); // buffer-first, not resource unknown
    assert_eq!(resource.base.reference_count(), 0);
    view.1.unwrap().lock().unwrap().release();
    drop(resource);
    assert_eq!(b.lock().unwrap().references, 1);
}

#[test]
fn literal_v0_v1_raw_prefix_and_stored_format_conversion_failure() {
    let buffer = owned();
    let mut resource = BufferResource::new(Some(buffer.clone()));
    let mut record = SerialRecord::new(Bytes::input(V1), true);
    assert!(resource.load(&mut record, Format::new(3, 16), &NoCodec));
    assert_eq!(buffer.lock().unwrap().bytes(), [0x34, 0x12, 0xcd, 0xab]);
    assert!(!buffer.lock().unwrap().locked);
    // Raw first 8 bytes are LE even for flag 0, typed words after it are BE.
    let be = [
        1, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0x34, 0x12, 0xcd, 0xab,
    ];
    assert!(resource.load(
        &mut SerialRecord::new(Bytes::input(&be), false),
        Format::new(3, 16),
        &NoCodec
    ));
    let v0 = [
        0x24, 0xda, 0xb9, 0x62, 2, 0, 0, 0, 0, 0, 0, 1, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
    ];
    assert!(resource.load(
        &mut SerialRecord::new(Bytes::input(&v0), false),
        Format::new(3, 16),
        &NoCodec
    ));
    assert_eq!(buffer.lock().unwrap().bytes(), [0x44, 0x33, 0x88, 0x77]);
    assert!(!resource.load(
        &mut SerialRecord::new(Bytes::input(V1), true),
        Format::new(4, 16),
        &NoCodec
    ));
    assert_eq!(buffer.lock().unwrap().layout().format, Format::new(3, 16)); // stored initialization survives failed conversion
    assert!(!buffer.lock().unwrap().locked);
    // Failed v2 codec still unlocks and skips target conversion.
    assert!(!resource.load(
        &mut SerialRecord::new(Bytes::input(V2), true),
        Format::new(3, 16),
        &NoCodec
    ));
    assert!(!buffer.lock().unwrap().locked);
}

#[test]
fn bmp_headers_palette_padding_row_choice_and_save_ignored_write_failures() {
    // Literal 1x2 24-bit BMP, bottom red then top green. Offset deliberately
    // wrong: §6.4 ignores it. All headers and pixels are independent literals.
    let bmp = [
        0x42, 0x4d, 62, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 24, 0, 0, 0, 0, 0,
        8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0,
    ];
    let buffer = owned();
    let mut resource = BufferResource::new(Some(buffer.clone()));
    assert!(resource.load(
        &mut SerialRecord::new(Bytes::input(&bmp), false),
        Format::new(3, 16),
        &NoCodec
    ));
    assert_eq!(buffer.lock().unwrap().bytes(), [0, 255, 255, 0]); // PR11 chosen row direction
    let mut record = SerialRecord::new(Bytes::default(), false);
    assert!(resource.save(&mut record, 0x55d4fb53, &NoCodec));
    assert_eq!(
        &record.raw().bytes[..14],
        &[b'B', b'M', 62, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0]
    );
    assert_eq!(&record.raw().bytes[54..], &[0x7f, 0, 255, 0, 0x7f, 255, 0, 0]);
    record.raw_mut().fail_write = true;
    assert!(resource.save(&mut record, 0x55d4fb53, &NoCodec));
    assert_eq!(&record.raw().writes[3..], &[14, 40, 8]); // all three attempted despite false
    assert!(!resource.save(&mut record, 1, &NoCodec));
    assert!(!buffer.lock().unwrap().locked);
    // 4-bit high nibble first, 2-entry palette, width 2, one padded row.
    let mut palette = bmp[..54].to_vec();
    palette[18..22].copy_from_slice(&[2, 0, 0, 0]);
    palette[22..26].copy_from_slice(&[1, 0, 0, 0]);
    palette[28..30].copy_from_slice(&[4, 0]);
    palette[46..50].copy_from_slice(&[2, 0, 0, 0]);
    palette.extend_from_slice(&[3, 2, 1, 0, 6, 5, 4, 0, 0x10, 0, 0, 0]);
    assert!(resource.load(
        &mut SerialRecord::new(Bytes::input(&palette), true),
        Format::new(3, 16),
        &NoCodec
    ));
    assert_eq!(buffer.lock().unwrap().bytes(), [4, 5, 1, 2]);
    // Same palette indices as 8-bit data, target 8 is a conversion primitive.
    palette[28..30].copy_from_slice(&[8, 0]);
    let n = palette.len();
    palette[n - 4..].copy_from_slice(&[1, 0, 0, 0]);
    assert!(resource.load(
        &mut SerialRecord::new(Bytes::input(&palette), true),
        Format::new(2, 8),
        &NoCodec
    ));
    assert_eq!(buffer.lock().unwrap().bytes(), [1, 0]);
    assert!(!resource.save(&mut record, 0x55d4fb53, &NoCodec)); // non-16 source, no writes
}

#[test]
fn factory_defaults_type_failures_temporary_cleanup_and_settings() {
    let buffer = owned();
    let mut factory = BufferResourceFactory::new(NoCodec, || Some(buffer.clone()), 1);
    assert_eq!(factory.target(), Format::new(3, 16));
    assert_eq!(factory.write_version(), 2);
    assert!(!factory.pop_write_version());
    factory.push_write_version(7);
    assert_eq!(factory.write_version(), 7);
    assert!(factory.pop_write_version());
    assert!(factory.allocate(0, RESOURCE_IID).is_none());
    assert!(factory.allocate(BUFFER_TYPE, 0xdead).is_none());
    assert_eq!(buffer.lock().unwrap().references, 0); // creator temporary released after failed QI
                                                      // Supply fresh allocation references for later load/save calls.
    let mut factory = BufferResourceFactory::new(NoCodec, || Some(owned()), 1);
    let output = factory.load(&mut SerialRecord::new(Bytes::input(V1), true)).unwrap();
    assert_eq!(output.lock().unwrap().bytes(), [0x34, 0x12, 0xcd, 0xab]);
    output.lock().unwrap().release();
    assert!(!factory.save(None, &mut SerialRecord::new(Bytes::default(), true)));
}

#[test]
fn factory_record_type_order_fail_read_qi_and_forwarded_arguments() {
    struct Hooks {
        events: Vec<(u32, u32, u32, u32)>,
        resource: ResourceHandle,
        fail_allocate: bool,
        read: bool,
    }
    impl super::factory::sealed::RuntimeHooks for Hooks {}
    impl FactoryHooks for Hooks {
        fn allocate(&mut self, ty: u32, iid: u32) -> Option<FactoryOutput> {
            self.events.push((ty, iid, 0, 0));
            if self.fail_allocate {
                None
            } else {
                self.resource.lock().unwrap().base.add_ref();
                Some(FactoryOutput::Resource(self.resource.clone()))
            }
        }
        fn create_type(&mut self, ty: u32, iid: u32, a: u32, b: u32) -> Option<FactoryOutput> {
            self.events.push((ty, iid, a, b));
            self.allocate(ty, iid)
        }
        fn serial_read<S: RawStream>(&mut self, resource: &ResourceHandle, _: &mut SerialRecord<S>) -> bool {
            assert_eq!(resource.lock().unwrap().base.key(), Key::new(BUFFER_TYPE, 2, 3));
            self.events.push((5, 0, 0, 0));
            self.read
        }
        fn serial_write<S: RawStream>(&mut self, _: &ResourceHandle, _: &mut SerialRecord<S>) -> bool {
            false
        }
    }
    let base = ResourceFactory::new(1);
    let resource = Arc::new(Mutex::new(BufferResource::new(None)));
    let mut hooks = Hooks {
        events: vec![],
        resource: resource.clone(),
        fail_allocate: true,
        read: true,
    };
    let mut record = SerialRecord::new(Bytes::default(), true);
    record.record.set_key(Key::new(BUFFER_TYPE, 2, 3));
    let mut out = None;
    assert!(!base.create(&mut hooks, &mut record, RESOURCE_IID, &mut out));
    assert_eq!(hooks.events, [(BUFFER_TYPE, RESOURCE_IID, 0, 0)]);
    hooks.fail_allocate = false;
    assert!(base.create_record(&mut hooks, &mut record, RESOURCE_IID, &mut out, 11, 22));
    assert_eq!(
        &hooks.events[1..],
        [
            (BUFFER_TYPE, RESOURCE_IID, 11, 22),
            (BUFFER_TYPE, RESOURCE_IID, 0, 0),
            (5, 0, 0, 0)
        ]
    );
    assert_eq!(record.record.reference_count(), 1);
    assert_eq!(resource.lock().unwrap().base.reference_count(), 1);
    out.take().unwrap().release();
    assert_eq!(resource.lock().unwrap().base.reference_count(), 0);
    hooks.read = false;
    assert!(!base.create(&mut hooks, &mut record, RESOURCE_IID, &mut out));
    assert_eq!(resource.lock().unwrap().base.reference_count(), 0);
    hooks.read = true;
    assert!(!base.create(&mut hooks, &mut record, 0xdead, &mut out));
    assert!(out.is_none());
    assert_eq!(resource.lock().unwrap().base.reference_count(), 0);
}

#[test]
fn variant_no_marker_unsupported_tags_and_temporary_publication() {
    use crate::stream::RefValue;
    use crate::variant::{mem, Elem, Variant};
    let mut record = SerialRecord::new(Bytes::default(), false);
    assert!(record.set_variant(&Variant::Scalar(Elem::I16(-2))));
    assert!(record.set_variant(&Variant::array(
        mem::UINT32,
        vec![Elem::U32(0x12345678), Elem::U32(0x9abcdef0)]
    )));
    assert_eq!(
        record.raw().bytes,
        [0, 0, 0, 6, 0xff, 0xfe, 0, 0, 0x80, 3, 0, 0, 0, 2, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]
    );
    let mut unsupported = SerialRecord::new(Bytes::default(), true);
    assert!(!unsupported.set_variant(&Variant::Object(RefValue::Null)));
    assert_eq!(unsupported.raw().bytes, [13, 0, 0, 0]);
    assert!(!unsupported.set_variant(&Variant::Scalar(Elem::VoidPtr(17))));
    assert_eq!(&unsupported.raw().bytes[4..], &[18, 0, 0, 0]);
    let mut out = Some(Variant::Scalar(Elem::U8(99)));
    let mut bad = SerialRecord::new(Bytes::input(&[0x11, 0, 0, 0, 1]), true);
    assert!(!bad.get_variant(&mut out)); // scalar char written but not accepted
    assert_eq!(out, Some(Variant::Scalar(Elem::U8(99))));
    let mut good = SerialRecord::new(
        Bytes::input(&[0, 0, 0x80, 2, 0, 0, 0, 2, 0x12, 0x34, 0xab, 0xcd]),
        false,
    );
    assert!(good.get_variant(&mut out));
    assert_eq!(
        out,
        Some(Variant::array(mem::UINT16, vec![Elem::U16(0x1234), Elem::U16(0xabcd)]))
    );
    let mut short = SerialRecord::new(Bytes::input(&[3, 0, 0, 0, 1]), true);
    assert!(!short.get_variant(&mut out));
    assert_eq!(
        out,
        Some(Variant::array(mem::UINT16, vec![Elem::U16(0x1234), Elem::U16(0xabcd)]))
    );
    let mut list = KeyList::new(); // authoritative owner, not a cloned list implementation
    list.insert(Key::new(1, 2, 3));
    list.insert(Key::new(1, 2, 3));
    assert_eq!(list.size(), 2);
}

#[test]
fn colour_table_and_explicit_owned_storage_limits() {
    assert_eq!(
        (0..=10).map(|t| Format::stored(t).bits).collect::<Vec<_>>(),
        [0, 0, 8, 16, 16, 16, 16, 16, 32, 32, 0]
    );
    let mut buffer = OwnedBuffer::new(Synthetic, 16);
    assert!(!buffer.initialize(0, 1, Format::new(3, 16)));
    assert!(!buffer.initialize(1, 0, Format::new(3, 16)));
    assert!(!buffer.initialize(1, 1, Format::new(3, 32)));
    assert!(!buffer.initialize(u32::MAX, u32::MAX, Format::new(9, 32)));
    assert!(buffer.initialize(4, 2, Format::new(3, 16)));
    assert!(!buffer.initialize(5, 2, Format::new(3, 16)));
    assert_eq!(buffer.layout().stride, 8);
}

#[test]
fn independent_get_widths_floats_and_defined_partial_host_bytes() {
    let bytes = [
        0xfe, 0x80, 0xff, 0xfe, 0x12, 0x34, 0xff, 0xff, 0xff, 0xfe, 0x12, 0x34, 0x56, 0x78, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xfe, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0xf0, 0x3f,
    ];
    let mut record = SerialRecord::new(Bytes::input(&bytes), false);
    let (mut i8v, mut u8v, mut i16v, mut u16v, mut i32v, mut u32v, mut i64v, mut u64v, mut f32v, mut f64v) =
        (0i8, 0u8, 0i16, 0u16, 0i32, 0u32, 0i64, 0u64, 0f32, 0f64);
    assert!(
        record.get(&mut i8v)
            && record.get(&mut u8v)
            && record.get(&mut i16v)
            && record.get(&mut u16v)
            && record.get(&mut i32v)
            && record.get(&mut u32v)
            && record.get(&mut i64v)
            && record.get(&mut u64v)
            && record.get(&mut f32v)
            && record.get(&mut f64v)
    );
    assert_eq!(
        (i8v, u8v, i16v, u16v, i32v, u32v, i64v, u64v),
        (-2, 0x80, -2, 0x1234, -2, 0x12345678, -2, 0x0102030405060708)
    );
    assert_eq!((f32v, f64v), (1.0, 1.0));
    // std Cursor read_exact modifies the available prefix on a short read.
    let mut record = SerialRecord::new(std::io::Cursor::new(vec![0xab]), false);
    let mut value = 0x1234u16;
    assert!(!record.get(&mut value));
    assert_eq!(value, 0xab12); // failed get still swaps the host-modified bits
}
