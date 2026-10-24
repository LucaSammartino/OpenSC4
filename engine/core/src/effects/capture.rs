// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only retained visual records (effects §18.7). No native owner calls.
use super::{directory::EffectDescription, transform::Placement, EffectDirectory};
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

pub mod observation;
pub mod payload;

/// The host copies readable spans at an owner-thread quiescent boundary.
pub trait Memory {
    fn read(&mut self, address: u32, bytes: &mut [u8]) -> bool;
}

struct Reader<M> {
    memory: M,
    remaining: usize,
}
impl<M: Memory> Reader<M> {
    fn new(memory: M) -> Self {
        Self {
            memory,
            remaining: 16 * 1024 * 1024,
        }
    }
    fn bytes(&mut self, address: u32, length: usize) -> Option<Vec<u8>> {
        if length > self.remaining || (length != 0 && address == 0) {
            return None;
        }
        address.checked_add(u32::try_from(length).ok()?)?;
        self.remaining -= length;
        let mut bytes = vec![0; length];
        (length == 0 || self.memory.read(address, &mut bytes)).then_some(bytes)
    }
    fn word(&mut self, address: u32) -> Option<u32> {
        Some(word(&self.bytes(address, 4)?, 0))
    }
    fn vector(&mut self, address: u32, stride: usize) -> Option<Vec<u8>> {
        let h = self.bytes(address, 12)?;
        let (begin, end, capacity) = (word(&h, 0), word(&h, 4), word(&h, 8));
        let length = end.checked_sub(begin)? as usize;
        if end > capacity || length % stride != 0 || length / stride > 65536 {
            return None;
        }
        self.bytes(begin, length)
    }
    fn string(&mut self, header: &[u8], offset: usize) -> Option<Vec<u8>> {
        let (begin, end, capacity) = (word(header, offset), word(header, offset + 4), word(header, offset + 8));
        if end > capacity || end.checked_sub(begin)? > 4096 {
            return None;
        }
        self.bytes(begin, (end - begin) as usize)
    }
    fn names(&mut self, header: u32, count: usize) -> Option<Vec<(Vec<u8>, u32)>> {
        let buckets = self.vector(header.checked_add(4)?, 4)?;
        let declared = self.word(header.checked_add(16)?)? as usize;
        if declared > 65536 {
            return None;
        }
        let mut seen = BTreeSet::new();
        let mut names = Vec::new();
        for bucket in buckets.chunks_exact(4) {
            let mut node = word(bucket, 0);
            while node != 0 {
                if !seen.insert(node) || seen.len() > declared {
                    return None;
                }
                let bytes = self.bytes(node, 20)?;
                let index = word(&bytes, 16);
                if index as usize >= count {
                    return None;
                }
                names.push((self.string(&bytes, 4)?, index));
                node = word(&bytes, 0);
            }
        }
        (names.len() == declared).then_some(names)
    }
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn half(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

/// Count actual independently compared values, rather than hashing pointers or padding.
#[derive(Default, Debug)]
pub struct Comparison {
    pub records: u32,
    pub fields: u32,
    pub differences: u32,
}
impl Comparison {
    fn check<T: PartialEq>(&mut self, native: T, expected: T) {
        self.fields += 1;
        self.differences += u32::from(native != expected);
    }
}

fn description<M: Memory>(
    r: &mut Reader<M>,
    bytes: &[u8],
    expected: &EffectDescription,
    c: &mut Comparison,
) -> Option<()> {
    c.check(word(bytes, 0), expected.flags);
    c.check(word(bytes, 4), expected.priority);
    for (i, value) in expected.extra.iter().enumerate() {
        c.check(word(bytes, 32 + i * 4), *value);
    }
    c.check(r.string(bytes, 44)?, expected.text.as_bytes().to_vec());
    let component_bytes = inline_vector(r, bytes, 8, 108)?;
    c.check(component_bytes.len() / 108, expected.components.len());
    for (b, e) in component_bytes.chunks_exact(108).zip(&expected.components) {
        c.check(r.string(b, 0)?, e.name.as_bytes().to_vec());
        c.check(b[12], e.kind);
        c.check(word(b, 16), e.flags);
        let p = Placement::decode(&e.placement);
        c.check(b[20], p.revision);
        c.check(b[21] & 7, p.flags & 7);
        for (i, value) in e.placement[..52].chunks_exact(4).enumerate() {
            c.check(word(b, 24 + i * 4), word(value, 0));
        }
        c.check(b[76], e.lod_min);
        c.check(b[77], e.lod_max);
        c.check(half(b, 78), e.copies);
        c.check(half(b, 80), e.spacing);
        for (i, value) in e.render.iter().enumerate() {
            c.check(word(b, 84 + i * 4), value.to_bits());
        }
        c.check(half(b, 100), e.select_group);
        c.check(half(b, 102), e.select_weight);
        c.check(word(b, 104), e.index);
    }
    let bindings = inline_vector(r, bytes, 20, 24)?;
    c.check(bindings.len() / 24, expected.bindings.len());
    for (b, e) in bindings.chunks_exact(24).zip(&expected.bindings) {
        c.check(word(b, 0), e.index);
        c.check(r.string(b, 4)?, e.name.as_bytes().to_vec());
        c.check(word(b, 16), e.value.to_bits());
        c.check(word(b, 20), e.extra);
    }
    Some(())
}

fn inline_vector<M: Memory>(r: &mut Reader<M>, bytes: &[u8], offset: usize, stride: usize) -> Option<Vec<u8>> {
    let (begin, end, capacity) = (word(bytes, offset), word(bytes, offset + 4), word(bytes, offset + 8));
    let length = end.checked_sub(begin)? as usize;
    if end > capacity || length % stride != 0 || length / stride > 65536 {
        return None;
    }
    r.bytes(begin, length)
}

/// Visual-vector payload plus complete (name,index) multiset, preserving duplicate
/// names and stable vector indices. Hash bucket order is an allocation detail.
/// This deliberately does not claim the other families or whole-owner closure.
pub fn compare_visuals(memory: impl Memory, collection: u32, directory: &EffectDirectory) -> Option<Comparison> {
    let mut r = Reader::new(memory);
    let records = r.vector(collection.checked_add(24)?, 56)?;
    let mut c = Comparison {
        records: (records.len() / 56) as u32,
        ..Comparison::default()
    };
    c.check(records.len() / 56, directory.effects.len());
    for (b, e) in records.chunks_exact(56).zip(&directory.effects) {
        description(&mut r, b, e, &mut c)?;
    }
    let mut names = r.names(collection.checked_add(4)?, records.len() / 56)?;
    let mut expected: Vec<_> = directory
        .names
        .iter()
        .map(|(n, i)| (n.as_bytes().to_vec(), *i))
        .collect();
    names.sort();
    expected.sort();
    c.check(names, expected);
    Some(c)
}
