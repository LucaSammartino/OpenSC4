// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//! Tests for the §2.3 base record, the `xform` codec and the §3 underground
//! behaviour.

use super::identity::NetOccClass;
use super::piece::NetworkPiece;
use super::record::*;
use super::underground::*;
use super::xform::*;
use crate::automata::OccupantBase;
use crate::stream::{ByteReader, ByteWriter};

fn part_len(mask: u8) -> usize {
    let mut n = 1;
    if mask & 4 != 0 {
        n += 36;
    }
    if mask & 1 != 0 {
        n += 12;
    }
    if mask & 2 != 0 {
        n += 4;
    }
    n
}

#[test]
fn xform_round_trips_every_mask_and_has_the_stated_length() {
    for mask in 0u8..8 {
        let x = Xform {
            mask,
            matrix: [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5],
            translation: [10.0, 20.0, 30.0],
            scale: 2.0,
        };
        let mut w = ByteWriter::new();
        assert!(x.write(&mut w));
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), part_len(mask));
        assert_eq!(x.encoded_len(), part_len(mask));
        assert_eq!(bytes[0], mask);
        let y = Xform::read(&mut ByteReader::new(&bytes)).unwrap();
        assert_eq!(y.mask, mask);
        // Absent parts read back as identity (NO15 literal reading).
        let m = if mask & 4 != 0 { x.matrix } else { IDENTITY_MATRIX };
        assert_eq!(y.matrix, m);
        let t = if mask & 1 != 0 { x.translation } else { [0.0; 3] };
        assert_eq!(y.translation, t);
        assert_eq!(y.scale, if mask & 2 != 0 { 2.0 } else { 1.0 });
    }
    // Order is matrix, translation, scale: with no matrix the first float is
    // the translation, the scale comes last.
    let x = Xform {
        mask: 3,
        translation: [7.0, 8.0, 9.0],
        scale: 4.0,
        ..Xform::default()
    };
    let mut w = ByteWriter::new();
    x.write(&mut w);
    let b = w.into_bytes();
    assert_eq!(f32::from_le_bytes([b[1], b[2], b[3], b[4]]), 7.0);
    assert_eq!(f32::from_le_bytes([b[13], b[14], b[15], b[16]]), 4.0);
    // Short stream: fails.
    assert!(Xform::read(&mut ByteReader::new(&b[..16])).is_none());
}

fn sample_network_record() -> NetworkBaseRecord {
    let mut r = NetworkBaseRecord {
        base: OccupantBase::new(0x7475_8925),
        transform: Xform {
            mask: 1,
            translation: [16.0, 2.0, 48.0],
            ..Xform::default()
        },
        sub100: Sub100 { a: 3, b: 1, c: 0x4B00 },
        bbox_words: [1, 2, 3, 4, 5, 6],
        saved_flags: 5,
        word_15c: 9,
        int64: -1_234_567_890_123,
        ..NetworkBaseRecord::default()
    };
    // `Read` turns the highlight into the status `(h << 4) | 2` (lr §2.3).
    r.base.status = 2;
    for (i, q) in r.quads.iter_mut().enumerate() {
        q.words = [i as u32, 10, 20, 30, 40];
        q.bytes = [1, 2, 3, i as u8];
    }
    r.quad_tail = QuadTail {
        word_f4: 0xF4,
        byte_fd: 0xFD,
        word_f8: 0xF8,
        byte_fc: 0xFC,
    };
    r.triangles = vec![
        TriangleRecord {
            word0: 1,
            p: [1.0, 2.0, 3.0],
            q: [4.0, 5.0, 6.0],
            word1: 7,
            word2: 8,
            bytes: [9, 10, 11, 0xFF],
        },
        TriangleRecord {
            word0: 2,
            ..TriangleRecord::default()
        },
    ];
    r
}

fn encode(r: &NetworkBaseRecord) -> Vec<u8> {
    let mut w = ByteWriter::new();
    assert!(write_network_record(r, &mut w));
    w.into_bytes()
}

fn base_len(r: &NetworkBaseRecord) -> usize {
    let mut w = ByteWriter::new();
    r.base.write(&mut w);
    w.into_bytes().len()
}

#[test]
fn network_record_v8_round_trips() {
    let r = sample_network_record();
    let bytes = encode(&r);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 8);
    // Length: version, base, xform, quads 4 × 24, tail 10, sub-record 8,
    // count 4, 2 × 40 triangle bytes, bbox 24, flags 4, u32 4, null ref 4, i64 8.
    let want = 2 + base_len(&r) + r.transform.encoded_len() + 96 + 10 + 8 + 4 + 80 + 24 + 4 + 4 + 4 + 8;
    assert_eq!(bytes.len(), want);
    let back = read_network_record(&mut ByteReader::new(&bytes)).unwrap();
    assert_eq!(back.version, 8);
    assert_eq!(back.base, r.base);
    assert_eq!(back.transform, r.transform);
    assert_eq!(back.quads, r.quads);
    assert_eq!(back.quad_tail, r.quad_tail);
    assert_eq!(back.sub100, r.sub100);
    assert_eq!(back.triangles, r.triangles);
    assert_eq!(back.bbox_words, r.bbox_words);
    assert_eq!(back.saved_flags, 5);
    assert_eq!(back.word_15c, 9);
    assert!(back.owned_ref.is_null());
    assert_eq!(back.int64, r.int64);
    assert_eq!(back.load_fixup(), LoadFixup::ApplyAltitude);
    assert!(!back.flags_need_v5_conversion());
    assert_eq!(encode(&back), bytes);
    assert!(read_network_record(&mut ByteReader::new(&bytes[..bytes.len() - 1])).is_none());
}

