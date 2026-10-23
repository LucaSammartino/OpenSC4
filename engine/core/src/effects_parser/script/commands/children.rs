// SPDX-License-Identifier: GPL-3.0-or-later
//! Dynamic particle, sequence, shake and light child owners (§§19.27–34).
use super::*;
use definitions::DefinitionKind::{DynamicParticle, Light, Sequence, Shake};

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceElement {
    pub first: f32,
    pub second: f32,
    pub name: Vec<u8>,
}
pub const SHAKE_TABLE: &[(&str, u32)] = &[("random", 0), ("sineY", 1)];

impl Commands {
    fn require_dynamic(&self) -> Result<()> {
        if self.definitions.get(DynamicParticle).name.is_empty() {
            Err(Error::Parse("Not in dynamic particle block".into()))
        } else {
            Ok(())
        }
    }
    pub fn dynamic_base(&mut self, tokens: &[ScriptToken], locale: &mut impl CharacterRules) -> Result<()> {
        self.require_dynamic()?;
        self.positional(tokens, 1, locale)?;
        let text = self.value(tokens, 0).to_vec();
        self.definitions
            .get_mut(DynamicParticle)
            .description
            .strings
            .insert(0x0c, text);
        Ok(())
    }
    pub fn dynamic_mass(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.require_dynamic()?;
        self.positional(tokens, 1, locale)?;
        let mass = float(host, self.value(tokens, 0))?;
        self.definitions.get_mut(DynamicParticle).description.scalar(0x18, mass);
        Ok(())
    }
    pub fn dynamic_friction(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.require_dynamic()?;
        self.positional(tokens, 2, locale)?;
        for (i, at) in [(0, 0x20), (1, 0x24)] {
            let value = float(host, self.value(tokens, i))?;
            self.definitions.get_mut(DynamicParticle).description.scalar(at, value);
        }
        if self.switch(tokens, b"angular", 1, locale)? {
            let value = float(host, self.value(tokens, 0))?;
            self.definitions
                .get_mut(DynamicParticle)
                .description
                .scalar(0x28, value);
        }
        Ok(())
    }
    /// Full EAX and selected indices are separate primitive inputs, as in EC126
    /// and EC130. Normal script readers supply the initial positional prefix;
    /// no bool-to-full-register value is inferred here.
    pub fn dynamic_model_selected(
        &mut self,
        tokens: &[ScriptToken],
        full_eax: i32,
        selected: &[usize],
        locale: &mut impl CharacterRules,
    ) -> Result<()> {
        self.require_dynamic()?;
        if full_eax <= 0 {
            return Ok(());
        }
        let mut ids = Vec::new();
        for &index in selected {
            let name = &tokens
                .get(index)
                .ok_or(Error::Unsupported("invalid primitive index"))?
                .bytes;
            let key = fold(name, locale);
            let id = self.id_maps[1]
                .get(&key)
                .copied()
                .ok_or_else(|| Error::Parse(format!("No such model: '{}'", String::from_utf8_lossy(name))))?;
            ids.push(id);
        }
        let description = &mut self.definitions.get_mut(DynamicParticle).description;
        if ids.len() == 1 {
            description.words.insert(0x30, ids[0]);
            description.lists.entry(0x34).or_default().clear();
        } else {
            description.lists.insert(0x34, ids);
        }
        Ok(())
    }
    pub fn sequence_element(
        &mut self,
        play: bool,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, if play { 2 } else { 1 }, locale)?;
        let name = if play {
            self.value(tokens, 0).to_vec()
        } else {
            Vec::new()
        };
        let at = usize::from(play);
        let first = float(host, self.value(tokens, at))?;
        let second = if self.scratch.len() > at + 1 {
            float(host, self.value(tokens, at + 1))?
        } else {
            first
        };
        let definition = self.definitions.get_mut(Sequence);
        if definition.object.is_none() {
            return Err(Error::Unsupported("installed sequence required"));
        }
        definition
            .sequence_elements
            .push(SequenceElement { first, second, name });
        Ok(())
    }
    fn raw_operand(tokens: &[ScriptToken]) -> Result<&[u8]> {
        tokens
            .get(1)
            .map(|t| t.bytes.as_slice())
            .ok_or_else(|| Error::Parse("Not enough arguments".into()))
    }
    pub fn shake_aspect(&mut self, tokens: &[ScriptToken], host: &mut impl Services) -> Result<()> {
        let value = float(host, Self::raw_operand(tokens)?)?;
        self.definitions.get_mut(Shake).description.scalar(0x20, value);
        Ok(())
    }
    pub fn shake_table(&mut self, tokens: &[ScriptToken], host: &mut impl Services) -> Result<()> {
        let value = match host.call(Call::Enum(Self::raw_operand(tokens)?.to_vec(), SHAKE_TABLE))? {
            Reply::Uint(v) => v,
            _ => return Err(Error::Unsupported("enum reply")),
        };
        self.definitions.get_mut(Shake).description.words.insert(0x24, value);
        Ok(())
    }
    pub fn shake_light_length(
        &mut self,
        light: bool,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        let value = if light {
            self.positional(tokens, 1, locale)?;
            float(host, self.value(tokens, 0))?
        } else {
            float(host, Self::raw_operand(tokens)?)?
        };
        self.definitions
            .get_mut(if light { Light } else { Shake })
            .description
            .scalar(if light { 0x18 } else { 0 }, value);
        if self.switch(tokens, b"fade", 1, locale)? {
            let fade = float(host, self.value(tokens, 0))?;
            self.definitions.get_mut(Shake).description.scalar(4, fade);
        }
        let d = &mut self.definitions.get_mut(Shake).description;
        // Unassigned fields are outside this command's valid initialized
        // description domain, rather than silently synthesized defaults.
        let length = d
            .get_scalar(0)
            .ok_or(Error::Unsupported("initialized shake length required"))?;
        let fade = d
            .get_scalar(4)
            .ok_or(Error::Unsupported("initialized shake fade required"))?;
        if fade > length {
            d.words.insert(4, length.to_bits());
        }
        Ok(())
    }
    pub fn shake_light_curve(
        &mut self,
        kind: DefinitionKind,
        at: u32,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        self.definitions
            .get_mut(kind)
            .description
            .floats
            .entry(at)
            .or_default()
            .clear();
        for i in 0..self.scratch.len() {
            let value = float(host, self.value(tokens, i))?;
            self.definitions
                .get_mut(kind)
                .description
                .floats
                .get_mut(&at)
                .unwrap()
                .push(value);
        }
        Ok(())
    }
    pub fn light_color(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        self.definitions
            .get_mut(Light)
            .description
            .vectors
            .entry(0)
            .or_default()
            .clear();
        for i in 0..self.scratch.len() {
            let color = match host.call(Call::Color(self.value(tokens, i).to_vec()))? {
                Reply::Vector3(v) => v,
                _ => return Err(Error::Unsupported("color reply")),
            };
            self.definitions
                .get_mut(Light)
                .description
                .vectors
                .get_mut(&0)
                .unwrap()
                .push(color);
        }
        let command = &tokens[0].bytes;
        if command.len() < 3 {
            return Err(Error::Foreign("substring range".into()));
        }
        if &command[command.len() - 3..] == b"255" {
            let scale = f64::from(f32::from_bits(0x3b808081));
            for vector in self.definitions.get_mut(Light).description.vectors.get_mut(&0).unwrap() {
                for value in vector {
                    *value = (f64::from(*value) * scale) as f32;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
