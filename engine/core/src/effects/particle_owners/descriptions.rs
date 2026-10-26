// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §§2–3, 18: value owners and primitive IO, not live effects.
pub use crate::effects_parser::script::commands::children::SequenceElement;
use crate::effects_parser::script::commands::definitions::{DefinitionKind, Description};
use std::sync::Arc;

/// Foreign stream primitives. Strings are byte strings; no native pointers are
/// serialized. Errors propagate at the operation that failed, without rollback.
pub trait Read {
    type Error;
    fn u8(&mut self) -> Result<u8, Self::Error>;
    fn u16(&mut self) -> Result<u16, Self::Error>;
    fn u32(&mut self) -> Result<u32, Self::Error>;
    fn f32(&mut self) -> Result<f32, Self::Error>;
    fn string(&mut self) -> Result<Vec<u8>, Self::Error>;
}
pub trait Write {
    type Error;
    fn u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn f32(&mut self, value: f32) -> Result<(), Self::Error>;
    fn string(&mut self, value: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequenceDescription {
    pub elements: Vec<SequenceElement>,
    pub flags: u32,
}
/// Allocation is foreign; the core supplies the retained description to the
/// constructor only after allocation succeeds. No sequence is played here.
pub trait SequenceAllocator {
    type Allocation;
    type Component;
    fn allocate(&mut self, bytes: usize) -> Option<Self::Allocation>;
    fn construct(&mut self, allocation: Self::Allocation, description: Arc<SequenceDescription>) -> Self::Component;
}
impl SequenceDescription {
    pub fn create<A: SequenceAllocator>(self: &Arc<Self>, allocator: &mut A) -> Option<A::Component> {
        allocator
            .allocate(168)
            .map(|allocation| allocator.construct(allocation, Arc::clone(self)))
    }
    pub fn read<S: Read>(&mut self, stream: &mut S) -> Result<(), S::Error> {
        // Literal §18.1: consume but do not validate version.
        stream.u16()?;
        let count = stream.u32()?;
        self.elements.resize_with(count as usize, || SequenceElement {
            first: 0.,
            second: 0.,
            name: Vec::new(),
        });
        for element in &mut self.elements {
            element.first = stream.f32()?;
            element.second = stream.f32()?;
            element.name = stream.string()?;
        }
        self.flags = stream.u32()? & 7;
        Ok(())
    }
    pub fn write<S: Write>(&self, stream: &mut S) -> Result<(), S::Error> {
        stream.u16(1)?;
        stream.u32(self.elements.len() as u32)?;
        for element in &self.elements {
            stream.f32(element.first)?;
            stream.f32(element.second)?;
            stream.string(&element.name)?;
        }
        stream.u32(self.flags)
    }
}

/// Explicitly provided runtime globals, never guessed from pointer-like images.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleGlobals {
    pub scalar: f32,
    pub colour_curve: [f32; 3],
}
/// Reuse the parser's value storage. Clone deep-copies every owned container;
/// no refcount or runtime interface exists in this base.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleDescriptionBase {
    pub value: Description,
}
impl ParticleDescriptionBase {
    pub fn new(globals: Option<ParticleGlobals>) -> Self {
        let mut value = Description::reset(DefinitionKind::Particles, &Description::default());
        value.strings.entry(0x160).or_default();
        for offset in [0x12c, 0x1a4] {
            value.floats.entry(offset).or_default();
        }
        for offset in [0x108, 0x1b8] {
            value.vectors.entry(offset).or_default();
        }
        // particles.md §2 PT3: the three scalar seeds are positive zero.
        for base in [0x7c, 0xe0, 0xf4, 0x18c] {
            for offset in [base, base + 4, base + 8] {
                value.scalar(offset, globals.as_ref().map_or(0., |g| g.scalar));
            }
        }
        value.vectors.remove(&0x94);
        if let Some(g) = globals {
            value.vectors.insert(0x94, vec![g.colour_curve]);
        }
        Self { value }
    }
    pub fn assign(&mut self, source: &Self) {
        // Literal §18.2: parser storage can also carry the derived description's
        // +204 refcount. Base assignment must never copy that trailing word.
        let derived_count = self.value.words.get(&0x204).copied();
        self.value.clone_from(&source.value);
        if let Some(count) = derived_count {
            self.value.words.insert(0x204, count);
        } else {
            self.value.words.remove(&0x204);
        }
    }

