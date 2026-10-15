// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The encodings of §1 (`vec3`, `time`, `curve`, `bytes`, reference lists),
//! reference identity, the IID check on read, and the `cSC4Occupant` base
//! record of `docs/specs/lot-records.md` §2.3 that every automaton starts with
//! (§6.1 field 1).

use std::rc::Rc;

use crate::property::{ExemplarHolder, ExemplarSource, PropertyHolder};
use crate::stream::{InStream, OutStream, RefValue, ResKey};

use super::{Automaton, Controller};

/// `vec3`: x, y, z.
pub type Vec3 = [f32; 3];

/// The most entries a count may announce before the reader stops trusting it
/// for pre-allocation (the read itself still runs to the count or the first
/// stream error).
const PREALLOC_LIMIT: usize = 4096;

/// A controller time (§1 `time`): `f32` hour (0 ≤ h < 24), then `s16` day of
/// week (0–6).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControllerTime {
    pub hour: f32,
    pub day: i16,
}

pub(crate) fn get_vec3(inp: &mut dyn InStream) -> Option<Vec3> {
    Some([inp.get_f32()?, inp.get_f32()?, inp.get_f32()?])
}

pub(crate) fn put_vec3(out: &mut dyn OutStream, v: &Vec3) -> bool {
    out.put_f32(v[0]) && out.put_f32(v[1]) && out.put_f32(v[2])
}

pub(crate) fn get_time(inp: &mut dyn InStream) -> Option<ControllerTime> {
    Some(ControllerTime {
        hour: inp.get_f32()?,
        day: inp.get_i16()?,
    })
}

pub(crate) fn put_time(out: &mut dyn OutStream, t: &ControllerTime) -> bool {
    out.put_f32(t.hour) && out.put_i16(t.day)
}

/// `curve`: `u32` n, then n × (`f32` x, `f32` y).
pub(crate) fn get_curve(inp: &mut dyn InStream) -> Option<Vec<(f32, f32)>> {
    let n = inp.get_u32()? as usize;
    let mut v = Vec::with_capacity(n.min(PREALLOC_LIMIT));
    for _ in 0..n {
        v.push((inp.get_f32()?, inp.get_f32()?));
    }
    Some(v)
}

pub(crate) fn put_curve(out: &mut dyn OutStream, c: &[(f32, f32)]) -> bool {
    if !out.put_u32(c.len() as u32) {
        return false;
    }
    c.iter().all(|&(x, y)| out.put_f32(x) && out.put_f32(y))
}

/// `bytes`: `u32` n, then n bytes.
pub(crate) fn get_byte_vec(inp: &mut dyn InStream) -> Option<Vec<u8>> {
    let n = inp.get_u32()? as usize;
    inp.get_bytes(n)
}

pub(crate) fn put_byte_vec(out: &mut dyn OutStream, b: &[u8]) -> bool {
    out.put_u32(b.len() as u32) && out.put_bytes(b)
}

/// A `u32` count followed by that many references, each read with `iid`.
pub(crate) fn get_refs(inp: &mut dyn InStream, iid: u32) -> Option<Vec<RefValue>> {
    let n = inp.get_u32()? as usize;
    get_n_refs(inp, n, iid)
}

/// `n` references, each read with `iid`.
pub(crate) fn get_n_refs(inp: &mut dyn InStream, n: usize, iid: u32) -> Option<Vec<RefValue>> {
    let mut v = Vec::with_capacity(n.min(PREALLOC_LIMIT));
    for _ in 0..n {
        v.push(get_ref_checked(inp, iid)?);
    }
    Some(v)
}

pub(crate) fn put_refs(out: &mut dyn OutStream, refs: &[RefValue]) -> bool {
    out.put_u32(refs.len() as u32) && refs.iter().all(|r| out.put_ref(r))
}

/// A `key3`.
pub(crate) fn get_key3(inp: &mut dyn InStream) -> Option<ResKey> {
    inp.get_res_key()
}

/// "Read checks IID X" (§1): the reference is read with `iid`; a core object
/// of this layer that does not answer `iid` reads as null (the stream stays
/// good), as a host object without the interface does (foundations §2.7).
pub fn get_ref_checked(inp: &mut dyn InStream, iid: u32) -> Option<RefValue> {
    let r = inp.get_ref_as(iid)?;
    Some(if ref_answers(&r, iid) { r } else { RefValue::Null })
}

/// Whether a reference's object answers `iid`: decided for the core objects
/// of this layer, assumed for everything else (host objects were already
/// queried by the host stream).
pub fn ref_answers(r: &RefValue, iid: u32) -> bool {
    match r {
        RefValue::Object(o) => {
            let any = o.clone().as_any_rc();
            if let Ok(c) = any.clone().downcast::<Controller>() {
                return c.answers_iid(iid);
            }
            if let Ok(a) = any.downcast::<Automaton>() {
                return a.answers_iid(iid);
            }
            true
        }
        _ => true,
    }
}

/// The identity of a referenced object, for lookups by occupant and the like:
/// a host token, or the address of a core object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RefId {
    Null,
    Token(u64),
    Object(usize),
}

