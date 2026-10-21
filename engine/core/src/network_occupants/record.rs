// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//! The base network-occupant save record (`network-occupants.md` §2.3,
//! `Write` `0x0061A190` v8 / `Read` `0x0061CC00` v ≥ 4).
//!
//! The record is kept in **stream order** as a literal field list: §2.3 and
//! §2.2 disagree on several offsets (the quad block, the triangle row, the
//! bounding box pairs), so the live-object ↔ record mapping is not made here
//! (question NO16).
//!
//! Blocked parts (each a numbered question):
//! - records below v5 use `S3DTransformLegacyLoad`, whose layout is not
//!   specified (NO15): [`read_network_record`] fails on them;
//! - `ConvertV5FlagsToCurrentFlags` (records below v6) is not specified
//!   (NO16): the flags are kept raw (then masked) and
//!   [`NetworkBaseRecord::flags_need_v5_conversion`] reports it;
//! - the `Read` tail's position fix is reported as a [`LoadFixup`] decision
//!   only; the floats it passes are not identified (NO16).

use super::xform::Xform;
use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

/// The version `Write` emits.
pub const NETWORK_RECORD_VERSION: u16 = 8;
/// The oldest version `Read` accepts.
pub const NETWORK_RECORD_MIN_VERSION: u16 = 4;
/// `Read` masks the saved flags with this, every version.
pub const SAVED_FLAGS_MASK: u32 = 0xFFFF_1FFF;
/// The `+0x100` sub-record's only accepted version.
pub const SUB100_VERSION: u16 = 2;
/// The IID `Write` QIs the `+0x138` reference for (segment-o-stream QI).
pub const IID_OWNED_REF_WRITE: u32 = 0x6999_A8FB;
/// The IID `Read` reads the `+0x138` reference as.
pub const IID_OWNED_REF_READ: u32 = 0xA966_7793;

/// Field 4: one road quad, five u32s then four bytes in stream order
/// (the spec's `+0xA`, `+0x9`, `+0x8`, `+0xB`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadRecord {
    pub words: [u32; 5],
    pub bytes: [u8; 4],
}

/// Field 5: u32 `+0xF4`, u8 `+0xFD`, u32 `+0xF8`, u8 `+0xFC`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadTail {
    pub word_f4: u32,
    pub byte_fd: u8,
    pub word_f8: u32,
    pub byte_fc: u8,
}

/// Field 6: the `+0x100` sub-record (helper `0x00619C10` / `0x00619C90`):
/// u16 version 2, u8, u8, u32.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sub100 {
    pub a: u8,
    pub b: u8,
    pub c: u32,
}

/// Field 8: one triangle entry. The spec's type column is used literally
/// (u32 + 3 floats + 3 floats + u32 + u32 + 4 × u8, 40 bytes); its notes
/// column names fewer fields (NO16).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TriangleRecord {
    pub word0: u32,
    pub p: [f32; 3],
    pub q: [f32; 3],
    pub word1: u32,
    pub word2: u32,
    pub bytes: [u8; 4],
}

/// The base network record in stream order.
#[derive(Clone, Debug)]
pub struct NetworkBaseRecord {
    /// The version read (written as [`NETWORK_RECORD_VERSION`]).
    pub version: u16,
    pub base: OccupantBase,
    pub transform: Xform,
    pub quads: [QuadRecord; 4],
    pub quad_tail: QuadTail,
    pub sub100: Sub100,
    pub triangles: Vec<TriangleRecord>,
    /// Field 9: the bounding box as three u32 pairs, in stream order.
    pub bbox_words: [u32; 6],
    /// Field 10, already masked with [`SAVED_FLAGS_MASK`] after a read.
    pub saved_flags: u32,
    /// Field 11 (`+0x15C`).
    pub word_15c: u32,
    /// Field 12 (v8 only; `Null` below).
    pub owned_ref: RefValue,
    /// Field 13 (v8 only; 0 below).
    pub int64: i64,
}

impl Default for NetworkBaseRecord {
    fn default() -> Self {
        NetworkBaseRecord {
            version: NETWORK_RECORD_VERSION,
            base: OccupantBase::default(),
            transform: Xform::default(),
            quads: [QuadRecord::default(); 4],
            quad_tail: QuadTail::default(),
            sub100: Sub100::default(),
            triangles: Vec::new(),
            bbox_words: [0; 6],
            saved_flags: 0,
            word_15c: 0,
            owned_ref: RefValue::Null,
            int64: 0,
        }
    }
}

/// What `Read`'s tail does with the loaded position (§2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadFixup {
    /// v < 7 and `+0x15C` ≠ 0: recompute the occupied cell (§2.4 formula)
    /// and call the `+0x34` object's slot 0x48.
    RecomputeCell,
    /// v ≥ 7: call the `+0x34` object's slot 0x120 with two floats (which
    /// two is NO16).
    ApplyAltitude,
    /// v < 7 and `+0x15C` = 0. *[literal: §2.3 names no action]*
    None,
}

