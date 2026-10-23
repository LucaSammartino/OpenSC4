// SPDX-License-Identifier: GPL-3.0-or-later
//! Particle child owners, effects §§19.18–26. All offsets are relative to D.
use super::*;
use definitions::DefinitionKind::Particles;

pub const ALIGN: &[(&str, u32)] = &[("camera", 0), ("ground", 1), ("dirX", 2), ("dirY", 3), ("dirZ", 4)];

pub(super) fn bounded(host: &mut impl Services, text: &[u8], lo: f64, hi: f64) -> Result<f64> {
    let x = raw_float(host, text)?;
    // Literal ordered comparisons: unordered primitive replies are accepted.
    if x < lo || x > hi {
        return Err(Error::Parse(format!(
            "argument out of range: expecting [{lo}..{hi}], got {x}"
        )));
    }
    Ok(x)
}
pub(super) fn color(host: &mut impl Services, text: &[u8]) -> Result<[f32; 3]> {
    match host.call(Call::Color(text.to_vec()))? {
        Reply::Vector3(v) => Ok(v),
        _ => Err(Error::Unsupported("color reply")),
    }
}
pub(super) fn normalize_suffix(command: &[u8]) -> Result<bool> {
    if command.len() < 3 {
        return Err(Error::Foreign("substring range".into()));
    }
    Ok(&command[command.len() - 3..] == b"255")
}
pub(super) fn scale255(x: f32) -> f32 {
    (f64::from(x) * f64::from(f32::from_bits(0x3b808081))) as f32
}

