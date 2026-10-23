// SPDX-License-Identifier: GPL-3.0-or-later
//! Nested command owners, effects §§19.53–56. No generic stack mutations here.
use super::*;
use crate::effects_parser::{
    numeric::{self, Extended},
    rotation, NumericPolicy, RawFloat, Record,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub name: Vec<u8>,
    pub record: Record,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Auxiliary {
    pub flags: u32,
    pub name: Vec<u8>,
    pub value: u32,
    pub index: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    Particle,
    Dynamic,
    Decal,
    Visual,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuxiliaryKind {
    Shake,
    Flash,
    Tint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sound,
    Brush,
    Scrubber,
    Automata,
    Camera,
}

fn numeric_error(_: crate::effects_parser::Unsupported) -> Error {
    Error::Unsupported("unqualified numeric primitive/control state")
}
fn raw(host: &mut impl Services, bytes: &[u8]) -> Result<RawFloat> {
    match host.call(Call::Float(bytes.to_vec()))? {
        Reply::RawFloat(x) => Ok(x),
        Reply::Float(x) => Ok(RawFloat::Extended(Extended::binary64(x.to_bits()))),
        Reply::Float32(x) => Ok(RawFloat::Binary32(x)),
        _ => Err(Error::Unsupported("float reply")),
    }
}
fn extended(host: &mut impl Services, bytes: &[u8]) -> Result<Extended> {
    Extended::input(raw(host, bytes)?).map_err(numeric_error)
}
fn stored(host: &mut impl Services, bytes: &[u8], policy: NumericPolicy) -> Result<u32> {
    Ok(extended(host, bytes)?.store(numeric::control(policy).map_err(numeric_error)?))
}
fn range(value: i32, lo: i32, hi: i32) -> Result<u32> {
    if (lo..=hi).contains(&value) {
        Ok(value as u32)
    } else {
        Err(Error::Parse(format!(
            "argument out of range: expecting [{lo}..{hi}], got {value}"
        )))
    }
}
fn fresh() -> Record {
    Record::shared_constructor_projection()
}

impl Commands {
    fn nested_context(&self) -> Result<()> {
        if self.name.is_empty() {
            Err(Error::Parse("Not in effects block".into()))
        } else {
            Ok(())
        }
    }
    fn selected<'a>(&self, tokens: &'a [ScriptToken], index: usize) -> Result<&'a [u8]> {
        self.scratch
            .get(index)
            .and_then(|&i| tokens.get(i))
            .map(|t| t.bytes.as_slice())
            .ok_or(Error::Unsupported("unchecked empty selection"))
    }
    fn validate_nested(&self, slot: u32, name: &[u8], error: &str, host: &mut impl Services) -> Result<()> {
        if let Some(collection) = self.collection {
            if !boolean(host.call(Call::ValidateNested(collection, slot, name.to_vec()))?)? {
                return Err(Error::Parse(format!("{error}{}", String::from_utf8_lossy(name))));
            }
        }
        Ok(())
    }
    /// Common options retain the same instruction ordering as parse_options,
    /// with concrete script readers and fallible individual primitive calls.
    fn component_options(
        &mut self,
        record: &mut Record,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
        policy: NumericPolicy,
    ) -> Result<()> {
        if self.switch(tokens, b"offset", 1, locale)? {
            let words = match host.call(Call::Vector3(self.selected(tokens, 0)?.to_vec()))? {
                Reply::Vector3(x) => x,
                _ => return Err(Error::Unsupported("vector reply")),
            };
            for (at, x) in [40, 44, 48].into_iter().zip(words) {
                record.word(at, x.to_bits());
            }
            record.component.placement[1] |= 1;
            record.increment();
        }
        for (key, axes) in [
            (b"rotateX".as_slice(), &[rotation::Axis::X][..]),
            (b"rotateY".as_slice(), &[rotation::Axis::Y][..]),
            (b"rotateZ".as_slice(), &[rotation::Axis::Z][..]),
            (
                b"rotateXYZ".as_slice(),
                &[rotation::Axis::X, rotation::Axis::Y, rotation::Axis::Z][..],
            ),
            (
                b"rotateZXY".as_slice(),
                &[rotation::Axis::Z, rotation::Axis::X, rotation::Axis::Y][..],
            ),
        ] {
            if self.switch(tokens, key, axes.len(), locale)? {
                for (i, &axis) in axes.iter().enumerate() {
                    let value = raw(host, self.selected(tokens, i)?)?;
                    let arithmetic = host
                        .rotation_arithmetic()
                        .ok_or(Error::Unsupported("qualified rotation arithmetic required"))?;
                    rotation::apply(record, axis, value, arithmetic).map_err(numeric_error)?;
                }
            }
        }
        if self.switch(tokens, b"scale", 1, locale)? {
            let value = extended(host, self.selected(tokens, 0)?)?;
            record.word(52, value.store(numeric::control(policy).map_err(numeric_error)?));
            record.component.placement[1] = (record.component.placement[1] & !2) | if value.is_one() { 0 } else { 2 };
            record.increment();
        }
        if self.switch(tokens, b"lodRange", 2, locale)? {
            if self.word_4a0 != 0 {
                return Err(Error::Parse("Can't specify LOD in a select".into()));
            }
            record.component.lod_min = uint(host, self.selected(tokens, 0)?)? as u8;
            record.component.lod_max = (uint(host, self.selected(tokens, 1)?)? as u8).wrapping_add(1);
        } else if self.switch(tokens, b"lod", 1, locale)? {
            if self.word_4a0 != 0 {
                return Err(Error::Parse("Can't specify LOD in a select".into()));
            }
            let x = uint(host, self.selected(tokens, 0)?)? as u8;
            record.component.lod_min = x;
            record.component.lod_max = x.wrapping_add(1);
        } else {
            record.component.lod_min = 1;
            record.component.lod_max = 6;
        }
        for (key, at) in [(b"emitScale".as_slice(), 0), (b"sizeScale".as_slice(), 2)] {
            if self.switch(tokens, key, 1, locale)? {
                let c = policy;
                let first = stored(host, self.selected(tokens, 0)?, c)?;
                record.component.render[at] = f32::from_bits(first);
                let second = if self.scratch.len() > 1 {
                    stored(host, self.selected(tokens, 1)?, c)?
                } else {
                    first
                };
                record.component.render[at + 1] = f32::from_bits(second);
            }
        }
        if self.switch(tokens, b"prob", 1, locale)? {
            if self.word_4a0 == 0 {
                return Err(Error::Parse("-prob only valid inside a select group".into()));
            }
            record.component.select_weight = numeric::probability(raw(host, self.selected(tokens, 0)?)?, policy)
                .map_err(numeric_error)?
                .2 as u16;
        }
        if self.switch(tokens, b"ignoreLength", 0, locale)? {
            record.component.flags |= 1;
        }
        if self.switch(tokens, b"respectLength", 0, locale)? {
            record.component.flags &= !1;
        }
        Ok(())
    }
    pub fn named_component(
        &mut self,
        kind: Named,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
        policy: NumericPolicy,
    ) -> Result<()> {
        self.nested_context()?;
        self.positional(tokens, 1, locale)?;
        let name = self.selected(tokens, 0)?.to_vec();
        let (slot, error, ty) = match kind {
            Named::Particle => (0x20, "Unknown particle definition: ", 0),
            Named::Dynamic => (0, "", 16),
            Named::Decal => (0x30, "Unknown decal definition: ", 1),
            Named::Visual => (0x18, "Unknown visual effect: ", 2),
        };
        if slot != 0 {
            self.validate_nested(slot, &name, error, host)?;
        }
        let mut record = fresh();
        record.set_type(ty);
        record.component.select_group = self.word_4a0 as u16;
        if matches!(kind, Named::Particle | Named::Dynamic) {
            record.component.flags = (record.component.flags & !2) | if self.byte_4ac != 0 { 2 } else { 0 };
        }
        self.component_options(&mut record, tokens, locale, host, policy)?;
        if kind == Named::Particle && self.switch(tokens, b"shells", 1, locale)? {
            record.component.copies = uint(host, self.selected(tokens, 0)?)? as u16;
            if self.scratch.len() > 1 {
                let c = numeric::control(policy).map_err(numeric_error)?;
                let x = extended(host, self.selected(tokens, 1)?)?;
                let t = x
                    .add(Extended::input(RawFloat::Dyadic(1, -1)).map_err(numeric_error)?, c)
                    .add(
                        Extended::input(RawFloat::Dyadic(12582912, 0)).map_err(numeric_error)?,
                        c,
                    )
                    .store(c);
                let j = t.wrapping_sub(0x4b400000);
                let previous = j.wrapping_sub(1) as i32;
                let equals = x.ordered_equal(Extended::input(RawFloat::Dyadic(previous, 0)).map_err(numeric_error)?);
                record.component.spacing = if equals { previous as u16 } else { j as u16 };
            }
        }
        self.visual.components.push(Component { name, record });
        Ok(())
    }
    pub fn auxiliary_component(
        &mut self,
        kind: AuxiliaryKind,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
        policy: NumericPolicy,
    ) -> Result<()> {
        self.nested_context()?;
        self.positional(tokens, 1, locale)?;
        let name = self.selected(tokens, 0)?.to_vec();
        self.validate_nested(
            if kind == AuxiliaryKind::Shake { 0x38 } else { 0x40 },
            &name,
            if kind == AuxiliaryKind::Shake {
                "unknown shake definition: "
            } else {
                "unknown light definition: "
            },
            host,
        )?;
        self.visual.auxiliary.push(Auxiliary {
            flags: match kind {
                AuxiliaryKind::Shake => 1,
                AuxiliaryKind::Flash => 4,
                AuxiliaryKind::Tint => 8,
            },
            name,
            value: 0,
            index: u32::MAX,
        });
        let i = self.visual.auxiliary.len() - 1;
        if kind == AuxiliaryKind::Shake {
            if !(self.switch(tokens, b"noEpicentre", 0, locale)? || self.switch(tokens, b"noEpicenter", 0, locale)?) {
                self.visual.auxiliary[i].flags |= 2;
            }
        } else if kind == AuxiliaryKind::Flash
            && (self.switch(tokens, b"epicentre", 0, locale)? || self.switch(tokens, b"epicenter", 0, locale)?)
        {
            self.visual.auxiliary[i].flags |= 2;
            self.visual.auxiliary[i].value = if self.scratch.is_empty() {
                0x447a0000
            } else {
                stored(host, self.selected(tokens, 0)?, policy)?
            };
        }
        Ok(())
    }
    pub fn chain_component(&mut self, tokens: &[ScriptToken], locale: &mut impl CharacterRules) -> Result<()> {
        self.nested_context()?;
        self.positional(tokens, 1, locale)?;
        self.visual.chain = self.selected(tokens, 0)?.to_vec();
        Ok(())
    }
    pub fn enter_select(&mut self, host: &mut impl Services) -> Result<()> {
        self.nested_context()?;
        self.word_4a4 = self.word_4a4.wrapping_add(1);
        self.word_4a0 = self.word_4a4;
        self.word_4a8 = self.visual.components.len() as u32;
        host.call(Call::InheritedBlock)?;
        Ok(())
    }
    pub fn exit_select(&mut self) {
        self.word_4a0 = 0;
        let range = self
            .visual
            .components
            .get_mut(self.word_4a8 as usize..)
            .unwrap_or(&mut []);
        let z = range.iter().filter(|r| r.record.component.select_weight == 0).count() as u32;
        let mut a = (range.len() as u32).wrapping_sub(z);
        if let Some(q) = 65536u32.wrapping_sub(a).checked_div(z) {
            for r in range {
                if r.record.component.select_weight == 0 {
                    r.record.component.select_weight = q as u16;
                    a = a.wrapping_add(q);
                }
            }
        }
        if let Some(last) = self.visual.components.last_mut() {
            last.record.component.select_weight = last
                .record
                .component
                .select_weight
                .wrapping_add(65535u32.wrapping_sub(a) as u16);
        }
    }
    pub fn enter_particle_sequence(&mut self, host: &mut impl Services) -> Result<()> {
        self.nested_context()?;
        self.byte_4ac = 1;
        host.call(Call::InheritedBlock)?;
        Ok(())
    }
    pub fn exit_particle_sequence(&mut self) {
        self.byte_4ac = 0;
    }

    /// The shared constructor word is an explicit primitive input, not a value
    /// read from an executable image or guessed from a default city.
    pub fn anonymous_component(
        &mut self,
        kind: Kind,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
        policy: NumericPolicy,
        shared_word: u32,
    ) -> Result<()> {
        self.nested_context()?;
        self.positional(tokens, 0, locale)?;
        if kind == Kind::Sound && !self.scratch.is_empty() {
            return Err(Error::Parse("sound description unimplemented".into()));
        }
        let object =
            object(host.call(Call::AllocateNested(kind))?)?.ok_or(Error::Unsupported("typed allocation failure"))?;
        host.call(Call::Retain(object))?;
        let mut d = Description::default();
        let c = policy;
        let result = (|| {
            Ok(match kind {
                Kind::Sound => self.sound_payload(&mut d, tokens, locale, host, c)?,
                Kind::Brush => {
                    self.brush_payload(&mut d, tokens, locale, host, c)?;
                    false
                }
                Kind::Scrubber => {
                    self.scrubber_payload(&mut d, tokens, locale, host, c, shared_word)?;
                    true
                }
                Kind::Automata => {
                    d.strings.insert(0x0c, Vec::new());
                    d.bytes.insert(0x18, 0);
                    if self.switch(tokens, b"name", 1, locale)? {
                        d.strings.insert(0x0c, self.selected(tokens, 0)?.to_vec());
                    } else if self.switch(tokens, b"group", 1, locale)? {
                        d.strings.insert(0x0c, self.selected(tokens, 0)?.to_vec());
                        d.bytes.insert(0x18, 1);
                    } else {
                        return Err(Error::Parse(
                            "Need at least -name or -group for anonymous attractor effect".into(),
                        ));
                    }
                    true
                }
                Kind::Camera => {
                    d.words.insert(0x0c, 0);
                    d.words.insert(0x14, 0);
                    d.bytes.insert(0x10, 0);
                    d.bytes.insert(0x11, 0);
                    for (key, bit) in [(b"slave".as_slice(), 8), (b"target".as_slice(), 4)] {
                        if self.switch(tokens, key, 0, locale)? {
                            *d.words.get_mut(&0x0c).unwrap() |= bit;
                        }
                    }
                    if self.switch(tokens, b"attachRadius", 1, locale)? {
                        d.words.insert(0x14, stored(host, self.selected(tokens, 0)?, c)?);
                    }
                    for (key, lo, hi, bit) in [(b"zoom".as_slice(), 1, 5, 1), (b"rotation".as_slice(), 0, 3, 2)] {
                        if self.switch(tokens, key, 1, locale)? {
                            let value = range(int(host, self.selected(tokens, 0)?)?, lo, hi)?;
                            d.bytes.insert(0x10, (value - lo as u32) as u8);
                            *d.words.get_mut(&0x0c).unwrap() |= bit;
                        }
                    }
                    true
                }
            })
        })();
        self.pending_nested = Some((object, kind, d.clone()));
        let initial: bool = result?;
        if let Some(collection) = self.collection {
            let mut record = fresh();
            record.component.index = match host.call(Call::InsertNested(collection, object, d))? {
                Reply::Uint(x) => x,
                _ => return Err(Error::Unsupported("anonymous index reply")),
            };
            let ty = match host.call(Call::NestedType(object))? {
                Reply::Uint(x) => x,
                _ => return Err(Error::Unsupported("description type reply")),
            };
            record.set_type(ty);
            record.component.select_group = self.word_4a0 as u16;
            record.initial_bit(initial);
            self.component_options(&mut record, tokens, locale, host, policy)?;
            self.visual.components.push(Component {
                name: Vec::new(),
                record,
            });
        }
        host.call(Call::Release(object))?;
        self.pending_nested = None;
        Ok(())
    }
    fn sound_payload(
        &mut self,
        d: &mut Description,
        t: &mut [ScriptToken],
        l: &mut impl CharacterRules,
        h: &mut impl Services,
        c: NumericPolicy,
    ) -> Result<bool> {
        d.words
            .extend([(0x0c, 0), (0x10, 0), (0x14, 0.5f32.to_bits()), (0x18, 0)]);
        if !self.switch(t, b"name", 1, l)? {
            return Err(Error::Parse("Need at least -name for anonymous soundEffect".into()));
        }
        let key = fold(self.selected(t, 0)?, l);
        let id = self.id_maps[2]
            .get(&key)
            .copied()
            .ok_or_else(|| Error::Parse(format!("Unknown sound {}", String::from_utf8_lossy(&key))))?;
        d.words.insert(0x10, id);
        if self.switch(t, b"locationUpdateRate", 1, l)? {
            let x = extended(h, self.selected(t, 0)?)?;
            d.words.insert(
                0x14,
                if x.positive() {
                    let control = numeric::control(c).map_err(numeric_error)?;
                    x.reciprocal(control).store(control)
                } else {
                    0
                },
            );
        }
        if self.switch(t, b"length", 1, l)? {
            d.words.insert(0x18, stored(h, self.selected(t, 0)?, c)?);
        }
        Ok(d.words[&0x18] & 0x7fffffff == 0)
    }
    fn brush_payload(
        &mut self,
        d: &mut Description,
        t: &mut [ScriptToken],
        l: &mut impl CharacterRules,
        h: &mut impl Services,
        c: NumericPolicy,
    ) -> Result<()> {
        d.words.extend([
            (0x0c, 7),
            (0x10, 1f32.to_bits()),
            (0x14, 0),
            (0x1c, u32::MAX),
            (0x20, 1f32.to_bits()),
            (0x24, 1f32.to_bits()),
            (0x28, 1f32.to_bits()),
            (0x2c, 1f32.to_bits()),
            (0x30, 100f32.to_bits()),
        ]);
        d.bytes.insert(0x18, 0);
        if !self.switch(t, b"name", 1, l)? {
            return Err(Error::Parse("need at least -name for an anonymous brush effect".into()));
        }
        let key = fold(self.selected(t, 0)?, l);
        let mut fallback = key.clone();
        fallback.extend_from_slice(b"brush");
        let id = self.id_maps[3]
            .get(&key)
            .or_else(|| self.id_maps[3].get(&fallback))
            .copied()
            .ok_or_else(|| Error::Parse(format!("No such brush: '{}'", String::from_utf8_lossy(&key))))?;
        d.words.insert(0x0c, id);
        for (key, at) in [
            (b"rate".as_slice(), 0x10),
            (b"apply".as_slice(), 0x10),
            (b"length".as_slice(), 0x14),
        ] {
            if self.switch(t, key, 1, l)? {
                d.words.insert(at, stored(h, self.selected(t, 0)?, c)?);
                if key == b"apply" {
                    d.bytes.insert(0x18, 1);
                }
            }
        }
        if self.switch(t, b"zoom", 1, l)? {
            d.words.insert(0x1c, range(int(h, self.selected(t, 0)?)?, 1, 5)? - 1);
        }
        for (key, at) in [(b"strength".as_slice(), 0x20), (b"width".as_slice(), 0x28)] {
            if self.switch(t, key, 1, l)? {
                let first = stored(h, self.selected(t, 0)?, c)?;
                d.words.insert(at, first);
                let second = if self.scratch.len() > 1 {
                    stored(h, self.selected(t, 1)?, c)?
                } else {
                    first
                };
                d.words.insert(at + 4, second);
            }
        }
        if self.switch(t, b"level", 1, l)? {
            d.words.insert(0x30, stored(h, self.selected(t, 0)?, c)?);
        }
        Ok(())
    }
    fn scrubber_payload(
        &mut self,
        d: &mut Description,
        t: &mut [ScriptToken],
        l: &mut impl CharacterRules,
        h: &mut impl Services,
        c: NumericPolicy,
        shared: u32,
    ) -> Result<()> {
        for at in [
            0x0c, 0x10, 0x14, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x48, 0x4c,
        ] {
            d.words.insert(at, 0);
        }
        d.words
            .extend([(0x18, 1), (0x3c, 16f32.to_bits()), (0x40, shared), (0x44, shared)]);
        let mut admitted = false;
        for (key, at) in [
            (b"demolish".as_slice(), 0x14),
            (b"burn".as_slice(), 0x24),
            (b"toxic".as_slice(), 0x28),
        ] {
            if self.switch(t, key, 1, l)? {
                admitted = true;
                let x = raw(h, self.selected(t, 0)?)?;
                let v = match h.call(Call::Ftoll(x))? {
                    Reply::Int(x) => x as u32,
                    _ => return Err(Error::Unsupported("CRT integer reply")),
                };
                d.words.insert(at, v);
            }
        }
        if self.switch(t, b"extinguishFire", 1, l)? {
            admitted = true;
            d.words.insert(0x2c, uint(h, self.selected(t, 0)?)?);
        }
        for (key, bit) in [
            (b"pauseSim".as_slice(), 0x10),
            (b"pauseSimHidden".as_slice(), 0x20),
            (b"pauseClock".as_slice(), 0x40),
        ] {
            if self.switch(t, key, 0, l)? {
                admitted = true;
                *d.words.get_mut(&0x0c).unwrap() |= bit;
                if !self.scratch.is_empty() {
                    d.words.insert(0x4c, stored(h, self.selected(t, 0)?, c)?);
                }
            }
        }
        if self.switch(t, b"message", 1, l)? {
            admitted = true;
            d.words.insert(0x30, uint(h, self.selected(t, 0)?)?);
            if self.scratch.len() > 1 {
                d.words.insert(0x34, uint(h, self.selected(t, 1)?)?);
            }
        }
        for (key, minimum) in [(b"blob".as_slice(), 3), (b"rect".as_slice(), 4)] {
            if self.switch(t, key, minimum, l)? {
                admitted = true;
                d.words.insert(0x38, range(int(h, self.selected(t, 0)?)?, 1, 8)?);
                d.words.insert(0x3c, stored(h, self.selected(t, 1)?, c)?);
                let third = stored(h, self.selected(t, 2)?, c)?;
                d.words.insert(0x40, third);
                d.words.insert(
                    0x44,
                    if minimum == 3 {
                        third
                    } else {
                        stored(h, self.selected(t, 3)?, c)?
                    },
                );
                if self.scratch.len() > minimum {
                    d.words.insert(0x48, stored(h, self.selected(t, minimum)?, c)?);
                }
            }
        }
        if !admitted {
            return Err(Error::Parse("Need some options for anonymous scrubber effect".into()));
        }
        for (key, bit) in [
            (b"noNetworks".as_slice(), 1),
            (b"noFlora".as_slice(), 2),
            (b"dezone".as_slice(), 4),
            (b"single".as_slice(), 8),
        ] {
            if self.switch(t, key, 0, l)? {
                *d.words.get_mut(&0x0c).unwrap() |= bit;
            }
        }
        let mut upper = 0;
        let mut low = 1;
        if self.switch(t, b"explode", 0, l)? {
            upper = 0x1300;
            low = 2;
        }
        if self.switch(t, b"createRubble", 0, l)? {
            upper = 0x1300;
        }
        if self.switch(t, b"createBurntRubble", 0, l)? {
            upper |= 0x300;
        }
        if self.switch(t, b"demolishEffectID", 0, l)? {
            low = uint(h, self.selected(t, 0)?)? & 255;
        }
        d.words.insert(0x18, upper | low);
        for (key, at) in [
            (b"minDemolishSize".as_slice(), 0x1c),
            (b"maxDemolishSize".as_slice(), 0x20),
        ] {
            if self.switch(t, key, 1, l)? {
                d.words.insert(at, stored(h, self.selected(t, 0)?, c)?);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