impl NetworkBaseRecord {
    /// True when the record predates v6 and its flags would need
    /// `ConvertV5FlagsToCurrentFlags` (not specified, NO16).
    pub fn flags_need_v5_conversion(&self) -> bool {
        self.version < 6
    }

    /// The tail decision of `Read` (§2.3).
    pub fn load_fixup(&self) -> LoadFixup {
        if self.version < 7 {
            if self.word_15c != 0 {
                LoadFixup::RecomputeCell
            } else {
                LoadFixup::None
            }
        } else {
            LoadFixup::ApplyAltitude
        }
    }
}

/// `Write` (`0x0061A190`): always version 8; true when the stream has no
/// error afterwards.
pub fn write_network_record(rec: &NetworkBaseRecord, out: &mut dyn OutStream) -> bool {
    out.put_u16(NETWORK_RECORD_VERSION);
    if !rec.base.write(out) {
        return false;
    }
    rec.transform.write(out);
    for q in &rec.quads {
        for &w in &q.words {
            out.put_u32(w);
        }
        for &b in &q.bytes {
            out.put_u8(b);
        }
    }
    out.put_u32(rec.quad_tail.word_f4);
    out.put_u8(rec.quad_tail.byte_fd);
    out.put_u32(rec.quad_tail.word_f8);
    out.put_u8(rec.quad_tail.byte_fc);
    out.put_u16(SUB100_VERSION);
    out.put_u8(rec.sub100.a);
    out.put_u8(rec.sub100.b);
    out.put_u32(rec.sub100.c);
    out.put_u32(rec.triangles.len() as u32);
    for t in &rec.triangles {
        out.put_u32(t.word0);
        for &v in t.p.iter().chain(t.q.iter()) {
            out.put_f32(v);
        }
        out.put_u32(t.word1);
        out.put_u32(t.word2);
        for &b in &t.bytes {
            out.put_u8(b);
        }
    }
    for &w in &rec.bbox_words {
        out.put_u32(w);
    }
    out.put_u32(rec.saved_flags);
    out.put_u32(rec.word_15c);
    out.put_ref(&rec.owned_ref);
    out.put_i64(rec.int64);
    out.error() == 0
}

/// `Read` (`0x0061CC00`). `None` for a version below 4 (nothing more read),
/// for v4 (legacy transform, NO15), for a `+0x100` sub-record version other
/// than 2, and on a short stream.
pub fn read_network_record(inp: &mut dyn InStream) -> Option<NetworkBaseRecord> {
    let version = inp.get_u16()?;
    if version < NETWORK_RECORD_MIN_VERSION {
        return None;
    }
    let base = OccupantBase::read(inp)?;
    if version < 5 {
        // S3DTransformLegacyLoad: layout not specified (NO15).
        return None;
    }
    let transform = Xform::read(inp)?;
    let mut quads = [QuadRecord::default(); 4];
    for q in quads.iter_mut() {
        for w in q.words.iter_mut() {
            *w = inp.get_u32()?;
        }
        for b in q.bytes.iter_mut() {
            *b = inp.get_u8()?;
        }
    }
    let quad_tail = QuadTail {
        word_f4: inp.get_u32()?,
        byte_fd: inp.get_u8()?,
        word_f8: inp.get_u32()?,
        byte_fc: inp.get_u8()?,
    };
    if inp.get_u16()? != SUB100_VERSION {
        return None;
    }
    let sub100 = Sub100 {
        a: inp.get_u8()?,
        b: inp.get_u8()?,
        c: inp.get_u32()?,
    };
    let count = inp.get_u32()?;
    // No capacity from the stream's count: a corrupt count fails on the
    // first short read instead of allocating.
    let mut triangles = Vec::new();
    for _ in 0..count {
        let word0 = inp.get_u32()?;
        let mut p = [0.0; 3];
        let mut q = [0.0; 3];
        for v in p.iter_mut().chain(q.iter_mut()) {
            *v = inp.get_f32()?;
        }
        triangles.push(TriangleRecord {
            word0,
            p,
            q,
            word1: inp.get_u32()?,
            word2: inp.get_u32()?,
            bytes: [inp.get_u8()?, inp.get_u8()?, inp.get_u8()?, inp.get_u8()?],
        });
    }
    let mut bbox_words = [0u32; 6];
    for w in bbox_words.iter_mut() {
        *w = inp.get_u32()?;
    }
    // Below v6 the game converts first (NO16); the mask applies always.
    let saved_flags = inp.get_u32()? & SAVED_FLAGS_MASK;
    let word_15c = inp.get_u32()?;
    let (owned_ref, int64) = if version >= 8 {
        (inp.get_ref_as(IID_OWNED_REF_READ)?, inp.get_i64()?)
    } else {
        (RefValue::Null, 0)
    };
    if inp.error() != 0 {
        return None;
    }
    Some(NetworkBaseRecord {
        version,
        base,
        transform,
        quads,
        quad_tail,
        sub100,
        triangles,
        bbox_words,
        saved_flags,
        word_15c,
        owned_ref,
        int64,
    })
}
