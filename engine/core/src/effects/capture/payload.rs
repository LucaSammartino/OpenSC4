// SPDX-License-Identifier: GPL-3.0-or-later
//! EF21 retained payload copies. This module does not attach an observer or
//! certify quiescence. Call only inside a verified outer owner boundary.
//! Raw capture is evidence, never an independent expected value.
use super::{half, inline_vector, word, Memory, Reader};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Byte(u8),
    Half(u16),
    Word(u32),
    Bytes(Vec<u8>),
    Words(Vec<u32>),
    Rows(Vec<Payload>),
    /// EF21: +1C8 has no deterministic default. Preserve provenance without
    /// allowing an uninitialized word to become a deterministic golden.
    Indeterminate(u32),
}
pub type Payload = BTreeMap<u32, Value>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Particle,
    Dynamic,
    Decal,
    Shake,
    Light,
    Component(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// Borrowed address for trace-local alias relationships only. No pointer
    /// escapes to a worker or is dereferenced after this synchronous copy.
    pub identity: Option<u32>,
    /// None denotes a foreign reserved type, never an empty/exact payload.
    pub payload: Option<Payload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    pub kind: Kind,
    pub slots: Vec<Slot>,
    pub names: Vec<(Vec<u8>, u32)>,
}

impl Family {
    pub fn supported(&self) -> bool {
        // EF23: sequence flags are outside EF21's stated extent. Its listed
        // vector is useful evidence, but does not establish complete payload.
        !(self.kind == Kind::Component(6) && !self.slots.is_empty()) && self.slots.iter().all(|s| s.payload.is_some())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Builtins {
    pub families: Vec<Family>,
}

impl Builtins {
    pub fn supported(&self) -> bool {
        self.families.iter().all(Family::supported)
    }
}

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|b| word(b, 0)).collect()
}

fn scalars(p: &mut Payload, b: &[u8], start: usize, end: usize) {
    for offset in (start..=end).step_by(4) {
        p.insert(offset as u32, Value::Word(word(b, offset)));
    }
}

fn vector<M: Memory>(r: &mut Reader<M>, p: &mut Payload, b: &[u8], offset: usize, stride: usize) -> Option<()> {
    let bytes = inline_vector(r, b, offset, stride)?;
    p.insert(offset as u32, Value::Words(words(&bytes)));
    Some(())
}

fn string<M: Memory>(r: &mut Reader<M>, p: &mut Payload, b: &[u8], offset: usize) -> Option<()> {
    p.insert(offset as u32, Value::Bytes(r.string(b, offset)?));
    Some(())
}

fn nested<M: Memory>(
    r: &mut Reader<M>,
    p: &mut Payload,
    b: &[u8],
    offset: usize,
    stride: usize,
    string_at: usize,
) -> Option<()> {
    let bytes = inline_vector(r, b, offset, stride)?;
    let mut rows = Vec::new();
    for row in bytes.chunks_exact(stride) {
        let mut value = Payload::new();
        for at in (0..stride).step_by(4) {
            if at == string_at {
                string(r, &mut value, row, at)?;
            } else if !(string_at..string_at + 12).contains(&at) {
                value.insert(at as u32, Value::Word(word(row, at)));
            }
        }
        rows.push(value);
    }
    p.insert(offset as u32, Value::Rows(rows));
    Some(())
}

fn extent(kind: Kind) -> Option<usize> {
    Some(match kind {
        Kind::Particle => 0x204,
        Kind::Dynamic => 0x40,
        Kind::Decal => 0x64,
        Kind::Shake => 0x28,
        Kind::Light => 0x1c,
        Kind::Component(3) => 0x34,
        Kind::Component(4 | 7) => 0x1c,
        Kind::Component(5) => 0x50,
        Kind::Component(6 | 8) => 0x18,
        Kind::Component(_) => return None,
    })
}

