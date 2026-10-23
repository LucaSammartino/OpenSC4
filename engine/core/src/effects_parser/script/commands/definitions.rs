// SPDX-License-Identifier: GPL-3.0-or-later
//! Definition construction/publication (§§19.13–16). Field coordinates are the
//! spec's native coordinates, not a binary struct layout or allocator model.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum DefinitionKind {
    Particles,
    DynamicParticle,
    Decal,
    Shake,
    Light,
    Sequence,
}
impl DefinitionKind {
    pub const ALL: [Self; 6] = [
        Self::Particles,
        Self::DynamicParticle,
        Self::Decal,
        Self::Shake,
        Self::Light,
        Self::Sequence,
    ];
    pub fn inherit_slot(self) -> Option<u32> {
        match self {
            Self::Particles => Some(0x20),
            Self::DynamicParticle => Some(0x28),
            Self::Decal => Some(0x30),
            Self::Shake => Some(0x38),
            Self::Light => Some(0x40),
            Self::Sequence => None,
        }
    }
    pub fn publish_slot(self) -> u32 {
        match self {
            Self::Particles => 0x1c,
            Self::DynamicParticle => 0x24,
            Self::Decal => 0x2c,
            Self::Shake => 0x34,
            Self::Light => 0x3c,
            Self::Sequence => 0x44,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Description {
    /// Absence means unassigned/indeterminate, not an implicit zero.
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
    pub floats: BTreeMap<u32, Vec<f32>>,
    pub vectors: BTreeMap<u32, Vec<[f32; 3]>>,
    pub lists: BTreeMap<u32, Vec<u32>>,
    pub strings: BTreeMap<u32, Vec<u8>>,
}
impl Description {
    pub fn scalar(&mut self, at: u32, value: f32) {
        self.words.insert(at, value.to_bits());
    }
    pub fn get_scalar(&self, at: u32) -> Option<f32> {
        self.words.get(&at).map(|&w| f32::from_bits(w))
    }
    pub fn reset(kind: DefinitionKind, old: &Self) -> Self {
        let mut d = Self::default();
        match kind {
            DefinitionKind::Particles => {
                for at in [
                    0x00, 0x04, 0x08, 0x18, 0x1c, 0x30, 0x38, 0x3c, 0x44, 0x48, 0x4c, 0x68, 0x6c, 0x70, 0x74, 0x78,
                    0x7c, 0x80, 0x84, 0xd0, 0xd8, 0xdc, 0xe0, 0xe4, 0xe8, 0xec, 0xf0, 0xf4, 0xf8, 0xfc, 0x100, 0x104,
                    0x114, 0x11c, 0x120, 0x128, 0x138, 0x13c, 0x140, 0x144, 0x148, 0x150, 0x18c, 0x190, 0x194, 0x198,
                    0x19c, 0x1a0, 0x1b4, 0x1c4, 0x1d8, 0x1dc, 0x1ec,
                ] {
                    d.words.insert(at, 0);
                }
                for (ats, value) in [
                    (&[0x0c, 0x10][..], 2.),
                    (&[0x14, 0x34, 0x40, 0x118, 0x124, 0x154, 0x170, 0x1b0][..], 1.),
                    (&[0x20, 0x24, 0x28, 0x2c, 0x50, 0x54, 0x58][..], -1.),
                    (&[0x5c, 0x60, 0x64][..], 1.),
                    (&[0x174, 0x178][..], 5.),
                    (&[0x17c, 0x180][..], 50.),
                    (&[0x1f0, 0x200][..], 4.),
                ] {
                    for &at in ats {
                        d.scalar(at, value);
                    }
                }
                d.words.insert(0x14c, 0xce6e6b28);
                d.scalar(0x158, -10000.);
                d.scalar(0x15c, 10000.);
                // Two 16-bit fields occupy the one native word at +16C.
                d.words.insert(0x16c, 0x0001_0000);
                d.words.insert(0x184, 0x3dcccccd);
                d.words.insert(0x188, 0x3e4ccccd);
                d.bytes.insert(0xd4, 0);
                d.bytes.insert(0xd5, 0);
                for (at, values) in [
                    (0x88, vec![30.]),
                    (0xa0, vec![1.]),
                    (0xac, vec![1.]),
                    (0xb8, vec![1.]),
                    (0xc4, vec![0.]),
                    (0x1f4, vec![400.]),
                ] {
                    d.floats.insert(at, values);
                }
                d.vectors.insert(0x94, vec![[1.; 3]]);
                for at in [0x108, 0x12c, 0x1a4, 0x1b8, 0x1cc, 0x1e0] {
                    d.lists.insert(at, Vec::new());
                }
                d.strings.insert(0x160, Vec::new());
                if let Some(&word) = old.words.get(&0x204) {
                    d.words.insert(0x204, word);
                }
                // +1C8 is indeterminate after temporary assignment, not old/zero.
            }
            DefinitionKind::DynamicParticle => {
                for at in [0x08, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30] {
                    d.words.insert(at, 0);
                }
                d.scalar(0x18, 1.);
                d.strings.insert(0x0c, Vec::new());
                d.lists.insert(0x34, Vec::new());
            }
            DefinitionKind::Decal => {
                for at in [0x00, 0x04, 0x4c, 0x50, 0x54, 0x5c, 0x60] {
                    d.words.insert(at, 0);
                }
                d.bytes.insert(0x08, 0);
                d.bytes.insert(0x09, 1);
                d.scalar(0x0c, 5.);
                d.scalar(0x58, 1.);
                for (at, value) in [(0x10, 0.), (0x1c, 1.), (0x28, 1.), (0x40, 1.)] {
                    d.floats.insert(at, vec![value]);
                }
                d.vectors.insert(0x34, vec![[1.; 3]]);
            }
            DefinitionKind::Shake => {
                for (at, value) in [(0, 5.), (4, 1.), (0x20, 1.)] {
                    d.scalar(at, value);
                }
                d.words.insert(0x24, 0);
                d.floats.insert(0x08, vec![1.]);
                d.floats.insert(0x14, vec![60.]);
            }
            DefinitionKind::Light => {
                d.vectors.insert(0, Vec::new());
                d.floats.insert(0x0c, Vec::new());
                d.scalar(0x18, 2.);
            }
            DefinitionKind::Sequence => {
                d.lists.insert(0x0c, Vec::new());
                d.words.insert(0x18, 0);
            }
        }
        d
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Definition {
    pub name: Vec<u8>,
    pub description: Description,
    /// Stable held-object identity for dynamicParticle; replaced for sequence.
    pub object: Option<Identity>,
    pub sequence_elements: Vec<super::children::SequenceElement>,
}
#[derive(Default)]
pub struct Definitions(pub [Definition; 6]);
impl Definitions {
    pub fn get(&self, kind: DefinitionKind) -> &Definition {
        &self.0[kind as usize]
    }
    pub fn get_mut(&mut self, kind: DefinitionKind) -> &mut Definition {
        &mut self.0[kind as usize]
    }
}
impl Commands {
    pub fn enter_definition(
        &mut self,
        kind: DefinitionKind,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        let selected: Vec<usize> = if matches!(kind, DefinitionKind::Light | DefinitionKind::Sequence) {
            self.positional(tokens, 1, locale)?;
            self.scratch.clone()
        } else {
            if tokens.len() < 2 {
                return Err(Error::Parse("No name specified!".into()));
            }
            (1..tokens.len()).collect()
        };
        let definition = self.definitions.get_mut(kind);
        definition.name.clone_from(&tokens[selected[0]].bytes);
        if kind == DefinitionKind::Sequence {
            let new = object(host.call(Call::AllocateSequence)?)?;
            if new != definition.object {
                if let Some(new) = new {
                    host.call(Call::Retain(new))?;
                }
                let old = definition.object;
                definition.object = new;
                if let Some(old) = old {
                    host.call(Call::Release(old))?;
                }
            }
            if new.is_some() {
                definition.description = Description::reset(kind, &definition.description);
                definition.sequence_elements.clear();
            }
            for (key, flag) in [(b"loop".as_slice(), 1), (b"noOverlap", 2), (b"hardStart", 4)] {
                if self.switch(tokens, key, 0, locale)? {
                    let definition = self.definitions.get_mut(kind);
                    if definition.object.is_none() {
                        return Err(Error::Unsupported("sequence allocation failure dereference"));
                    }
                    *definition.description.words.get_mut(&0x18).unwrap() |= flag;
                }
            }
            return Ok(());
        }
        definition.description = Description::reset(kind, &definition.description);
        if selected.len() > 1 && tokens[selected[1]].bytes != b":" {
            return Err(Error::Parse("Expecting ':'".into()));
        }
        if selected.len() > 2 {
            if let Some(collection) = self.collection {
                let parent = tokens[selected[2]].bytes.clone();
                let reply = host.call(Call::Inherit(
                    collection,
                    kind,
                    parent.clone(),
                    definition.description.clone(),
                ))?;
                let (success, output) = match reply {
                    Reply::Inherited(s, d) => (s, d),
                    _ => return Err(Error::Unsupported("inherit reply")),
                };
                definition.description = output;
                if !success {
                    let label = match kind {
                        DefinitionKind::Shake => "shake",
                        DefinitionKind::Light => "light",
                        _ => "particle",
                    };
                    return Err(Error::Parse(format!(
                        "Unknown {label} definition: {}",
                        String::from_utf8_lossy(&parent)
                    )));
                }
            }
        }
        if kind == DefinitionKind::Particles {
            self.word_494 = definition
                .description
                .lists
                .get(&0x1b8)
                .and_then(|v| v.last())
                .copied()
                .unwrap_or(0);
            self.scalar_498 = 1.;
        }
        Ok(())
    }
    pub fn exit_definition(&mut self, kind: DefinitionKind, host: &mut impl Services) -> Result<()> {
        let definition = self.definitions.get_mut(kind);
        if kind == DefinitionKind::Light {
            let list = definition.description.floats.entry(0x0c).or_default();
            if list.is_empty() {
                list.push(1.);
            }
            let list = definition.description.vectors.entry(0).or_default();
            if list.is_empty() {
                list.push([0., 0., 1.]);
            }
        }
        if let Some(collection) = self.collection {
            if kind == DefinitionKind::Sequence {
                host.call(Call::PublishSequence(
                    collection,
                    definition.name.clone(),
                    definition.object,
                ))?;
            } else {
                host.call(Call::PublishDefinition(
                    collection,
                    kind,
                    definition.name.clone(),
                    definition.description.clone(),
                ))?;
            }
        }
        definition.name.clear();
        if kind == DefinitionKind::Sequence {
            if let Some(old) = definition.object.take() {
                host.call(Call::Release(old))?;
            }
        }
        Ok(())
    }
    pub fn effect_base(&mut self, tokens: &[ScriptToken], locale: &mut impl CharacterRules) -> Result<()> {
        if self.definitions.get(DefinitionKind::Particles).name.is_empty() {
            return Err(Error::Parse("Not in particles block".into()));
        }
        self.positional(tokens, 1, locale)?;
        let text = self.value(tokens, 0).to_vec();
        self.definitions
            .get_mut(DefinitionKind::Particles)
            .description
            .strings
            .insert(0x160, text);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
