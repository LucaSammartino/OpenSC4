// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//! Tests for the §2.3 base record, the transform stream form and the §3
//! underground records and behaviour (answers NO15-NO17).

use super::identity::NetOccClass;
use super::piece::NetworkPiece;
use super::record::*;
use super::underground::*;
use super::xform::*;
use crate::automata::OccupantBase;
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, ResKey};

fn f(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn xbytes(x: &Xform) -> Vec<u8> {
    let mut w = ByteWriter::new();
    assert!(x.write(&mut w));
    w.into_bytes()
}

#[test]
fn xform_worked_examples_are_exact() {
    // §2.3: mask 1, translation (8, 2.5, 8) -> 13 bytes.
    let x = Xform {
        mask: 1,
        translation: [8.0, 2.5, 8.0],
        ..Xform::default()
    };
    let b = xbytes(&x);
    assert_eq!(b, [0x01, 0, 0, 0, 0x41, 0, 0, 0x20, 0x40, 0, 0, 0, 0x41]);
    // mask 7, identity, translation 0, scale 2.0 -> 53 bytes ending 00 00 00 40.
    let x = Xform {
        mask: 7,
        scale: 2.0,
        ..Xform::default()
    };
    let b = xbytes(&x);
    assert_eq!(b.len(), 53);
    assert_eq!(x.encoded_len(), 53);
    assert_eq!(&b[49..], &[0, 0, 0, 0x40]);
    assert_eq!(f(&b, 1), 1.0);
    let y = Xform::read(&mut ByteReader::new(&b)).unwrap();
    assert_eq!((y.mask, y.scale, y.counter), (7, 2.0, 3));
}

#[test]
fn xform_read_rules() {
    // A written mask 7 with scale 1.0 reads back as mask 5.
    let b = xbytes(&Xform {
        mask: 7,
        ..Xform::default()
    });
    let y = Xform::read(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(y.mask, 5);
    assert_eq!(y.counter, 3);
    // A NaN scale counts as different: bit 2 set.
    let b = xbytes(&Xform {
        mask: 2,
        scale: f32::NAN,
        ..Xform::default()
    });
    assert_eq!(Xform::read(&mut ByteReader::new(&b)).unwrap().mask, 2);
    // Live mask non-zero: reset first (counter +1), omitted parts identity.
    let mut live = Xform {
        counter: 10,
        mask: 4,
        matrix: [2.0; 9],
        translation: [5.0; 3],
        ..Xform::default()
    };
    let b = xbytes(&Xform {
        mask: 1,
        translation: [1.0, 2.0, 3.0],
        ..Xform::default()
    });
    live.read_into(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(live.mask, 1);
    assert_eq!(live.matrix, IDENTITY_MATRIX);
    assert_eq!(live.translation, [1.0, 2.0, 3.0]);
    assert_eq!(live.counter, 12);
    // Live mask 0: no reset, an omitted part keeps its prior value.
    let mut live = Xform {
        mask: 0,
        matrix: [2.0; 9],
        ..Xform::default()
    };
    live.read_into(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(live.matrix, [2.0; 9]);
    assert_eq!((live.mask, live.counter), (1, 1));
    // Short stream fails.
    assert!(Xform::read(&mut ByteReader::new(&b[..12])).is_none());
}

#[test]
fn xform_legacy_load_is_56_bytes() {
    let mut w = ByteWriter::new();
    for i in 0..9 {
        w.put_f32(i as f32);
    }
    for v in [4.0f32, 5.0, 6.0] {
        w.put_f32(v);
    }
    w.put_f32(1.0);
    w.put_u32(0x1234_5678);
    w.put_u8(0xEE);
    let b = w.into_bytes();
    assert_eq!(b.len(), 57);
    let mut live = Xform {
        mask: 2,
        counter: 9,
        ..Xform::default()
    };
    let mut r = ByteReader::new(&b);
    live.legacy_load_into(&mut r).unwrap();
    assert_eq!(r.get_u8(), Some(0xEE));
    // Never reset; scale 1.0 clears bit 2; counter = low byte.
    assert_eq!(live.mask, 5);
    assert_eq!(live.counter, 0x78);
    assert_eq!(live.matrix[8], 8.0);
    assert_eq!(live.translation, [4.0, 5.0, 6.0]);
}

#[test]
fn v5_flag_conversion_examples() {
    assert_eq!(convert_v5_flags(0x0000_0283), 0x0080_0003);
    assert_eq!(convert_v5_flags(0x0000_1001), 0x0800_0001);
    // Bit 0x80 clear drops 0x6000000 (from old 0x800 / 0x1000).
    assert_eq!(convert_v5_flags(0x1800), 0x0800_0000);
    assert_eq!(convert_v5_flags(0x1880), 0x0600_0000);
    // Silent old bits produce nothing.
    assert_eq!(convert_v5_flags(0xD202_0180), 0);
    assert_eq!(convert_v5_flags(0x0400_0080), 0x8000_0000);
}

fn vtx(x: f32, y: f32, z: f32, color: u32) -> Vertex {
    Vertex {
        pos: [x, y, z],
        color,
        u: 0.25,
        v: 0.75,
    }
}

fn sample_network_record() -> NetworkBaseRecord {
    let mut r = NetworkBaseRecord {
        base: OccupantBase::new(0x7475_8925),
        transform: Xform {
            mask: 1,
            translation: [16.0, 2.0, 48.0],
            ..Xform::default()
        },
        word_fc: 0x4B00,
        rotation: 2,
        variation: 7,
        orientation: 0x81,
        cell_info: CellInfo {
            byte_10c: 1,
            word_110: 0x99,
            entries: vec![(3, 0x10), (4, 0x20)],
        },
        bbox_min: [0.0, 1.0, 2.0],
        bbox_max: [16.0, 3.0, 18.0],
        network_flags: 5,
        int64: -1_234_567_890_123,
        ..NetworkBaseRecord::default()
    };
    r.base.status = 2;
    r.quad = [
        vtx(0.0, 1.0, 0.0, 0xFF11_2233),
        vtx(16.0, 1.0, 0.0, 0xFF11_2233),
        vtx(16.0, 1.0, 16.0, 0xFF11_2233),
        vtx(0.0, 1.0, 16.0, 0xFF11_2233),
    ];
    r.triangles = vec![Triangle {
        word0: 1,
        verts: [vtx(1.0, 2.0, 3.0, 0xFFAA_BBCC); 3],
    }];
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

fn prebuilt_worked_case(gate: u32) -> PrebuiltRecord {
    PrebuiltRecord {
        base: sample_network_record(),
        saved_178: [1.0, 2.0, 3.0],
        saved_1b0: 0x1122_3344,
        saved_1b4: 0xA5,
        gate_19c: gate,
        saved_1a0: [4.0, 5.0, 6.0, 7.0],
    }
}

#[test]
fn no46_prebuilt_bridge_envelope_raw_word_gate_and_v3_discard() {
    for gate in [0, 0x8000_0000] {
        let record = prebuilt_worked_case(gate);
        let mut out = ByteWriter::new();
        assert!(record.write(&mut out));
        let bytes = out.into_bytes();
        let suffix = 2 + encode(&record.base).len();
        assert_eq!(&bytes[..4], &[4, 0, 8, 0]);
        assert_eq!(bytes.len() - suffix, if gate == 0 { 21 } else { 37 });
        assert_eq!(
            &bytes[suffix + 12..suffix + 21],
            &[0x44, 0x33, 0x22, 0x11, 0xA5, 0, 0, 0, (gate >> 24) as u8]
        );
        if gate != 0 {
            assert_eq!([f(&bytes, suffix + 21), f(&bytes, suffix + 33)], [4.0, 7.0]);
        }
        let mut loaded = PrebuiltRecord {
            saved_1a0: [99.0; 4],
            ..PrebuiltRecord::default()
        };
        assert!(loaded.read_into(&mut ByteReader::new(&bytes)));
        assert_eq!(loaded.saved_1a0, if gate == 0 { [99.0; 4] } else { record.saved_1a0 });
        let mut again = ByteWriter::new();
        assert!(loaded.write(&mut again));
        assert_eq!(again.bytes(), bytes);

        let mut v3 = bytes.clone();
        v3[..2].copy_from_slice(&3u16.to_le_bytes());
        let discarded: Vec<_> = (10u32..16).flat_map(u32::to_le_bytes).collect();
        v3.splice(suffix + 12..suffix + 12, discarded);
        v3.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        let mut inp = ByteReader::new(&v3);
        assert!(loaded.read_into(&mut inp));
        assert_eq!(inp.remaining(), 0);
        let mut canonical = ByteWriter::new();
        assert!(loaded.write(&mut canonical));
        assert_eq!(canonical.bytes(), bytes);
        assert_eq!(v3.len() - canonical.len(), 28);

        for n in 0..bytes.len() {
            assert!(!loaded.read_into(&mut ByteReader::new(&bytes[..n])), "truncation {n}");
        }
    }
    for version in [0u16, 2, 5] {
        let mut bytes = version.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[8, 0, 0, 0]);
        let mut inp = ByteReader::new(&bytes);
        assert!(!PrebuiltRecord::default().read_into(&mut inp));
        assert_eq!(inp.position(), 2);
    }
}

#[test]
fn no46_tunnel_wraps_prebuilt_and_clears_pair() {
    use crate::stream::RefValue;
    let mut tunnel = TunnelRecord {
        prebuilt: prebuilt_worked_case(0),
        pair: RefValue::Host {
            token: 99,
            class_id: 0x8A4B_D52B,
        },
    };
    let mut out = ByteWriter::new();
    // A failed serialization QI emits no reference even when a pair exists.
    assert!(tunnel.write(&RefValue::Null, &mut out));
    let bytes = out.into_bytes();
    assert_eq!(&bytes[..6], &[2, 0, 4, 0, 8, 0]);
    assert_eq!(bytes.last(), Some(&0));
    assert!(tunnel.read_into(&mut ByteReader::new(&bytes)));
    assert!(tunnel.pair.is_null());
    let mut bad = ByteReader::new(&[3, 0, 4, 0, 8, 0]);
    assert!(!tunnel.read_into(&mut bad));
    assert_eq!(bad.position(), 2);
}

#[test]
fn network_record_v8_layout_and_round_trip() {
    let r = sample_network_record();
    let b = encode(&r);
    // 2 + base + xform + 96 + 10 + (8 + 5·2) + 4 + 76·1 + 24 + 4 + 4 + key 12 + 8.
    let bl = base_len(&r);
    assert_eq!(b.len(), 2 + bl + 13 + 96 + 10 + 18 + 4 + 76 + 24 + 4 + 4 + 12 + 8);
    // Vertex 0: x, y, z, u, v then R, G, B, A of 0xFF112233.
    let q = 2 + bl + 13;
    assert_eq!(f(&b, q + 4), 1.0);
    assert_eq!(f(&b, q + 12), 0.25);
    assert_eq!(&b[q + 20..q + 24], &[0x11, 0x22, 0x33, 0xFF]);
    // The four scalars: u32 +0xFC, u8 +0x105, u32 +0x100, u8 +0x104.
    let s = q + 96;
    assert_eq!(&b[s..s + 10], &[0x00, 0x4B, 0, 0, 2, 7, 0, 0, 0, 0x81]);
    // Cell info: u16 2, count 2, u8, u32, entries.
    assert_eq!(&b[s + 10..s + 18], &[2, 0, 2, 1, 0x99, 0, 0, 0]);
    assert_eq!(&b[s + 18..s + 23], &[3, 0x10, 0, 0, 0]);
    // Bbox interleaved: min x, max x, min y, ...
    let bb = s + 28 + 4 + 76;
    assert_eq!([f(&b, bb), f(&b, bb + 4), f(&b, bb + 8)], [0.0, 16.0, 1.0]);
    let back = read_network_record(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(back.quad, r.quad);
    assert_eq!(back.cell_info, r.cell_info);
    assert_eq!(back.triangles, r.triangles);
    assert_eq!((back.bbox_min, back.bbox_max), (r.bbox_min, r.bbox_max));
    assert_eq!(
        (back.word_fc, back.rotation, back.variation, back.orientation),
        (0x4B00, 2, 7, 0x81)
    );
    assert_eq!(back.int64, r.int64);
    assert_eq!(back.load_effects(), LoadEffects::default());
    assert_eq!(encode(&back), b);
    assert!(read_network_record(&mut ByteReader::new(&b[..b.len() - 1])).is_none());
}

#[test]
fn network_record_read_forces_triangle_alpha_and_masks_flags() {
    let mut r = sample_network_record();
    r.triangles[0].verts[1].color = 0x0012_3456;
    r.network_flags = 0xFFFF_FFFF;
    let back = read_network_record(&mut ByteReader::new(&encode(&r))).unwrap();
    assert_eq!(back.triangles[0].verts[1].color, 0xFF12_3456);
    assert_eq!(back.network_flags, SAVED_FLAGS_MASK);
    // The quad alpha is untouched by the read; the final fix-up forces it.
    r.quad[0].color = 0x0011_2233;
    let mut back = read_network_record(&mut ByteReader::new(&encode(&r))).unwrap();
    assert_eq!(back.quad[0].alpha(), 0);
    assert!(back.finish_read_alpha(false));
    assert!(back.quad.iter().all(|v| v.alpha() == 0xFF));
    assert!(!back.finish_read_alpha(false));
    assert!(back.finish_read_alpha(true));
}

/// Builds an old-version stream by patching the version and inserting the
/// v < 7 offsets after the path id.
fn old_version(r: &NetworkBaseRecord, version: u16, offsets: Option<[f32; 2]>) -> Vec<u8> {
    let mut b = encode(r);
    b[0..2].copy_from_slice(&version.to_le_bytes());
    if let Some([dx, dz]) = offsets {
        let at = b.len() - 20; // before the resource key (12) and the i64 (8)
        let mut ins = dx.to_le_bytes().to_vec();
        ins.extend_from_slice(&dz.to_le_bytes());
        b.splice(at..at, ins);
    }
    b
}

#[test]
fn network_record_old_versions_and_tail() {
    let mut r = sample_network_record();
    r.path_id = 9;
    // v7, path id set: no offsets, the mark property only.
    let back = read_network_record(&mut ByteReader::new(&old_version(&r, 7, None))).unwrap();
    assert_eq!(
        back.load_effects(),
        LoadEffects {
            legacy_cell: None,
            add_legacy_mark: true
        }
    );
    // v6, path id set: two offsets; quad v0.x 0, v2.x 16 -> (8 + 40) / 16 = 3;
    // z: (0 + 16) / 2 + 9 = 17 -> 1.0625 -> 1.
    let b = old_version(&r, 6, Some([40.0, 9.0]));
    let back = read_network_record(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(back.legacy_offsets, Some([40.0, 9.0]));
    let e = back.load_effects();
    assert_eq!(e.legacy_cell, Some((3, 1)));
    assert!(e.add_legacy_mark);
    assert_eq!(legacy_cell_value(3, 1), 0x0001_0003);
    assert_eq!(legacy_cell_value(-1, 0), 0xFFFF_FFFF);
    // Negative cells floor down.
    let b = old_version(&r, 6, Some([-9.0, -8.0]));
    let e = read_network_record(&mut ByteReader::new(&b)).unwrap().load_effects();
    assert_eq!(e.legacy_cell, Some((-1, 0)));
    // v6, path id 0: no offsets read, no effects.
    r.path_id = 0;
    let back = read_network_record(&mut ByteReader::new(&old_version(&r, 6, None))).unwrap();
    assert_eq!(back.load_effects(), LoadEffects::default());
    // v5: the flags are converted (old 0x1001 -> 0x08000001, then masked).
    r.network_flags = 0x1001;
    let back = read_network_record(&mut ByteReader::new(&old_version(&r, 5, None))).unwrap();
    assert_eq!(back.network_flags, 0x0800_0001);
    // v3 rejected.
    assert!(read_network_record(&mut ByteReader::new(&old_version(&r, 3, None))).is_none());
}

#[test]
fn network_path_info_is_a_resource_key_in_every_version() {
    let mut rec = sample_network_record();
    rec.path_id = 9;
    for key in [
        ResKey::default(),
        ResKey {
            type_id: 0x2966_7894,
            group: 0x1122_3344,
            instance: 0x5566_7788,
        },
    ] {
        rec.path_info_key = key;
        for version in 5..=8 {
            let offsets = (version < 7).then_some([0.0, 0.0]);
            let bytes = old_version(&rec, version, offsets);
            let at = bytes.len() - 20;
            assert_eq!(&bytes[at..at + 4], &key.type_id.to_le_bytes());
            assert_eq!(&bytes[at + 4..at + 8], &key.group.to_le_bytes());
            assert_eq!(&bytes[at + 8..at + 12], &key.instance.to_le_bytes());
            assert_eq!(&bytes[at + 12..], &rec.int64.to_le_bytes());
            let mut input = ByteReader::new(&bytes);
            let back = read_network_record(&mut input).unwrap();
            assert_eq!(back.path_info_key, key);
            assert_eq!(back.int64, rec.int64);
            assert_eq!(input.error(), 0);
            assert!(input.get_u8().is_none());
            for end in at..bytes.len() {
                assert!(read_network_record(&mut ByteReader::new(&bytes[..end])).is_none());
            }
        }
    }
}

#[test]
fn native_read_fixups_prevent_an_unconditional_byte_round_trip() {
    // Literal §2.3 Read behavior, not a divergence to repair for a shadow.
    // NO45 asks how a raw-record shadow should account for these mutations.
    let mut rec = sample_network_record();
    rec.network_flags = 0x0600_8000;
    let original = encode(&rec);
    let loaded = read_network_record(&mut ByteReader::new(&original)).unwrap();
    assert_eq!(loaded.network_flags, 0x0600_0000);
    let rewritten = encode(&loaded);
    let flags = original.len() - 28;
    assert_eq!(&original[flags..flags + 4], &[0, 0x80, 0, 6]);
    assert_eq!(&rewritten[flags..flags + 4], &[0, 0, 0, 6]);
    assert_ne!(original, rewritten);
}

#[test]
fn network_record_v4_uses_the_legacy_transform() {
    let r = sample_network_record();
    let b = encode(&r);
    let bl = base_len(&r);
    let xf = 2 + bl;
    let mut w = ByteWriter::new();
    for v in IDENTITY_MATRIX {
        w.put_f32(v);
    }
    for v in [8.0f32, 0.0, 8.0] {
        w.put_f32(v);
    }
    w.put_f32(3.0);
    w.put_u32(0x42);
    let legacy = w.into_bytes();
    let mut v4 = vec![4, 0];
    v4.extend_from_slice(&b[2..xf]);
    v4.extend_from_slice(&legacy);
    v4.extend_from_slice(&b[xf + 13..]);
    // The flags are converted below v6: old 5 -> 0x5 | 0x08000000.
    let back = read_network_record(&mut ByteReader::new(&v4)).unwrap();
    assert_eq!(back.version, 4);
    assert_eq!(back.transform.mask, 7);
    assert_eq!(back.transform.counter, 0x42);
    assert_eq!(back.transform.translation, [8.0, 0.0, 8.0]);
    assert_eq!(back.quad, r.quad);
    assert_eq!(back.network_flags, 0x0800_0005);
}

#[test]
fn network_record_bad_cell_info_version_continues_misaligned() {
    let r = NetworkBaseRecord {
        base: OccupantBase::new(0x7475_8925),
        ..NetworkBaseRecord::default()
    };
    let mut b = encode(&r);
    let off = 2 + base_len(&r) + 1 + 96 + 10;
    assert_eq!(&b[off..off + 2], &[2, 0]);
    b[off] = 3;
    // Only the u16 is consumed and the read goes on 6 bytes early (all
    // zero here: triangle count 0, a zero bbox/flags/path/null ref/i64);
    // the cell info stays as it was.
    let back = read_network_record(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(back.cell_info, CellInfo::default());
}

fn sample_tail() -> UndergroundTail {
    let mut m = [0.0f32; 16];
    for i in 0..4 {
        m[i * 5] = 1.0;
    }
    UndergroundTail {
        bytes_50: [1, 0],
        float_60: 2.0,
        word_17c: 7,
        matrix: m,
        cell: [12, 34],
        heights: [1.0, 1.5, 1.0, 1.5],
        class_id: 0x49C0_5B9F,
        ..UndergroundTail::default()
    }
}

#[test]
fn underground_tail_worked_example_is_137_exact_bytes() {
    let rec = UndergroundRecord {
        base: NetworkBaseRecord {
            base: OccupantBase::new(0x7475_8925),
            ..NetworkBaseRecord::default()
        },
        tail: sample_tail(),
    };
    let mut w = ByteWriter::new();
    assert!(write_underground_record(&rec, &mut w));
    let b = w.into_bytes();
    assert_eq!(&b[0..2], &[3, 0]);
    assert_eq!(&b[2..4], &[8, 0]);
    let base = encode(&rec.base);
    assert_eq!(&b[2..2 + base.len()], &base[..]);
    let t = &b[2 + base.len()..];
    assert_eq!(t.len(), 137);
    assert_eq!(rec.tail.encoded_len(), 137);
    let mut want = vec![1, 0, 0, 0, 0, 0x40, 7, 0, 0, 0];
    want.extend_from_slice(&[0; 24]);
    for i in 0..16 {
        let v: f32 = if i % 5 == 0 { 1.0 } else { 0.0 };
        want.extend_from_slice(&v.to_le_bytes());
    }
    want.extend_from_slice(&[0x0C, 0, 0, 0, 0x22, 0, 0, 0, 0, 0, 0]);
    want.extend_from_slice(&[0, 0, 0x80, 0x3F, 0, 0, 0xC0, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0xC0, 0x3F]);
    want.extend_from_slice(&[0, 0, 0, 0, 0x9F, 0x5B, 0xC0, 0x49, 0, 0, 0, 0]);
    assert_eq!(t, &want[..]);
    let back = read_underground_record(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(back.tail, rec.tail);
}

#[test]
fn underground_walls_pipe_and_subway_records() {
    let mut tail = sample_tail();
    tail.walls[2] = vec![vtx(1.0, 2.0, 3.0, 0x0001_0203); 2];
    tail.walls[5] = vec![vtx(4.0, 5.0, 6.0, 0x8000_0000)];
    let rec = UndergroundRecord {
        base: sample_network_record(),
        tail,
    };
    assert_eq!(rec.tail.encoded_len(), 137 + 72);
    // Pipe: u16 3, u16 3, u16 8 ... then the u32 +0x270.
    let mut w = ByteWriter::new();
    assert!(write_pipe_record(&rec, 0xABCD, &mut w));
    let b = w.into_bytes();
    assert_eq!(&b[0..6], &[3, 0, 3, 0, 8, 0]);
    assert_eq!(&b[b.len() - 4..], &[0xCD, 0xAB, 0, 0]);
    let (back, w270) = read_pipe_record(&mut ByteReader::new(&b)).unwrap();
    assert_eq!(w270, 0xABCD);
    // Wall vertices keep their alpha (no forcing).
    assert_eq!(back.tail.walls[2][0].color, 0x0001_0203);
    assert_eq!(back.tail, rec.tail);
    // Subway: u16 2 then the underground record; no extra field.
    let mut w = ByteWriter::new();
    assert!(write_subway_record(&rec, &mut w));
    let s = w.into_bytes();
    assert_eq!(s.len(), b.len() - 4);
    assert_eq!(&s[0..4], &[2, 0, 3, 0]);
    assert!(read_subway_record(&mut ByteReader::new(&s)).is_some());
    // Exact versions only.
    let mut bad = s.clone();
    bad[0] = 3;
    assert!(read_subway_record(&mut ByteReader::new(&bad)).is_none());
    let mut bad = b.clone();
    bad[0] = 2;
    assert!(read_pipe_record(&mut ByteReader::new(&bad)).is_none());
    let mut bad = b.clone();
    bad[2] = 4;
    assert!(read_pipe_record(&mut ByteReader::new(&bad)).is_none());
    // A failed base record skips the tail.
    let mut bad = b[2..].to_vec();
    bad[2] = 3;
    assert!(read_underground_record(&mut ByteReader::new(&bad)).is_none());

    // Applying a read record to the live piece.
    let mut u = UndergroundPiece::new(NetworkPiece::new(NetOccClass::Pipe));
    u.apply_record(&back);
    u.mark_loaded_subclass();
    assert_eq!(u.bytes_261, [1, 0]);
    assert_eq!(u.byte_260, 1);
    assert_eq!(u.cell, [12, 34]);
    assert_eq!(u.get_position(), [8.0, 1.0, 8.0]);
}

struct UgHost {
    found: Option<u32>,
    asked: Vec<(u32, i32, i32)>,
    refreshes: u32,
    height_updates: u32,
}

impl UndergroundHost for UgHost {
    fn find_underground_flags(&mut self, class_id: u32, x: i32, z: i32) -> Option<u32> {
        self.asked.push((class_id, x, z));
        self.found
    }
    fn refresh_cell_value(&mut self) {
        self.refreshes += 1;
    }
    fn terrain_heights_changed(&mut self) {
        self.height_updates += 1;
    }
}

fn host(found: Option<u32>) -> UgHost {
    UgHost {
        found,
        asked: Vec::new(),
        refreshes: 0,
        height_updates: 0,
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
    assert_eq!(s.class_id, 0xCA16_374F);

    // §3.2 example: x 0/16/16/0, z 0/0/16/16, heights 3, 2.5, 4, 2.5.
    for (v, (x, z)) in u
        .quad
        .iter_mut()
        .zip([(0.0, 0.0), (16.0, 0.0), (16.0, 16.0), (0.0, 16.0)])
    {
        v.pos = [x, 99.0, z];
    }
    u.heights = [3.0, 2.5, 4.0, 2.5];
    assert_eq!(u.get_position(), [8.0, 2.5, 8.0]);
    // NaN never wins; all FLT_MAX gives FLT_MAX.
    u.heights = [f32::NAN, 5.0, f32::NAN, 6.0];
    assert_eq!(u.get_position()[1], 5.0);
    u.heights = [f32::MAX; 4];
    assert_eq!(u.get_position()[1], f32::MAX);

    let mut h = host(None);
    u.set_terrain_heights(&[1.0, 2.0, 3.0, 4.0], &mut h);
    assert_eq!(u.get_terrain_heights(), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(h.height_updates, 1);
    u.store_init_heights(&[5.0, 6.0, 7.0, 8.0]);
    assert_eq!(u.get_terrain_heights(), [5.0, 6.0, 7.0, 8.0]);
    u.set_vertex_heights(&[9.0; 4]);
    assert!(u.quad.iter().all(|v| v.pos[1] == 9.0));
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

    // My bit 31 clear: false without a lookup.
    let mut h = host(Some(0));
    assert!(!u.is_imaginary_connection(0, &mut h));
    assert!(h.asked.is_empty());
    u.piece.network_flags = FLAG_IMAGINARY;
    u.cell = [10, 20];
    assert!(u.is_imaginary_connection(0, &mut h));
    assert!(u.is_imaginary_connection(3, &mut h));
    assert_eq!(h.asked, vec![(0x49C0_5B9F, 9, 20), (0x49C0_5B9F, 10, 21)]);
    h.found = Some(FLAG_IMAGINARY | 1);
    assert!(!u.is_imaginary_connection(1, &mut h));
    h.found = None;
    assert!(!u.is_imaginary_connection(2, &mut h));
    assert!(!u.is_imaginary_connection(4, &mut h));

    u.set_watered_state(1, &mut h);
    u.set_watered_state(1, &mut h);
    u.set_watered_state(0, &mut h);
    assert_eq!(h.refreshes, 2);
    u.pipe_270 = 5;
    u.pipe_init_tail();
    assert_eq!(u.pipe_270, 0);
}