/// The identity of `r` (see [`RefId`]).
pub fn ref_id(r: &RefValue) -> RefId {
    match r {
        RefValue::Null => RefId::Null,
        RefValue::Host { token, .. } => RefId::Token(*token),
        RefValue::Object(o) => RefId::Object(Rc::as_ptr(o) as *const () as usize),
    }
}

/// Two references name the same object.
pub fn same_ref(a: &RefValue, b: &RefValue) -> bool {
    ref_id(a) == ref_id(b)
}

/// `RefValue` has no `PartialEq`: references compare by identity and class.
pub fn refs_equal(a: &RefValue, b: &RefValue) -> bool {
    match (a, b) {
        (RefValue::Null, RefValue::Null) => true,
        (RefValue::Host { token: t1, class_id: c1 }, RefValue::Host { token: t2, class_id: c2 }) => {
            t1 == t2 && c1 == c2
        }
        (RefValue::Object(_), RefValue::Object(_)) => same_ref(a, b),
        _ => false,
    }
}

pub(crate) fn ref_lists_equal(a: &[RefValue], b: &[RefValue]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| refs_equal(x, y))
}

// ---- the occupant base record (lot-records §2.3) --------------------------------------------------------

/// The `cSC4Occupant` base record's version (lot-records §2.3: anything else
/// fails the read).
pub const OCCUPANT_BASE_VERSION: u16 = 4;

/// The `cSC4Occupant` base block (lot-records §2.3): 14 bytes, then the
/// exemplar holder.
#[derive(Clone, Debug, Default)]
pub struct OccupantBase {
    /// The removal counters word.
    pub removal: u16,
    /// `GetHighlight()`; written as is.
    pub highlight: u8,
    /// The status byte: `(highlight << 4) | 2` after a read ("not yet
    /// initialised, loaded from a save"). Not saved.
    pub status: u8,
    /// The flag byte; written with bits 1 and 5 cleared (`& 0xDD`). Bit 2 is
    /// "visible" (automaton slot 9 `SetVisible`).
    pub flags: u8,
    /// The occupant type; `0x496E5783` for automata (§6.1).
    pub occupant_type: u32,
    /// The occupant manager's spatial-index rectangle x1, z1, x2, z2.
    pub cell_rect: [u8; 4],
    /// The exemplar property holder (foundations §10.3). The parent key is
    /// kept even when no exemplar resolves it (see [`KeepParentKeys`]).
    pub holder: ExemplarHolder,
}

/// Compares the holder by its own properties and its parent **key** (an
/// `ExemplarHolder` holds a shared exemplar and has no `PartialEq`).
impl PartialEq for OccupantBase {
    fn eq(&self, other: &Self) -> bool {
        self.removal == other.removal
            && self.highlight == other.highlight
            && self.status == other.status
            && self.flags == other.flags
            && self.occupant_type == other.occupant_type
            && self.cell_rect == other.cell_rect
            && self.holder.own == other.holder.own
            && self.holder.parent_key() == other.holder.parent_key()
    }
}

/// The occupant flag bit that `SetVisible` sets (lot-records §2.3: bit 2 =
/// visible).
pub const OCCUPANT_FLAG_VISIBLE: u8 = 4;

impl OccupantBase {
    /// The base block of a freshly constructed occupant of `occupant_type`:
    /// the constructor's flag 4 (visible) set, everything else 0.
    pub fn new(occupant_type: u32) -> OccupantBase {
        OccupantBase {
            flags: OCCUPANT_FLAG_VISIBLE,
            occupant_type,
            ..OccupantBase::default()
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(OCCUPANT_BASE_VERSION);
        out.put_u16(self.removal);
        out.put_u8(self.highlight);
        // Bits 1 and 5 are cleared on write (`& 0xDD`).
        out.put_u8(self.flags & 0xDD);
        out.put_u32(self.occupant_type);
        for &c in &self.cell_rect {
            out.put_u8(c);
        }
        self.holder.write(out) && out.error() == 0
    }

    /// Reads the block; `None` on a version other than 4 or a stream error.
    pub fn read(inp: &mut dyn InStream) -> Option<OccupantBase> {
        if inp.get_u16()? != OCCUPANT_BASE_VERSION {
            return None;
        }
        let mut b = OccupantBase {
            removal: inp.get_u16()?,
            highlight: inp.get_u8()?,
            status: 0,
            flags: inp.get_u8()?,
            occupant_type: inp.get_u32()?,
            cell_rect: [inp.get_u8()?, inp.get_u8()?, inp.get_u8()?, inp.get_u8()?],
            holder: ExemplarHolder::default(),
        };
        b.status = (b.highlight << 4) | 2;
        if !b.holder.read(inp, &KeepParentKeys) {
            return None;
        }
        Some(b)
    }
}

/// An exemplar source that resolves **every** key to an empty exemplar, so an
/// occupant's parent key survives a read even where no resource manager is
/// at hand and the record re-encodes byte-identically *[choice]*. A host that
/// has the exemplars attaches the real parent after the load (foundations
/// §10.3: the parent is resolved against the current data files).
pub struct KeepParentKeys;

impl ExemplarSource for KeepParentKeys {
    fn exemplar(&self, _key: ResKey) -> Option<Rc<PropertyHolder>> {
        Some(Rc::new(PropertyHolder::new()))
    }
}