impl Commands {
    pub(super) fn particle_desc(&mut self) -> &mut Description {
        &mut self.definitions.get_mut(Particles).description
    }
    pub(super) fn require_particles(&self) -> Result<()> {
        if self.definitions.get(Particles).name.is_empty() {
            Err(Error::Parse("Not in particles block".into()))
        } else {
            Ok(())
        }
    }
    pub(super) fn p_or(&mut self, at: u32, bits: u32) {
        *self.particle_desc().words.entry(at).or_default() |= bits;
    }
    pub(super) fn p_set(&mut self, at: u32, value: f32) {
        self.particle_desc().scalar(at, value);
    }
    pub(super) fn selected_bytes(&self, tokens: &[ScriptToken]) -> Result<Vec<Vec<u8>>> {
        self.scratch
            .iter()
            .map(|&i| {
                tokens
                    .get(i)
                    .map(|t| t.bytes.clone())
                    .ok_or(Error::Unsupported("invalid primitive index"))
            })
            .collect()
    }
    pub(super) fn first_bytes(&self, tokens: &[ScriptToken]) -> Result<Vec<u8>> {
        self.selected_bytes(tokens)?
            .into_iter()
            .next()
            .ok_or(Error::Unsupported("unchecked empty selection"))
    }
    pub(super) fn pair(&self, tokens: &[ScriptToken], midpoint: bool, host: &mut impl Services) -> Result<[f32; 2]> {
        let args = self.selected_bytes(tokens)?;
        let a = float(
            host,
            args.first().ok_or(Error::Unsupported("unchecked empty selection"))?,
        )?;
        if midpoint {
            let r = if args.len() > 1 { raw_float(host, &args[1])? } else { 0. };
            Ok([(f64::from(a) - r) as f32, (r + f64::from(a)) as f32])
        } else {
            let b = if args.len() > 1 { float(host, &args[1])? } else { a };
            Ok([a, b])
        }
    }
    pub fn particle_zoom_stretch(
        &mut self,
        stretch: bool,
        tokens: &[ScriptToken],
        host: &mut impl Services,
    ) -> Result<()> {
        if tokens.len() < 2 {
            return Err(Error::Parse("Not enough arguments".into()));
        }
        if stretch {
            let x = float(host, &tokens[1].bytes)?;
            self.p_set(0xdc, x);
        } else {
            uint(host, &tokens[1].bytes)?;
        }
        Ok(())
    }
    pub fn particle_life(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        let m = float(host, &self.first_bytes(tokens)?)?;
        let r = if self.scratch.len() > 1 {
            raw_float(host, self.value(tokens, 1))?
        } else {
            0.
        };
        self.p_set(0x0c, (f64::from(m) - r) as f32);
        self.p_set(0x10, (f64::from(m) + r) as f32);
        let preroll = if self.switch(tokens, b"preroll", 1, locale)? {
            float(host, &self.first_bytes(tokens)?)?
        } else {
            m
        };
        self.p_set(0x1c, preroll);
        Ok(())
    }
    /// `mode`: 0 = rate, 2 = inject, 4 = maintain. Selection is concrete;
    /// full-register fixtures use particle_rate_selected instead.
    pub fn particle_rate(
        &mut self,
        mode: u32,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        let inject = mode == 0 && self.switch(tokens, b"inject", 1, locale)?;
        if !inject {
            self.positional(tokens, 1, locale)?;
        }
        let selected = self.scratch.clone();
        self.particle_rate_selected(if inject { 2 } else { mode }, tokens, 1, &selected, locale, host)
    }
    /// Full EAX is independent from selected indices (§19.19), never synthesized
    /// from AL. Other modes ignore that primitive return.
    pub fn particle_rate_selected(
        &mut self,
        mode: u32,
        tokens: &mut [ScriptToken],
        full_eax: i32,
        selected: &[usize],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.scratch = selected.to_vec();
        let args = self.selected_bytes(tokens)?;
        if mode == 0 {
            if full_eax != 0 {
                self.particle_desc().floats.entry(0x88).or_default().clear();
                for arg in args {
                    let x = float(host, &arg)?;
                    self.particle_desc().floats.get_mut(&0x88).unwrap().push(x);
                }
            }
        } else {
            self.particle_desc().floats.entry(0x88).or_default().resize(1, 0.);
            let x = float(
                host,
                args.first().ok_or(Error::Unsupported("unchecked empty selection"))?,
            )?;
            self.particle_desc().floats.get_mut(&0x88).unwrap()[0] = x;
            self.p_or(0, mode);
        }
        if mode == 4 {
            if self.switch(tokens, b"delay", 1, locale)? {
                let [a, b] = self.pair(tokens, false, host)?;
                self.p_set(0x20, a);
                self.p_set(0x24, b);
            }
            return Ok(());
        }
        self.particle_rate_options(tokens, locale, host)
    }
    pub(super) fn particle_rate_options(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if self.switch(tokens, b"loop", 1, locale)? {
            let args = self.selected_bytes(tokens)?;
            let x = float(host, &args[0])?;
            self.p_set(0x14, x);
            if args.len() > 1 {
                let n = uint(host, &args[1])?;
                self.particle_desc().words.insert(0x18, n);
            }
        }
        if self.switch(tokens, b"single", 0, locale)? {
            let args = self.selected_bytes(tokens)?;
            let x = if args.is_empty() {
                f32::from_bits(0x3a83126f)
            } else {
                float(host, &args[0])?
            };
            self.p_set(0x14, x);
            self.particle_desc().words.insert(0x18, 1);
        }
        for (key, bit) in [
            (b"sustain".as_slice(), 8),
            (b"scale", 0x80000),
            (b"areaScale", 0x100000),
            (b"volumeScale", 0x200000),
        ] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        for (key, at) in [(b"delay".as_slice(), 0x20), (b"trigger", 0x28)] {
            let found = self.switch(tokens, key, 1, locale)?;
            let found = found || (at == 0x28 && self.switch(tokens, b"retrigger", 1, locale)?);
            if found {
                let [a, b] = self.pair(tokens, false, host)?;
                self.p_set(at, a);
                self.p_set(at + 4, b);
            }
        }
        Ok(())
    }
    pub fn particle_align(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.require_particles()?;
        if tokens.len() < 2 {
            return Err(Error::Parse("Not enough arguments".into()));
        }
        let n = match host.call(Call::Enum(tokens[1].bytes.clone(), ALIGN))? {
            Reply::Uint(n) => n,
            _ => return Err(Error::Unsupported("enum reply")),
        };
        self.particle_desc().bytes.insert(0xd5, n as u8);
        if self.switch(tokens, b"damp", 1, locale)? {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.p_set(0x198, x);
        }
        let mut bank = self.switch(tokens, b"bank", 2, locale)?;
        if !bank && self.switch(tokens, b"windBank", 2, locale)? {
            self.p_or(8, 0x80);
            bank = true;
        }
        if bank {
            let args = self.selected_bytes(tokens)?;
            for (arg, at) in args.iter().take(2).zip([0x19c, 0x1a0]) {
                let x = float(host, arg)?;
                self.p_set(at, x);
            }
        }
        Ok(())
    }
    pub fn particle_collision(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.require_particles()?;
        self.p_or(0, 0x100);
        if self.switch(tokens, b"bounce", 1, locale)? {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.p_set(0x138, x);
        } else if self.particle_desc().get_scalar(0x138) == Some(0.) {
            self.p_set(0x138, f32::from_bits(0x3e99999a));
        }
        for (key, bit) in [(b"sticky".as_slice(), 0x200), (b"killOutsideCity", 0x800)] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        if self.switch(tokens, b"effect", 0, locale)? {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.p_set(0x150, x);
        }
        for (key, at) in [(b"death".as_slice(), 0x150), (b"deathByWater", 0x154)] {
            if self.switch(tokens, key, 1, locale)? {
                let x = bounded(host, &self.first_bytes(tokens)?, 0., 1.)?;
                self.p_set(at, x as f32);
            }
        }
        if self.switch(tokens, b"destroyBuildings", 0, locale)? {
            self.p_or(4, 1);
        }
        Ok(())
    }
    pub fn particle_terrain(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.p_or(0, 0x400);
        self.positional(tokens, 2, locale)?;
        let args = self.selected_bytes(tokens)?;
        for (arg, at) in args.iter().take(2).zip([0x13c, 0x140]) {
            let x = float(host, arg)?;
            self.p_set(at, x);
        }
        for (key, at) in [
            (b"scout".as_slice(), 0x144),
            (b"vertical", 0x148),
            (b"killHeight", 0x14c),
        ] {
            if self.switch(tokens, key, 1, locale)? {
                let x = float(host, &self.first_bytes(tokens)?)?;
                self.p_set(at, x);
            }
        }
        Ok(())
    }
    pub fn particle_walk(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.p_or(0, 0x800000);
        for (key, at) in [(b"delay".as_slice(), 0x174), (b"strength", 0x17c)] {
            if self.switch(tokens, key, 1, locale)? {
                let [a, b] = self.pair(tokens, true, host)?;
                self.p_set(at, a);
                self.p_set(at + 4, b);
            }
        }
        if self.switch(tokens, b"turn", 1, locale)? {
            let args = self.selected_bytes(tokens)?;
            let a = bounded(host, &args[0], 0., 1.)?;
            self.p_set(0x184, a as f32);
            let b = if args.len() > 1 {
                bounded(host, &args[1], 0., 1.)?
            } else {
                0.
            };
            self.p_set(0x188, b as f32);
        }
        for (key, bit) in [(b"wait".as_slice(), 0x1000000), (b"preferSea", 0x2000000)] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        if self.switch(tokens, b"preferDir", 0, locale)? {
            let v = vector(host, &self.first_bytes(tokens)?)?;
            for (at, x) in [0x18c, 0x190, 0x194].into_iter().zip(v) {
                self.p_set(at, x);
            }
        }
        Ok(())
    }
    pub fn particle_appearance(
        &mut self,
        kind: Appearance,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        let args = self.selected_bytes(tokens)?;
        let (at, vary) = match kind {
            Appearance::Color => (0x94, 0x7c),
            Appearance::Alpha => (0xa0, 0x78),
            Appearance::Size => (0xac, 0x68),
            Appearance::Aspect => (0xb8, 0x6c),
            Appearance::Rotate => (0xc4, 0x70),
        };
        if kind == Appearance::Color {
            self.particle_desc().vectors.entry(at).or_default().clear();
        } else {
            self.particle_desc().floats.entry(at).or_default().clear();
        }
        for arg in args {
            if kind == Appearance::Color {
                let v = color(host, &arg)?;
                self.particle_desc().vectors.get_mut(&at).unwrap().push(v);
            } else {
                let x = raw_float(host, &arg)?;
                let x = if kind == Appearance::Size { x * 50. } else { x };
                self.particle_desc().floats.get_mut(&at).unwrap().push(x as f32);
            }
        }
        if self.switch(tokens, b"vary", 1, locale)? {
            let arg = self.first_bytes(tokens)?;
            if kind == Appearance::Color {
                let v = vector(host, &arg)?;
                for x in v {
                    if x < 0. || x > 1. {
                        return Err(Error::Parse(format!(
                            "argument out of range: expecting components [0..1], got [{} {} {}]",
                            v[0], v[1], v[2]
                        )));
                    }
                }
                for (i, x) in v.into_iter().enumerate() {
                    self.p_set(vary + 4 * i as u32, x);
                }
            } else {
                let x = bounded(host, &arg, 0., 1.)?;
                self.p_set(vary, x as f32);
            }
        }
        if kind == Appearance::Rotate && self.switch(tokens, b"offset", 1, locale)? {
            let x = float(host, &self.first_bytes(tokens)?)?;
            self.p_set(0x74, x);
        }
        if matches!(kind, Appearance::Color | Appearance::Alpha) && normalize_suffix(&tokens[0].bytes)? {
            if kind == Appearance::Color {
                for v in self.particle_desc().vectors.get_mut(&at).unwrap() {
                    for x in v {
                        *x = scale255(*x);
                    }
                }
            } else {
                for x in self.particle_desc().floats.get_mut(&at).unwrap() {
                    *x = scale255(*x);
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Color,
    Alpha,
    Size,
    Aspect,
    Rotate,
}

#[cfg(test)]
mod tests;