fn payload<M: Memory>(r: &mut Reader<M>, kind: Kind, b: &[u8]) -> Option<Payload> {
    let mut p = Payload::new();
    match kind {
        Kind::Particle => {
            scalars(&mut p, b, 0, 0x84);
            for (at, stride) in [
                (0x88, 4),
                (0x94, 12),
                (0xa0, 4),
                (0xac, 4),
                (0xb8, 4),
                (0xc4, 4),
                (0x108, 0x1c),
                (0x12c, 4),
                (0x1a4, 4),
                (0x1b8, 0x20),
                (0x1e0, 4),
                (0x1f4, 4),
            ] {
                vector(r, &mut p, b, at, stride)?;
            }
            scalars(&mut p, b, 0xd0, 0xd0);
            p.insert(0xd4, Value::Byte(b[0xd4]));
            p.insert(0xd5, Value::Byte(b[0xd5]));
            for (start, end) in [
                (0xd8, 0x104),
                (0x114, 0x128),
                (0x138, 0x15c),
                (0x170, 0x1a0),
                (0x1b0, 0x1b4),
                (0x1c4, 0x1c4),
                (0x1d8, 0x1dc),
                (0x1ec, 0x1f0),
                (0x200, 0x200),
            ] {
                scalars(&mut p, b, start, end);
            }
            string(r, &mut p, b, 0x160)?;
            p.insert(0x16c, Value::Half(half(b, 0x16c)));
            p.insert(0x16e, Value::Half(half(b, 0x16e)));
            p.insert(0x1c8, Value::Indeterminate(word(b, 0x1c8)));
            nested(r, &mut p, b, 0x1cc, 0x10, 0)?;
        }
        Kind::Dynamic => {
            scalars(&mut p, b, 8, 8);
            string(r, &mut p, b, 0xc)?;
            scalars(&mut p, b, 0x18, 0x30);
            vector(r, &mut p, b, 0x34, 4)?;
        }
        Kind::Decal => {
            scalars(&mut p, b, 0, 4);
            p.insert(8, Value::Byte(b[8]));
            p.insert(9, Value::Byte(b[9]));
            scalars(&mut p, b, 0xc, 0xc);
            for (at, stride) in [(0x10, 4), (0x1c, 4), (0x28, 4), (0x34, 12), (0x40, 4)] {
                vector(r, &mut p, b, at, stride)?;
            }
            scalars(&mut p, b, 0x4c, 0x60);
        }
        Kind::Shake => {
            scalars(&mut p, b, 0, 4);
            vector(r, &mut p, b, 8, 4)?;
            vector(r, &mut p, b, 0x14, 4)?;
            scalars(&mut p, b, 0x20, 0x24);
        }
        Kind::Light => {
            vector(r, &mut p, b, 0, 12)?;
            vector(r, &mut p, b, 0xc, 4)?;
            scalars(&mut p, b, 0x18, 0x18);
        }
        Kind::Component(3) => {
            scalars(&mut p, b, 0xc, 0x14);
            p.insert(0x18, Value::Byte(b[0x18]));
            scalars(&mut p, b, 0x1c, 0x30);
        }
        Kind::Component(4) => {
            string(r, &mut p, b, 0xc)?;
            p.insert(0x18, Value::Byte(b[0x18]));
        }
        Kind::Component(5) => scalars(&mut p, b, 0xc, 0x4c),
        Kind::Component(6) => nested(r, &mut p, b, 0xc, 0x14, 8)?,
        Kind::Component(7) => scalars(&mut p, b, 0xc, 0x18),
        Kind::Component(8) => {
            scalars(&mut p, b, 0xc, 0xc);
            p.insert(0x10, Value::Byte(b[0x10]));
            p.insert(0x11, Value::Byte(b[0x11]));
            scalars(&mut p, b, 0x14, 0x14);
        }
        Kind::Component(_) => return None,
    }
    Some(p)
}

fn family<M: Memory>(r: &mut Reader<M>, c: u32, offset: u32, kind: Kind, pointers: bool) -> Option<Family> {
    let size = extent(kind);
    let stride = if pointers { 4 } else { size? };
    let records = r.vector(c.checked_add(offset)?, stride)?;
    let names = r.names(c.checked_add(offset.checked_sub(0x14)?)?, records.len() / stride)?;
    let mut slots = Vec::new();
    for bytes in records.chunks_exact(stride) {
        let identity = pointers.then(|| word(bytes, 0));
        if identity == Some(0) {
            return None;
        }
        let payload = match (identity, size) {
            (Some(address), Some(size)) => {
                let b = r.bytes(address, size)?;
                Some(payload(r, kind, &b)?)
            }
            (Some(_), None) => None,
            (None, _) => Some(payload(r, kind, bytes)?),
        };
        slots.push(Slot { identity, payload });
    }
    Some(Family { kind, slots, names })
}

/// Capture every EF21 built-in family, preserving anonymous slots, aliases,
/// indeterminate values and foreign unsupported framing. Excludes visuals,
/// derived publication tables and live effects; no whole-owner credit implied.
pub fn copy_builtins(memory: impl Memory, collection: u32) -> Option<Builtins> {
    let mut r = Reader::new(memory);
    let mut families = Vec::new();
    for (offset, kind, pointers) in [
        (0x38, Kind::Particle, true),
        (0x58, Kind::Dynamic, true),
        (0x78, Kind::Decal, false),
        (0x98, Kind::Shake, false),
        (0xb8, Kind::Light, false),
    ] {
        families.push(family(&mut r, collection, offset, kind, pointers)?);
    }
    let directories = r.vector(collection.checked_add(0xc4)?, 0x20)?;
    // Literal EF20/EF21: actual end defines readable slots. A nonstandard
    // directory count is retained, not silently synthesized as thirteen.
    if directories.len() / 0x20 > 13 {
        return None;
    }
    let header = r.bytes(collection.checked_add(0xc4)?, 12)?;
    let begin = word(&header, 0);
    for i in 0..directories.len() / 0x20 {
        families.push(family(
            &mut r,
            begin,
            (i * 0x20 + 0x14) as u32,
            Kind::Component(i as u8 + 3),
            true,
        )?);
    }
    Some(Builtins { families })
}
