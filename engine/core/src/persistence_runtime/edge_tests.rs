// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

struct Padded {
    layout: BufferLayout,
    bytes: Vec<u8>,
    references: u32,
    locks: usize,
    unlocks: usize,
    fail_lock: bool,
    resource_iid: bool,
}
impl Padded {
    fn new() -> Self {
        Self {
            layout: BufferLayout {
                width: 0,
                height: 0,
                stride: 0,
                format: Format::new(0, 0),
            },
            bytes: vec![],
            references: 1,
            locks: 0,
            unlocks: 0,
            fail_lock: false,
            resource_iid: false,
        }
    }
}
impl Buffer for Padded {
    fn add_ref(&mut self) {
        self.references += 1;
    }
    fn release(&mut self) {
        self.references -= 1;
    }
    fn query_interface(&mut self, iid: u32) -> bool {
        if iid == BUFFER_IID || (self.resource_iid && iid == BUFFER_RESOURCE_IID) {
            self.add_ref();
            true
        } else {
            false
        }
    }
    fn initialize(&mut self, width: u32, height: u32, format: Format) -> bool {
        if format != Format::new(3, 16) {
            return false;
        }
        self.layout = BufferLayout {
            width,
            height,
            stride: width as usize * 2 + 2,
            format,
        };
        self.bytes = vec![0x99; self.layout.stride * height as usize];
        true
    }
    fn layout(&self) -> BufferLayout {
        self.layout
    }
    fn lock(&mut self, _: bool) -> bool {
        if self.fail_lock {
            false
        } else {
            self.locks += 1;
            true
        }
    }
    fn unlock(&mut self) {
        self.unlocks += 1;
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    fn pixel32(&mut self, _: u32, _: u32, _: u32) -> bool {
        panic!("not a v0 fixture")
    }
    fn rgb(&mut self, _: u32, _: u32, _: [u8; 3], _: Option<u8>) -> bool {
        panic!("not a BMP fixture")
    }
    fn rgb16(&self, _: u16) -> Option<[u8; 3]> {
        None
    }
    fn convert(&mut self, target: Format) -> bool {
        target == self.layout.format
    }
}
const QFS: &[u8] = &[
    15, 0, 0, 0, 0x10, 0xfb, 0, 0, 4, 0xe0, 0x34, 0x12, 0xcd, 0xab, 0xfc,
];
struct LiteralCodec;
impl ImageCodec for LiteralCodec {
    fn decompress(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        assert_eq!(bytes, QFS);
        Some(vec![0x34, 0x12, 0xcd, 0xab])
    }
    fn compress(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        assert_eq!(bytes, [0x34, 0x12, 0xcd, 0xab]);
        Some(QFS.to_vec())
    }
}
fn serial(bytes: &[u8]) -> SerialRecord<Cursor<Vec<u8>>> {
    SerialRecord::new(Cursor::new(bytes.to_vec()), true)
}

#[test]
fn padded_v1_choice_v2_correct_rows_and_save_drops_padding() {
    let buffer = Arc::new(Mutex::new(Padded::new()));
    let mut resource = BufferResource::new(Some(buffer.clone()));
    let v1 = [
        1, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0x34, 0x12, 0xcd, 0xab,
    ];
    assert!(resource.load(&mut serial(&v1), Format::new(3, 16), &LiteralCodec));
    assert_eq!(
        buffer.lock().unwrap().bytes,
        [0x34, 0x12, 0x99, 0x99, 0xcd, 0xab, 0x99, 0x99]
    );
    let mut v2 = vec![2, 0, 0, 0, 15, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
    v2.extend_from_slice(QFS);
    assert!(resource.load(&mut serial(&v2), Format::new(3, 16), &LiteralCodec));
    assert_eq!(
        buffer.lock().unwrap().bytes,
        [0x34, 0x12, 0x99, 0x99, 0xcd, 0xab, 0x99, 0x99]
    );
    let mut output = serial(&[]);
    assert!(resource.save(&mut output, 2, &LiteralCodec));
    assert_eq!(output.raw().get_ref(), &v2);
    assert_eq!(buffer.lock().unwrap().locks, 3);
    assert_eq!(buffer.lock().unwrap().unlocks, 3);
    buffer.lock().unwrap().fail_lock = true;
    assert!(!resource.save(&mut output, 2, &LiteralCodec));
    assert_eq!(buffer.lock().unwrap().unlocks, 3);
}

#[test]
fn serial_qi_failure_does_not_call_read_or_release_unacquired_record() {
    struct Nonserial(SerialRecord<Cursor<Vec<u8>>>);
    impl FactoryRecord for Nonserial {
        type Stream = Cursor<Vec<u8>>;
        fn key(&self) -> Key {
            Key::new(BUFFER_TYPE, 2, 3)
        }
        fn query_serial(&mut self) -> bool {
            false
        }
        fn serial(&mut self) -> &mut SerialRecord<Self::Stream> {
            panic!("failed QI")
        }
        fn release_serial(&mut self) {
            panic!("not acquired")
        }
    }
    let buffer = Arc::new(Mutex::new(Padded::new()));
    let mut hooks = BufferResourceFactory::new(LiteralCodec, || Some(buffer.clone()), 1);
    let base = hooks.base.clone();
    let mut record = Nonserial(serial(&[]));
    let mut out = None;
    assert!(!base.create(&mut hooks, &mut record, RESOURCE_IID, &mut out));
    assert!(out.is_none());
    assert_eq!(record.0.record.reference_count(), 1);
    assert_eq!(buffer.lock().unwrap().references, 0);
}

#[test]
fn delegated_resource_iid_releases_the_buffer_reference_only() {
    let buffer = Arc::new(Mutex::new(Padded::new()));
    buffer.lock().unwrap().resource_iid = true;
    let resource = Arc::new(Mutex::new(BufferResource::new(Some(buffer.clone()))));
    resource.lock().unwrap().base.add_ref();
    let mut hooks = BufferResourceFactory::new(LiteralCodec, || None, 1);
    assert!(!hooks.serial_read(&resource, &mut serial(&[]))); // explicitly unsupported derived interface
    assert_eq!(buffer.lock().unwrap().references, 1);
    assert_eq!(resource.lock().unwrap().base.reference_count(), 1);
}

#[test]
fn legacy_string_total_size_and_key_failure_stop_at_first_raw_failure() {
    let mut record = serial(&[3, 0, 0, 2, b'x', b'y']);
    let mut text = vec![];
    assert!(record.get_string(&mut text));
    assert_eq!(text, [3, 0, 0, 2, b'x', b'y']);
    assert_eq!(record.raw().position(), 6);
    let mut record = serial(&[1, 2, 3, 4, 0x88]);
    let mut key = Key::new(0, 0x11223344, 0xaabbccdd);
    assert!(!record.get_key(&mut key));
    assert_eq!(key, Key::new(0x04030201, 0x11223388, 0xaabbccdd));
    assert_eq!(record.raw().position(), 5); // partial read belongs to this known host
}

#[test]
fn file_access_close_failure_and_module_target_selection_are_runtime_owned() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Opened {
        raw: Cursor<Vec<u8>>,
        closes: Arc<AtomicUsize>,
    }
    impl RawStream for Opened {
        fn read(&mut self, out: &mut [u8]) -> bool {
            RawStream::read(&mut self.raw, out)
        }
        fn write(&mut self, bytes: &[u8]) -> bool {
            RawStream::write(&mut self.raw, bytes)
        }
        fn position(&self) -> u64 {
            self.raw.position()
        }
        fn size(&self) -> u64 {
            RawStream::size(&self.raw)
        }
        fn seek(&mut self, position: u64) -> bool {
            RawStream::seek(&mut self.raw, position)
        }
        fn close(&mut self) -> bool {
            self.closes.fetch_add(1, Ordering::SeqCst);
            false
        }
    }
    struct Opener {
        closes: Arc<AtomicUsize>,
        access: Vec<u16>,
        modules: Vec<Option<usize>>,
    }
    impl Opener {
        fn record(&self) -> SerialRecord<Opened> {
            SerialRecord::new(
                Opened {
                    raw: Cursor::new(vec![
                        1, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0x34, 0x12,
                        0xcd, 0xab,
                    ]),
                    closes: self.closes.clone(),
                },
                true,
            )
        }
    }
    impl RecordOpener for Opener {
        type Stream = Opened;
        fn open_file(&mut self, path: &str, access: u16) -> Option<SerialRecord<Opened>> {
            assert_eq!(path, "memory-only");
            self.access.push(access);
            Some(self.record())
        }
        fn open_bitmap(&mut self, module: Option<usize>, id: u32) -> Option<SerialRecord<Opened>> {
            assert_eq!(id, 17);
            self.modules.push(module);
            Some(self.record())
        }
    }
    let closes = Arc::new(AtomicUsize::new(0));
    let mut opener = Opener {
        closes: closes.clone(),
        access: vec![],
        modules: vec![],
    };
    let mut factory = BufferResourceFactory::new(
        LiteralCodec,
        || Some(Arc::new(Mutex::new(Padded::new())) as BufferHandle),
        1,
    );
    factory.set_colour_type(Format::new(9, 32));
    let output = factory
        .load_file(&mut opener, "memory-only", Format::new(3, 16))
        .unwrap();
    assert_eq!(closes.load(Ordering::SeqCst), 1); // false close does not replace successful load result
    assert!(factory.save_file(&mut opener, "memory-only", Some(output.clone())));
    assert_eq!(closes.load(Ordering::SeqCst), 2);
    assert!(!factory.save_file(&mut opener, "memory-only", None));
    assert_eq!(opener.access, [1, 3]); // null buffer did not open a file
    output.lock().unwrap().release();
    let output = factory
        .load_resource_id(&mut opener, None, 17, Format::new(3, 16))
        .unwrap();
    output.lock().unwrap().release();
    assert!(factory
        .load_resource_id(&mut opener, Some(99), 17, Format::new(3, 16))
        .is_none());
    assert_eq!(opener.modules, [None, Some(99)]); // module form uses factory target, not caller target
}