#[test]
fn network_record_versions_and_masks() {
    let mut r = sample_network_record();
    r.saved_flags = 0xFFFF_FFFF;
    let mut bytes = encode(&r);
    let back = read_network_record(&mut ByteReader::new(&bytes)).unwrap();
    assert_eq!(back.saved_flags, SAVED_FLAGS_MASK);
    // v7: no reference / int64 (the v8 branch): 12 bytes shorter.
    let short = bytes.len() - 12;
    bytes[0] = 7;
    let back = read_network_record(&mut ByteReader::new(&bytes[..short])).unwrap();
    assert_eq!(back.version, 7);
    assert_eq!(back.int64, 0);
    assert_eq!(back.load_fixup(), LoadFixup::ApplyAltitude);
    // v6 with +0x15C non-zero: recompute the cell; zero: nothing.
    bytes[0] = 6;
    let mut back = read_network_record(&mut ByteReader::new(&bytes[..short])).unwrap();
    assert_eq!(back.load_fixup(), LoadFixup::RecomputeCell);
    back.word_15c = 0;
    assert_eq!(back.load_fixup(), LoadFixup::None);
    // v5: the flags would need the v5 conversion (NO16).
    bytes[0] = 5;
    let back = read_network_record(&mut ByteReader::new(&bytes[..short])).unwrap();
    assert!(back.flags_need_v5_conversion());
    // v3 rejected at once; v4 blocked on the legacy transform (NO15).
    bytes[0] = 3;
    assert!(read_network_record(&mut ByteReader::new(&bytes)).is_none());
    bytes[0] = 4;
    assert!(read_network_record(&mut ByteReader::new(&bytes)).is_none());
}

#[test]
fn network_record_rejects_a_bad_sub100_version() {
    let r = sample_network_record();
    let mut bytes = encode(&r);
    let off = 2 + base_len(&r) + r.transform.encoded_len() + 96 + 10;
    assert_eq!(u16::from_le_bytes([bytes[off], bytes[off + 1]]), SUB100_VERSION);
    bytes[off] = 3;
    assert!(read_network_record(&mut ByteReader::new(&bytes)).is_none());
}

struct UgHost {
    flags: Option<u32>,
    refreshes: u32,
}

impl UndergroundHost for UgHost {
    fn neighbour_flags(&mut self, _dir: u32) -> Option<u32> {
        self.flags
    }
    fn refresh_cell_value(&mut self) {
        self.refreshes += 1;
    }
}

#[test]
fn underground_sort_position_and_heights() {
    let mut u = UndergroundPiece::new(NetworkPiece::new(NetOccClass::Underground));
    u.piece.network_flags = 0;
    u.sort_value = 77;
    assert_eq!(u.get_sort_value(), UNDERGROUND_SORT_DEFAULT);
    u.piece.network_flags = 0x0200_0000;
    assert_eq!(u.get_sort_value(), 77);
    let s = UndergroundPiece::new(NetworkPiece::new(NetOccClass::Subway));
    assert_eq!(s.sort_default, 0xCA16_374F);

    for (i, p) in u.points.iter_mut().enumerate() {
        p.x = i as f32;
        p.z = 10.0 * i as f32;
    }
    u.heights = [5.0, 3.0, 4.0, 6.0, -100.0];
    assert_eq!(u.get_position(), [10.0, 3.0, 100.0]);
    u.store_init_heights(&[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(u.get_terrain_heights(), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(u.heights[0], 5.0);
    u.set_vertex_heights(&[9.0; 4]);
    assert_eq!(u.points[3].z, 9.0);
    assert_eq!(u.points[4].z, 40.0);
}

#[test]
fn underground_model_imaginary_and_watered() {
    let mut u = UndergroundPiece::new(NetworkPiece::new(NetOccClass::Pipe));
    u.piece.network_flags = FLAG_BURST | FLAG_DISTRESSED;
    assert_eq!(u.model_choice(), UndergroundModel::Burst);
    u.piece.network_flags = FLAG_DISTRESSED;
    assert_eq!(u.model_choice(), UndergroundModel::Distressed);
    u.piece.network_flags = 0;
    assert_eq!(u.model_choice(), UndergroundModel::Normal);

    let mut h = UgHost {
        flags: Some(0x8000_0001),
        refreshes: 0,
    };
    assert_eq!(u.is_imaginary_connection(0, &mut h), Some(false));
    h.flags = Some(1);
    assert_eq!(u.is_imaginary_connection(0, &mut h), Some(true));
    h.flags = None;
    assert_eq!(u.is_imaginary_connection(0, &mut h), None);

    u.set_watered_state(1, &mut h);
    u.set_watered_state(1, &mut h);
    u.set_watered_state(0, &mut h);
    assert_eq!(h.refreshes, 2);
    u.pipe_270 = 5;
    u.pipe_init_tail();
    assert_eq!(u.pipe_270, 0);
}