    pub fn read<S: Read>(&mut self, version: u16, stream: &mut S) -> Result<(), S::Error> {
        let d = &mut self.value;
        d.words.insert(0, stream.u32()?);
        if version != 1 {
            d.words.insert(4, stream.u32()? & 0xff);
        }
        d.words.insert(8, stream.u32()? & 0x7ff);
        for offset in (0x0c..=0x14).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        d.words.insert(0x18, stream.u32()?);
        for offset in (0x1c..=0x84).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        read_floats(d, 0x88, stream)?;
        let count = stream.u32()?;
        let values = d.vectors.entry(0x94).or_default();
        values.clear();
        for _ in 0..count {
            values.push([stream.f32()?, stream.f32()?, stream.f32()?]);
        }
        for offset in [0xa0, 0xac, 0xb8, 0xc4] {
            read_floats(d, offset, stream)?;
        }
        d.words.insert(0xd0, stream.u32()?);
        for offset in [0xd4, 0xd5] {
            d.bytes.insert(offset, stream.u8()?);
        }
        for offset in (0xd8..=0x104).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        // On disk each tractor/wiggle is exactly TWO vec3 values (§2).
        // Parser's larger records are deliberately separate storage formats.
        read_vectors(d, 0x108, stream)?;
        for offset in (0x114..=0x128).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        read_floats(d, 0x12c, stream)?;
        for offset in (0x138..=0x15c).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        d.strings.insert(0x160, stream.string()?);
        let first = u32::from(stream.u16()?);
        d.words.insert(0x16c, first | (u32::from(stream.u16()?) << 16));
        for offset in (0x170..=0x1a0).step_by(4) {
            d.scalar(offset, stream.f32()?);
        }
        read_floats(d, 0x1a4, stream)?;
        d.scalar(0x1b0, stream.f32()?);
        d.words.insert(0x1b4, stream.u32()?);
        read_vectors(d, 0x1b8, stream)?;
        d.scalar(0x1c4, stream.f32()?);
        if version != 1 {
            let count = stream.u32()?;
            d.timed_effects.clear();
            for _ in 0..count {
                d.timed_effects.push((stream.string()?, stream.f32()?));
            }
        }
        for offset in [0x1d8, 0x1dc] {
            d.scalar(offset, stream.f32()?);
        }
        let count = stream.u32()?;
        let values = d.lists.entry(0x1e0).or_default();
        values.clear();
        for _ in 0..count {
            values.push(stream.u32()?);
        }
        for offset in [0x1ec, 0x1f0] {
            d.scalar(offset, stream.f32()?);
        }
        read_floats(d, 0x1f4, stream)?;
        d.scalar(0x200, stream.f32()?);
        Ok(())
    }
}
fn read_floats<S: Read>(d: &mut Description, offset: u32, s: &mut S) -> Result<(), S::Error> {
    let count = s.u32()?;
    let values = d.floats.entry(offset).or_default();
    values.clear();
    for _ in 0..count {
        values.push(s.f32()?);
    }
    Ok(())
}
fn read_vectors<S: Read>(d: &mut Description, offset: u32, s: &mut S) -> Result<(), S::Error> {
    let count = s.u32()?;
    let values = d.vectors.entry(offset).or_default();
    values.clear();
    for _ in 0..count {
        values.push([s.f32()?, s.f32()?, s.f32()?]);
        values.push([s.f32()?, s.f32()?, s.f32()?]);
    }
    Ok(())
}
