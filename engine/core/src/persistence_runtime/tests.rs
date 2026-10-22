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
        Self { bytes: bytes.to_vec(), ..Self::default() }
    }
}
impl RawStream for Bytes {
    fn read(&mut self, out: &mut [u8]) -> bool {
        self.reads.push(out.len());
        // This fixture specifies its own atomic short-read behaviour. The
        // runtime may not generalise it to arbitrary native streams.
        if self.fail_read || self.position + out.len() > self.bytes.len() { return false; }
        out.copy_from_slice(&self.bytes[self.position..self.position + out.len()]);
        self.position += out.len();
        true
    }
    fn write(&mut self, data: &[u8]) -> bool {
        self.writes.push(data.len());
        if self.fail_write { return false; }
        self.bytes.resize(self.bytes.len().max(self.position + data.len()), 0);
        self.bytes[self.position..self.position + data.len()].copy_from_slice(data);
        self.position += data.len();
        true
    }
    fn position(&self) -> u64 { self.position as u64 }
    fn size(&self) -> u64 { self.bytes.len() as u64 }
    fn seek(&mut self, position: u64) -> bool { self.position = position as usize; true }
    fn close(&mut self) -> bool { self.closes += 1; false }
}

#[test]
fn independent_integer_widths_signed_bits_and_array_order() {
    for (native_order, expected) in [
        (true, vec![0xfe, 0x80, 0xfe, 0xff, 0x34, 0x12, 0xfe, 0xff, 0xff, 0xff,
            0x78, 0x56, 0x34, 0x12, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            8, 7, 6, 5, 4, 3, 2, 1]),
        (false, vec![0xfe, 0x80, 0xff, 0xfe, 0x12, 0x34, 0xff, 0xff, 0xff, 0xfe,
            0x12, 0x34, 0x56, 0x78, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
            1, 2, 3, 4, 5, 6, 7, 8]),
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
        assert_eq!(array.raw().bytes, if native_order { vec![0x34,0x12,0x76,0x98] } else { vec![0x12,0x34,0x98,0x76] });
        assert_eq!(array.raw().writes, [4]);
    }
}

#[test]
fn float_bytes_are_never_swapped_and_failed_get_still_swaps() {
    let mut serial = SerialRecord::new(Bytes::default(), false);
    assert!(serial.set_array(&[1.0f32, -2.0]));
    assert!(serial.set(&1.0f64));
    assert_eq!(serial.raw().bytes, [0,0,0x80,0x3f, 0,0,0,0xc0, 0,0,0,0,0,0,0xf0,0x3f]);
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
    assert_eq!(serial.raw().bytes, [0,0,0,3,b'c',b'a',b't',0,0,0,0]);
    let mut serial = SerialRecord::new(Bytes::input(b"xy"), true);
    let mut out = b"old".to_vec();
    assert!(serial.get_string(&mut out));
    assert_eq!(out, b"xy");
    let mut serial = SerialRecord::new(Bytes::default(), true);
    serial.raw_mut().fail_write = true;
    assert!(!serial.set_string(b"cat"));
    assert_eq!(serial.raw().writes, [4]);
    let mut serial = SerialRecord::new(Bytes::input(&[3,0,0,0,b'a']), true);
    assert!(!serial.get_string(&mut out)); // no assertion about unspecified out state
    let mut serial = SerialRecord::new(Bytes::input(&[2,0,0,0x10,0x41,0,0xa9,3]), true);
    assert!(serial.get_string(&mut out));
    assert_eq!(out, [b'A',0xce,0xa9]);
}

// v1 and v2 fixtures use literal little-endian headers and two 16-bit pixels.
const V1: &[u8] = &[1,0,0,0,4,0,0,0,2,0,0,0,1,0,0,0,3,0,0,0,0x34,0x12,0xcd,0xab];
const V2: &[u8] = &[2,0,0,0,15,0,0,0,2,0,0,0,1,0,0,0,3,0,0,0,
    15,0,0,0,0x10,0xfb,0,0,4,0xe0,0x34,0x12,0xcd,0xab,0xfc];
