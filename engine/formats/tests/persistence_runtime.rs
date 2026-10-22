// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal QFS pixels, not a compressor round-trip oracle.
use fc4_core::persistence_runtime::*;
use fc4_formats::qfs::PersistenceQfs;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

struct NoConversion;
impl PixelConversion for NoConversion {
    fn pixel32(&self, _: u32, _: Format, _: &mut [u8]) -> bool {
        false
    }
    fn rgb(&self, _: [u8; 3], _: Format, _: &mut [u8]) -> bool {
        false
    }
    fn rgb16(&self, _: u16, _: Format) -> Option<[u8; 3]> {
        None
    }
    fn convert(&self, _: &[u8], _: Format, _: Format) -> Option<Vec<u8>> {
        None
    }
}
#[test]
fn literal_qfs_load_exact_size_failure_and_save_header() {
    let literal = [
        2, 0, 0, 0, 15, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 15, 0, 0, 0, 0x10, 0xfb, 0, 0, 4, 0xe0, 0x34,
        0x12, 0xcd, 0xab, 0xfc,
    ];
    let buffer = Arc::new(Mutex::new(OwnedBuffer::new(NoConversion, 1024)));
    let mut resource = BufferResource::new(Some(buffer.clone()));
    assert!(resource.load(
        &mut SerialRecord::new(Cursor::new(literal.to_vec()), true),
        Format::new(3, 16),
        &PersistenceQfs
    ));
    assert_eq!(buffer.lock().unwrap().bytes(), [0x34, 0x12, 0xcd, 0xab]);
    assert!(!buffer.lock().unwrap().locked);
    let mut saved = SerialRecord::new(Cursor::new(Vec::new()), true);
    assert!(resource.save(&mut saved, 2, &PersistenceQfs));
    let saved_bytes = saved.raw().get_ref();
    assert_eq!(&saved_bytes[..4], &[2, 0, 0, 0]);
    assert_eq!(&saved_bytes[8..20], &[2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0]);
    assert_eq!(
        u32::from_le_bytes(saved_bytes[4..8].try_into().unwrap()) as usize,
        saved_bytes.len() - 20
    );
    // Compressor choices are free; the independent expected pixels are fixed.
    assert_eq!(
        fc4_formats::qfs::decompress(&saved_bytes[20..]).unwrap(),
        [0x34, 0x12, 0xcd, 0xab]
    );
    let mut wrong_size = literal;
    wrong_size[8] = 3; // decoded 4 bytes, initialized buffer requires 6
    assert!(!resource.load(
        &mut SerialRecord::new(Cursor::new(wrong_size.to_vec()), true),
        Format::new(3, 16),
        &PersistenceQfs
    ));
    assert!(!buffer.lock().unwrap().locked);
    let mut corrupt = literal;
    corrupt[25] = 0; // invalid QFS signature
    assert!(!resource.load(
        &mut SerialRecord::new(Cursor::new(corrupt.to_vec()), true),
        Format::new(3, 16),
        &PersistenceQfs
    ));
    assert!(!buffer.lock().unwrap().locked);
}
