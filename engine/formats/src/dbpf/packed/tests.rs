// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::dbpf::{Compression, MemSource, PackageBuilder};
use std::cell::Cell;

const A: Tgi = Tgi::new(0x12345678, 0, 1);
const B: Tgi = Tgi::new(0x12345678, 0, 2);
const Z: Tgi = Tgi::new(0x12345678, 0, 3);
const M: Tgi = Tgi::new(0x12345678, 0, 99);
const X: Tgi = Tgi::new(0xE86B1EEF, 0, 99);
const QFS: &[u8] = &[0x11, 0, 0, 0, 0x10, 0xfb, 0, 0, 12, 0xe0, 65, 66, 67, 68, 0x14, 3, 0xfc];

fn fixture() -> Vec<u8> {
    let mut b = PackageBuilder::new();
    b.add_stored(A, b"ABCD".to_vec(), None);
    b.add_stored(B, QFS.to_vec(), Some(12));
    b.add_stored(Z, vec![], None);
    b.build().unwrap()
}

#[test]
fn dfr1_through_dfr12() {
    let p = Package::from_bytes(fixture()).unwrap();
    let v = PackedView {
        package: &p,
        read_access: true,
        write_access: false,
    };
    // DFR1/2/3/4/11/12: complete, oversized, compressed, empty, prefix, zero capacity.
    for (key, capacity, expected) in [
        (A, 4, &b"ABCD"[..]),
        (A, 6, b"ABCD"),
        (B, 17, QFS),
        (Z, 4, b""),
        (A, 2, b"AB"),
        (A, 0, b""),
    ] {
        let mut buffer = vec![0xcc; capacity as usize + 1];
        let mut size = capacity;
        assert_eq!(
            v.read_record_raw(key, Some(&mut buffer), &mut size, false),
            expected.len() as u32
        );
        assert_eq!(size, expected.len() as u32);
        assert_eq!(&buffer[..expected.len()], expected);
        assert!(buffer[expected.len()..].iter().all(|&b| b == 0xcc));
    }
    // DFR5/6/7: null query returns logical size but clears the size reference.
    for (key, expected) in [(A, 4), (B, 12), (Z, 0)] {
        let mut size = 99;
        assert_eq!(v.read_record_raw(key, None, &mut size, false), expected);
        assert_eq!(size, 0);
    }
    // DFR8, including the null-buffer variant.
    let mut size = 6;
    let mut buffer = [0xcc; 6];
    assert_eq!(v.read_record_raw(M, Some(&mut buffer), &mut size, false), u32::MAX);
    assert_eq!(v.read_record_raw(M, None, &mut size, false), u32::MAX);
    assert_eq!(size, 6);
    // DFR9: read rejection precedes record-open rejection.
    let denied = PackedView {
        package: &p,
        read_access: false,
        write_access: true,
    };
    assert_eq!(denied.read_record_raw(A, Some(&mut buffer), &mut size, true), u32::MAX);
    assert_eq!(size, 6);
    // DFR10: matching write-open record clears size, touching no bytes.
    assert_eq!(v.read_record_raw(A, Some(&mut buffer), &mut size, true), u32::MAX);
    assert_eq!(size, 0);
    assert_eq!(buffer, [0xcc; 6]);
}

struct FaultSource {
    bytes: Vec<u8>,
    fault: Cell<u8>,
    reads: Cell<u32>,
}
impl Source for FaultSource {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> std::io::Result<()> {
        // DFR13 represents a seek failure (no read); DFR14 a failed read
        // which may already have modified the destination.
        if self.fault.get() == 1 {
            return Err(std::io::ErrorKind::Other.into());
        }
        self.reads.set(self.reads.get() + 1);
        if self.fault.get() == 2 {
            out.fill(0x55);
            return Err(std::io::ErrorKind::Other.into());
        }
        MemSource(&self.bytes).read_at(offset, out)
    }
}

#[test]
fn dfr13_dfr14_io_failures() {
    let p = Package::from_source(FaultSource {
        bytes: fixture(),
        fault: Cell::new(0),
        reads: Cell::new(0),
    })
    .unwrap();
    let v = PackedView {
        package: &p,
        read_access: true,
        write_access: false,
    };
    for fault in [1, 2] {
        p.src.fault.set(fault);
        let reads = p.src.reads.get();
        let mut size = 4;
        let mut buffer = [0xcc; 4];
        assert_eq!(v.read_record_raw(A, Some(&mut buffer), &mut size, false), 0);
        assert_eq!(size, 0);
        assert_eq!(p.src.reads.get() - reads, u32::from(fault == 2));
        if fault == 1 {
            assert_eq!(buffer, [0xcc; 4]);
        }
    }
}

