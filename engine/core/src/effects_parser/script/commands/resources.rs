// SPDX-License-Identifier: GPL-3.0-or-later
//! Decal children and particle texture/model/timed-effect (§§19.35–42).
use super::*;
use definitions::DefinitionKind::{Decal, Particles};
use particle::{bounded, color, normalize_suffix, scale255, Appearance};

pub const DECAL_DRAW: &[(&str, u32)] = &[
    ("decal", 0),
    ("additive", 1),
    ("modulate", 2),
    ("decalInvertDepth", 3),
    ("decalNoOverlap", 4),
];
pub const PARTICLE_DRAW: &[(&str, u32)] = &[
    ("decal", 0),
    ("decalInvertDepth", 1),
    ("decalIgnoreDepth", 2),
    ("depthDecal", 3),
    ("depthDecalMasked", 4),
    ("additive", 5),
    ("additiveIgnoreDepth", 6),
    ("modulate", 7),
];

fn enum_value(host: &mut impl Services, text: Vec<u8>, table: &'static [(&'static str, u32)]) -> Result<u8> {
    match host.call(Call::Enum(text, table))? {
        Reply::Uint(n) => Ok(n as u8),
        _ => Err(Error::Unsupported("enum reply")),
    }
}
pub(super) fn vector2(host: &mut impl Services, text: Vec<u8>) -> Result<[f32; 2]> {
    match host.call(Call::Vector2(text))? {
        Reply::Vector2(v) => Ok(v),
        _ => Err(Error::Unsupported("vector2 reply")),
    }
}
impl Commands {
    fn decal_desc(&mut self) -> &mut Description {
        &mut self.definitions.get_mut(Decal).description
    }
    pub fn decal_appearance(
        &mut self,
        kind: Appearance,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        let args = self.selected_bytes(tokens)?;
        let (at, vary) = match kind {
            Appearance::Color => (0x34, None),
            Appearance::Alpha => (0x28, Some(0x4c)),
            Appearance::Size => (0x1c, Some(0x50)),
            Appearance::Aspect => (0x40, None),
            Appearance::Rotate => (0x10, Some(0x54)),
        };
        if kind == Appearance::Color {
            self.decal_desc().vectors.entry(at).or_default().clear();
        } else {
            self.decal_desc().floats.entry(at).or_default().clear();
        }
        for arg in args {
            if kind == Appearance::Color {
                let v = color(host, &arg)?;
                self.decal_desc().vectors.get_mut(&at).unwrap().push(v);
            } else {
                let x = float(host, &arg)?;
                self.decal_desc().floats.get_mut(&at).unwrap().push(x);
            }
        }
        if let Some(vary) = vary {
            if self.switch(tokens, b"vary", 1, locale)? {
                let x = bounded(host, &self.first_bytes(tokens)?, 0., 1.)?;
                self.decal_desc().scalar(vary, x as f32);
            }
        }
        if kind == Appearance::Size && self.switch(tokens, b"cityScale", 0, locale)? {
            *self.decal_desc().words.entry(0).or_default() |= 0x10;
        }
        if matches!(kind, Appearance::Color | Appearance::Alpha) && normalize_suffix(&tokens[0].bytes)? {
            if kind == Appearance::Color {
                for v in self.decal_desc().vectors.get_mut(&at).unwrap() {
                    for x in v {
                        *x = scale255(*x);
                    }
                }
            } else {
                for x in self.decal_desc().floats.get_mut(&at).unwrap() {
                    *x = scale255(*x);
                }
            }
        }
        Ok(())
    }
    pub fn decal_life(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 0, locale)?;
        if !self.scratch.is_empty() {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.decal_desc().scalar(0xc, x);
        }
        for (key, mode) in [(b"static".as_slice(), 1), (b"loop", 1), (b"single", 2), (b"sustain", 3)] {
            if self.switch(tokens, key, 0, locale)? {
                if key == b"static" {
                    *self.decal_desc().words.entry(0).or_default() |= 0x40;
                }
                self.decal_desc().bytes.insert(9, mode);
            }
        }
        Ok(())
    }
    pub fn decal_texture_options(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if self.switch(tokens, b"draw", 1, locale)? {
            let n = enum_value(host, self.first_bytes(tokens)?, DECAL_DRAW)?;
            self.decal_desc().bytes.insert(8, n);
        }
        for (key, bit) in [(b"light".as_slice(), 2), (b"water", 4), (b"ring", 0x20)] {
            if self.switch(tokens, key, 0, locale)? {
                *self.decal_desc().words.entry(0).or_default() |= bit;
            }
        }
        if self.switch(tokens, b"repeat", 1, locale)? {
            *self.decal_desc().words.entry(0).or_default() |= 8;
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.decal_desc().scalar(0x58, x);
        }
        if self.switch(tokens, b"offset", 1, locale)? {
            let v = vector2(host, self.first_bytes(tokens)?)?;
            self.decal_desc().scalar(0x5c, v[0]);
            self.decal_desc().scalar(0x60, v[1]);
        }
        Ok(())
    }
    pub fn decal_texture(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.decal_texture_options(tokens, locale, host)?;
        self.select_texture(Decal, 4, tokens, locale)
    }
    /// Selected indices and full signed EAX are independent primitive inputs;
    /// callers can test nonpositive registers without manufacturing names.
    pub fn texture_selected(
        &mut self,
        kind: DefinitionKind,
        at: u32,
        tokens: &[ScriptToken],
        full_eax: i32,
        selected: &[usize],
        locale: &mut impl CharacterRules,
    ) -> Result<()> {
        if full_eax <= 0 {
            return Ok(());
        }
        self.scratch = selected.to_vec();
        let name = self.first_bytes(tokens)?;
        let key = fold(&name, locale);
        let id = self.id_maps[0]
            .get(&key)
            .copied()
            .ok_or_else(|| Error::Parse(format!("No such texture: '{}'", String::from_utf8_lossy(&name))))?;
        self.definitions.get_mut(kind).description.words.insert(at, id);
        Ok(())
    }
    fn select_texture(
        &mut self,
        kind: DefinitionKind,
        at: u32,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
    ) -> Result<()> {
        let min = usize::from(self.definitions.get(kind).description.words.get(&at) == Some(&0));
        self.positional(tokens, min, locale)?;
        let selected = self.scratch.clone();
        self.texture_selected(
            kind,
            at,
            tokens,
            if selected.is_empty() { 0 } else { 1 },
            &selected,
            locale,
        )
    }
    pub fn particle_draw(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if self.switch(tokens, b"draw", 1, locale)? {
            let n = enum_value(host, self.first_bytes(tokens)?, PARTICLE_DRAW)?;
            self.particle_desc().bytes.insert(0xd4, n);
        }
        for (key, bit) in [(b"light".as_slice(), 1), (b"noCull", 0x10)] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        if self.switch(tokens, b"sortOffset", 1, locale)? {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.p_set(0xd8, x);
        }
        Ok(())
    }
    pub fn particle_texture(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.particle_draw(tokens, locale, host)?;
        for (key, bit) in [(b"vflip".as_slice(), 0x40000000), (b"hflip", 0x20000000)] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        self.select_texture(Particles, 0xd0, tokens, locale)
    }
    pub fn particle_model_options(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.require_particles()?;
        self.p_or(8, 1);
        self.particle_desc().bytes.insert(0xd4, 3);
        self.particle_draw(tokens, locale, host)?;
        for (key, bit) in [(b"fakePerspective".as_slice(), 2), (b"moveEntireSlave", 8)] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(8, bit);
            }
        }
        if self.switch(tokens, b"slaveApplyAlpha", 0, locale)? || self.switch(tokens, b"applyAlpha", 0, locale)? {
            self.p_or(8, 0x10);
        }
        for (key, bit) in [(b"modelSpeed".as_slice(), 0x40), (b"modelSpeedStatic", 0)] {
            if self.switch(tokens, key, 1, locale)? {
                let x = float(host, &self.first_bytes(tokens)?)?;
                self.p_set(0x1dc, x);
                self.p_or(8, bit);
            }
        }
        for (key, bit) in [
            (b"sustain".as_slice(), 0x20),
            (b"applyLighting", 4),
            (b"noCullFaces", 0x400),
        ] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(8, bit);
            }
        }
        Ok(())
    }
    pub fn particle_model_selected(
        &mut self,
        tokens: &[ScriptToken],
        full_eax: i32,
        selected: &[usize],
        locale: &mut impl CharacterRules,
    ) -> Result<()> {
        if full_eax <= 0 {
            return Ok(());
        }
        self.scratch = selected.to_vec();
        let mut ids = Vec::new();
        for name in self.selected_bytes(tokens)? {
            let key = fold(&name, locale);
            let id = self.id_maps[1]
                .get(&key)
                .copied()
                .ok_or_else(|| Error::Parse(format!("No such model: '{}'", String::from_utf8_lossy(&name))))?;
            ids.push(id);
        }
        if ids.len() == 1 {
            self.particle_desc().words.insert(0xd0, ids[0]);
            self.particle_desc().lists.entry(0x1e0).or_default().clear();
        } else {
            self.particle_desc().lists.insert(0x1e0, ids);
        }
        Ok(())
    }
    pub fn particle_model(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.particle_model_options(tokens, locale, host)?;
        self.positional(tokens, 0, locale)?;
        let selected = self.scratch.clone();
        self.particle_model_selected(tokens, if selected.is_empty() { 0 } else { 1 }, &selected, locale)
    }
    pub fn particle_timed(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.p_or(0, 0x80000000);
        self.positional(tokens, 2, locale)?;
        let args = self.selected_bytes(tokens)?;
        let time = float(host, &args[1])?;
        let list = &mut self.particle_desc().timed_effects;
        let at = list.iter().position(|(_, old)| *old > time).unwrap_or(list.len());
        list.insert(at, (args[0].clone(), time));
        Ok(())
    }
}
#[cfg(test)]
mod tests;