fn package(keys: &[Tgi], compressed: bool) -> Package<MemSource<Vec<u8>>> {
    let mut b = PackageBuilder::new();
    for &key in keys {
        if key == B && compressed {
            b.add_stored(key, QFS.to_vec(), Some(12));
        } else {
            b.add(key, if key == DIR_TGI { &b""[..] } else { b"ABCD" }, Compression::None)
                .unwrap();
        }
    }
    Package::from_bytes(b.build().unwrap()).unwrap()
}

#[test]
fn dfk1_through_dfk6() {
    for (keys, compressed, count, expected) in [
        (vec![], false, 0, vec![]),
        (vec![A], false, 1, vec![A]),
        (vec![A, B], true, 3, vec![A, B]),
        (vec![DIR_TGI], false, 1, vec![]),
        (vec![A, X], false, 2, vec![A]),
        (vec![A, A], false, 1, vec![A]),
    ] {
        let p = package(&keys, compressed);
        let v = PackedView {
            package: &p,
            read_access: true,
            write_access: false,
        };
        let mut list = vec![];
        let n = v.resource_keys(
            Some(&mut |k| {
                list.push(k);
                true
            }),
            None,
        );
        list.sort();
        assert_eq!(list, expected);
        assert_eq!(n, expected.len() as u32);
        assert_eq!(n != 0, !expected.is_empty());
        assert_eq!(v.resource_keys(None, None), n);
        assert_eq!(v.record_count(None), count);
        assert_eq!(v.record_count(Some(&mut |_| true)), n);
        // EX55 closed-access quirk; write-only enumeration is still eligible.
        let closed = PackedView {
            package: &p,
            read_access: false,
            write_access: false,
        };
        assert_eq!(closed.resource_keys(None, None), 0);
        assert_eq!(closed.record_count(None), count);
        assert_eq!(closed.record_count(Some(&mut |_| true)), 0);
        let write = PackedView {
            package: &p,
            read_access: false,
            write_access: true,
        };
        assert_eq!(write.resource_keys(None, None), n);
    }
}

#[test]
fn dfk7_filters_dfk8_append_dfk9_failed_insert() {
    let p = package(&[A, B], true);
    let v = PackedView {
        package: &p,
        read_access: true,
        write_access: false,
    };
    let mut list = vec![];
    assert_eq!(
        v.resource_keys(
            Some(&mut |k| {
                list.push(k);
                true
            }),
            Some(&mut |k| k == B)
        ),
        1
    );
    assert_eq!(list, [B]);
    assert_eq!(v.record_count(Some(&mut |k| k == B)), 1);
    let mut queried = vec![];
    assert_eq!(
        v.resource_keys(
            None,
            Some(&mut |k| {
                queried.push(k);
                k.type_id == X.type_id
            })
        ),
        0
    );
    assert!(!queried.contains(&DIR_TGI));
    assert_eq!(v.record_count(Some(&mut |k| k.type_id == X.type_id)), 0);
    assert_eq!(
        v.resource_keys(
            Some(&mut |k| {
                list.push(k);
                true
            }),
            Some(&mut |_| false)
        ),
        0
    );
    assert_eq!(list, [B]);
    let plain = package(&[A], false);
    let v = PackedView {
        package: &plain,
        read_access: true,
        write_access: false,
    };
    let mut list = vec![A];
    for expected in [2, 3] {
        assert_eq!(
            v.resource_keys(
                Some(&mut |k| {
                    list.push(k);
                    true
                }),
                None
            ),
            1
        );
        assert_eq!(list, vec![A; expected]);
    }
    assert_eq!(v.resource_keys(Some(&mut |_| false), None), 1);
    let empty = package(&[], false);
    let v = PackedView {
        package: &empty,
        read_access: true,
        write_access: false,
    };
    assert_eq!(
        v.resource_keys(
            Some(&mut |k| {
                list.push(k);
                true
            }),
            None
        ),
        0
    );
    assert_eq!(list, [A, A, A]);
}
