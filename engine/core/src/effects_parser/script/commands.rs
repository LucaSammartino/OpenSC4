// SPDX-License-Identifier: GPL-3.0-or-later
//! Root command owners (effects §19). Every conversion and service call remains
//! an individual foreign operation; no service returns an executed command.

use super::*;
use crate::effects::{EffectDescription, DIRECTORY_GROUP, DIRECTORY_TYPE};

pub mod children;
pub mod components;
pub mod definitions;
pub mod motion;
pub mod particle;
pub mod resources;
use definitions::{DefinitionKind, Definitions, Description};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Bool,
    Int,
    Float,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub distances: Vec<f32>,
    pub first: f32,
    pub span: f32,
    pub size: f32,
    pub side_swipe: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Visual {
    pub description: EffectDescription,
    /// Constructor words +24/+28 are indeterminate until assigned. Payload
    /// placeholders in EffectDescription are never conformance values.
    pub message_known: [bool; 3],
    pub components: Vec<components::Component>,
    pub auxiliary: Vec<components::Auxiliary>,
    pub chain: Vec<u8>,
}
impl Default for Visual {
    fn default() -> Self {
        Self {
            description: EffectDescription::new(1, Vec::new()),
            message_known: [true, false, false],
            components: Vec::new(),
            auxiliary: Vec::new(),
            chain: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Priority {
    pub name: Vec<u8>,
    pub priority: i32,
    pub rename: Vec<u8>,
    pub used: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub change: u8,
    pub flags: u8,
    pub basis: [[f32; 3]; 3],
    pub translation: [f32; 3],
    pub scale: f32,
}
impl Transform {
    fn new(translation: [f32; 3], scale: f32) -> Self {
        Self {
            change: 2,
            flags: if scale == 1.0 { 1 } else { 3 },
            basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation,
            scale,
        }
    }
}
/// Independent returning primitives at the effects-command boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    ValidateNested(Identity, u32, Vec<u8>),
    AllocateNested(components::Kind),
    InsertNested(Identity, Identity, Description),
    NestedType(Identity),
    InheritedBlock,
    Ftoll(crate::effects_parser::RawFloat),
    Inherit(Identity, DefinitionKind, Vec<u8>, Description),
    PublishDefinition(Identity, DefinitionKind, Vec<u8>, Description),
    AllocateSequence,
    PublishSequence(Identity, Vec<u8>, Option<Identity>),
    Uint(Vec<u8>),
    Int(Vec<u8>),
    Float(Vec<u8>),
    Bool(Vec<u8>),
    Vector2(Vec<u8>),
    Vector3(Vec<u8>),
    Color(Vec<u8>),
    Enum(Vec<u8>, &'static [(&'static str, u32)]),
    CountArguments(Vec<u8>),
    SplitArguments(Vec<u8>),
    /// Individual transcendental primitive; qualification is EFP7.
    Cos(f64),
    Sin(f64),
    PropertyIndex(Identity, Category, Vec<u8>),
    SetProperty(Identity, Category, i32, Box<Reply>),
    EffectId(Identity, Vec<u8>, u32, u32),
    Message(Identity, u32, Vec<u8>),
    Camera(Identity, Camera),
    PublishVisual(Identity, Vec<u8>, Visual),
    CreateResource(Identity, [u32; 3]),
    NameResource(Identity, Vec<u8>),
    QueryCollection(Identity),
    Retain(Identity),
    Release(Identity),
    Segment(Identity, u32),
    SaveResource(Identity, [u32; 3], Option<Identity>),
    LoadResource(Identity, [u32; 3]),
    ImportResource(Identity, Option<Identity>),
    Terrain(f32, f32),
    Exists(Identity, Vec<u8>),
    CreatePreview(Identity, Vec<u8>),
    SourceTransform(Identity, Transform),
    InstanceTransform(Identity, Transform),
    Speed(Identity, f32),
    Target(Identity, [f32; 3]),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    RawFloat(crate::effects_parser::RawFloat),
    Inherited(bool, Description),
    Void,
    Uint(u32),
    Int(i32),
    Float(f64),
    /// Exact pre-store binary32-domain input, including nonfinite primitive
    /// replies. No owner finite-value validation is implied by a direct store.
    Float32(u32),
    Bool(bool),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Arguments(Vec<Vec<u8>>),
    Object(Option<Identity>),
    StatusObject(bool, Option<Identity>),
    /// A setter may mutate the shared temporary passed by address.
    Transform(Transform),
}
pub trait Services {
    fn call(&mut self, call: Call) -> Result<Reply>;
    fn rotation_arithmetic(&mut self) -> Option<&mut dyn crate::effects_parser::rotation::Arithmetic> {
        None
    }
}
fn uint(host: &mut impl Services, bytes: &[u8]) -> Result<u32> {
    match host.call(Call::Uint(bytes.to_vec()))? {
        Reply::Uint(x) => Ok(x),
        _ => Err(Error::Unsupported("uint reply")),
    }
}
fn int(host: &mut impl Services, bytes: &[u8]) -> Result<i32> {
    match host.call(Call::Int(bytes.to_vec()))? {
        Reply::Int(x) => Ok(x),
        _ => Err(Error::Unsupported("int reply")),
    }
}
fn raw_float(host: &mut impl Services, bytes: &[u8]) -> Result<f64> {
    match host.call(Call::Float(bytes.to_vec()))? {
        Reply::Float(x) => Ok(x),
        Reply::Float32(bits) => Ok(f64::from(f32::from_bits(bits))),
        _ => Err(Error::Unsupported("float reply")),
    }
}
fn float(host: &mut impl Services, bytes: &[u8]) -> Result<f32> {
    let x = raw_float(host, bytes)?;
    Ok(x as f32)
}
fn vector(host: &mut impl Services, bytes: &[u8]) -> Result<[f32; 3]> {
    match host.call(Call::Vector3(bytes.to_vec()))? {
        Reply::Vector3(v) => Ok(v),
        _ => Err(Error::Unsupported("vector reply")),
    }
}
fn object(reply: Reply) -> Result<Option<Identity>> {
    match reply {
        Reply::Object(o) => Ok(o),
        _ => Err(Error::Unsupported("object reply")),
    }
}
fn status_object(reply: Reply) -> Result<(bool, Option<Identity>)> {
    match reply {
        Reply::StatusObject(s, o) => Ok((s, o)),
        _ => Err(Error::Unsupported("status/object reply")),
    }
}
fn boolean(reply: Reply) -> Result<bool> {
    match reply {
        Reply::Bool(v) => Ok(v),
        _ => Err(Error::Unsupported("boolean reply")),
    }
}
fn arity(tokens: &[ScriptToken], arguments: usize) -> Result<()> {
    if tokens.len() != arguments + 1 {
        return Err(Error::Parse("Wrong number of arguments".into()));
    }
    Ok(())
}
fn key(instance: u32) -> [u32; 3] {
    [DIRECTORY_TYPE, DIRECTORY_GROUP, instance]
}

#[derive(Default)]
pub struct Commands {
    pub definitions: Definitions,
    pub word_494: u32,
    pub scalar_498: f32,
    pub manager: Option<Identity>,
    pub collection: Option<Identity>,
    pub renderer: Option<Identity>,
    pub resource_manager: Option<Identity>,
    pub resource: Option<Identity>,
    pub resource_enabled: bool,
    pub resource_key: [u32; 3],
    pub group: u32,
    /// Texture, model, sound, brush maps, in that order.
    pub id_maps: [BTreeMap<Vec<u8>, u32>; 4],
    pub pending_priorities: Vec<Priority>,
    pub name: Vec<u8>,
    pub visual: Visual,
    pub scratch: Vec<usize>,
    pub word_4a0: u32,
    pub word_4a4: u32,
    pub word_4a8: u32,
    pub byte_4ac: u8,
    pub previews: Vec<Identity>,
    pub hard: Vec<u32>,
    pub pending_nested: Option<(Identity, components::Kind, Description)>,
}
impl Commands {
    fn positional(&mut self, tokens: &[ScriptToken], count: usize, locale: &mut impl CharacterRules) -> Result<()> {
        find_positional(tokens, count, &mut self.scratch, locale)
    }
    fn switch(
        &mut self,
        tokens: &mut [ScriptToken],
        key: &[u8],
        count: usize,
        locale: &mut impl CharacterRules,
    ) -> Result<bool> {
        find_switch(tokens, key, count, &mut self.scratch, locale)
    }
    fn value<'a>(&self, tokens: &'a [ScriptToken], at: usize) -> &'a [u8] {
        &tokens[self.scratch[at]].bytes
    }
    pub fn render_property(&mut self, tokens: &[ScriptToken], host: &mut impl Services) -> Result<()> {
        arity(tokens, 2)?;
        let renderer = self
            .renderer
            .ok_or(Error::Unsupported("live renderer properties required"))?;
        for category in [Category::Bool, Category::Int, Category::Float] {
            let index = match host.call(Call::PropertyIndex(renderer, category, tokens[1].bytes.clone()))? {
                Reply::Int(v) => v,
                _ => return Err(Error::Unsupported("property index reply")),
            };
            if index < 0 {
                continue;
            }
            let value = match category {
                Category::Bool => Reply::Bool(boolean(host.call(Call::Bool(tokens[2].bytes.clone()))?)?),
                Category::Int => Reply::Int(int(host, &tokens[2].bytes)?),
                Category::Float => Reply::Float(f64::from(float(host, &tokens[2].bytes)?)),
            };
            host.call(Call::SetProperty(renderer, category, index, Box::new(value)))?;
            return Ok(());
        }
        Err(Error::Parse("Unknown render property".into()))
    }
    pub fn declare_id(
        &mut self,
        map: usize,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        arity(tokens, 2)?;
        let name = fold(&tokens[1].bytes, locale);
        let entry = self.id_maps[map].entry(name).or_default();
        *entry = uint(host, &tokens[2].bytes)?;
        Ok(())
    }
    pub fn effect_id(
        &mut self,
        tokens: &[ScriptToken],
        instance: bool,
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        arity(tokens, if instance { 2 } else { 3 })?;
        let group = if instance {
            None
        } else {
            Some(uint(host, &tokens[1].bytes)?)
        };
        let id = uint(host, &tokens[if instance { 1 } else { 2 }].bytes)?;
        let name = fold(&tokens[if instance { 2 } else { 3 }].bytes, locale);
        if let Some(c) = self.collection {
            host.call(Call::EffectId(c, name, group.unwrap_or(self.group), id))?;
        }
        Ok(())
    }
    pub fn enter_group(&mut self, tokens: &[ScriptToken], host: &mut impl Services) -> Result<()> {
        if tokens.len() < 2 {
            return Err(Error::Parse("No ID specified!".into()));
        }
        self.group = uint(host, &tokens[1].bytes)?;
        Ok(()) // caller pushes the registered block only after this returns
    }
    pub fn message(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 2, locale)?;
        let id = uint(host, self.value(tokens, 0))?;
        let name = self.value(tokens, 1).to_vec();
        if let Some(c) = self.collection {
            host.call(Call::Message(c, id, name))?;
        }
        Ok(())
    }
    pub fn camera(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        let mut distances = Vec::new();
        for &index in &self.scratch {
            distances.push(float(host, &tokens[index].bytes)?);
        }
        if distances.len() == 1 {
            for _ in 0..4 {
                distances.push(*distances.last().unwrap() * 0.5);
            }
        }
        let mut endpoints = [100f32, 101f32];
        if self.switch(tokens, b"parallax", 2, locale)? {
            endpoints[0] = float(host, self.value(tokens, 0))?;
            endpoints[1] = float(host, self.value(tokens, 1))?;
        }
        let mut size = 4.;
        if self.switch(tokens, b"size", 1, locale)? {
            size = float(host, self.value(tokens, 0))?;
        }
        let mut side_swipe = 7.;
        if self.switch(tokens, b"sideSwipe", 1, locale)? {
            side_swipe = float(host, self.value(tokens, 0))?;
        }
        let difference = (f64::from(endpoints[1]) - f64::from(endpoints[0])).trunc();
        if !difference.is_finite() || difference < f64::from(i32::MIN) || difference > f64::from(i32::MAX) {
            return Err(Error::Unsupported("EFP7 ftol2 domain"));
        }
        let record = Camera {
            distances,
            first: endpoints[0],
            span: (difference as i32).max(0) as f32,
            size,
            side_swipe,
        };
        if let Some(c) = self.collection {
            host.call(Call::Camera(c, record))?;
        }
        Ok(())
    }
    pub fn priority(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 2, locale)?;
        let name = self.value(tokens, 0).to_vec();
        let priority = int(host, self.value(tokens, 1))?;
        let rename = if self.scratch.len() > 2 {
            self.value(tokens, 2).to_vec()
        } else {
            Vec::new()
        };
        if !self.pending_priorities.iter().any(|p| p.name == name) {
            self.pending_priorities.push(Priority {
                name,
                priority,
                rename,
                used: false,
            });
        }
        Ok(())
    }
    pub fn enter_visual(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if !self.name.is_empty() {
            return Err(Error::Parse(
                "Can't nest effects! (Perhaps you meant visualEffect?)".into(),
            ));
        }
        self.positional(tokens, 1, locale)?;
        self.name = self.value(tokens, 0).to_vec();
        self.visual = Visual::default();
        for (key, flag) in [
            (b"viewRelative".as_slice(), 1),
            (b"noAutoStop", 2),
            (b"rigid", 8),
            (b"ignoreOrientation", 0x40),
            (b"hardStop", 4),
            (b"noPropagate", 0x10),
            (b"applyCursor", 0x20),
            (b"noLODStop", 0x80),
        ] {
            if self.switch(tokens, key, 0, locale)? {
                self.visual.description.flags |= flag;
            }
        }
        if self.switch(tokens, b"startMessage", 1, locale)? {
            for i in 0..self.scratch.len().min(3) {
                self.visual.description.extra[i] = uint(host, self.value(tokens, i))?;
                self.visual.message_known[i] = true;
            }
        }
        if self.switch(tokens, b"manualRestart", 0, locale)? {
            self.visual.description.flags |= 0x100;
        }
        if let Some(p) = self.pending_priorities.iter_mut().find(|p| p.name == self.name) {
            // Existing runtime representation is unsigned; preserve signed bits.
            self.visual.description.priority = p.priority as u32;
            if !p.rename.is_empty() {
                self.name.clone_from(&p.rename);
            }
            p.used = true;
        } else if self.switch(tokens, b"priority", 1, locale)? {
            let value = int(host, self.value(tokens, 0))?;
            if !(1..=5).contains(&value) {
                return Err(Error::Parse(format!(
                    "argument out of range: expecting [1..5], got {value}"
                )));
            }
            self.visual.description.priority = value as u32;
        }
        self.word_4a0 = 0;
        self.word_4a4 = 0;
        self.byte_4ac = 0;
        Ok(())
    }
    pub fn exit_visual(&mut self, host: &mut impl Services) -> Result<()> {
        if let Some(c) = self.collection {
            host.call(Call::PublishVisual(c, self.name.clone(), self.visual.clone()))?;
        }
        self.name.clear();
        Ok(())
    }
    fn assign_collection(&mut self, collection: Option<Identity>, host: &mut impl Services) -> Result<()> {
        if collection != self.collection {
            if let Some(c) = collection {
                host.call(Call::Retain(c))?;
            }
            let old = self.collection;
            self.collection = collection;
            if let Some(c) = old {
                host.call(Call::Release(c))?;
            }
        }
        Ok(())
    }
    fn query_collection(&mut self, owner: Identity, host: &mut impl Services) -> Result<()> {
        let new = object(host.call(Call::QueryCollection(owner))?)?;
        self.assign_collection(new, host)?;
        if let Some(c) = new {
            host.call(Call::Release(c))?;
        }
        Ok(())
    }
    pub fn enter_resource(&mut self, tokens: &[ScriptToken], host: &mut impl Services) -> Result<()> {
        arity(tokens, 2)?;
        if self.resource.is_some() {
            return Err(Error::Parse("Can't nest resource blocks".into()));
        }
        if self.resource_enabled {
            let id = uint(host, &tokens[1].bytes)?;
            let manager = self
                .resource_manager
                .ok_or(Error::Unsupported("resource manager required"))?;
            // Status deliberately ignored: the owner's decision is the output.
            let (_, resource) = status_object(host.call(Call::CreateResource(manager, key(id)))?)?;
            self.resource = resource;
            if let Some(resource) = resource {
                host.call(Call::NameResource(resource, tokens[2].bytes.clone()))?;
                self.resource_key = key(id);
                self.query_collection(resource, host)?;
            }
        }
        Ok(())
    }
    pub fn exit_resource(&mut self, host: &mut impl Services) -> Result<()> {
        if !self.resource_enabled || self.resource.is_none() {
            return Ok(());
        }
        let manager = self
            .resource_manager
            .ok_or(Error::Unsupported("resource manager required"))?;
        let (status, segment) = status_object(host.call(Call::Segment(manager, self.resource_key[1]))?)?;
        if !status {
            return Err(Error::Parse(format!(
                "couldn't find segment for effect group 0x{:08x}",
                self.resource_key[1]
            )));
        }
        if !boolean(host.call(Call::SaveResource(manager, self.resource_key, segment))?)? {
            return Err(Error::Parse(format!(
                "couldn't save effects resource key 0x{:08x}",
                self.resource_key[2]
            )));
        }
        if let Some(base) = self.manager {
            self.query_collection(base, host)?;
        } else {
            self.assign_collection(None, host)?;
        }
        if let Some(resource) = self.resource.take() {
            host.call(Call::Release(resource))?;
        }
        if let Some(segment) = segment {
            host.call(Call::Release(segment))?;
        }
        Ok(())
    }
    pub fn load_resource(
        &mut self,
        tokens: &[ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        self.positional(tokens, 1, locale)?;
        let id = uint(host, self.value(tokens, 0))?;
        let Some(manager) = self.resource_manager else {
            return Ok(());
        };
        let (status, resource) = status_object(host.call(Call::LoadResource(manager, key(id)))?)?;
        if !status {
            return Err(Error::Parse(format!("Effects resource 0x{id:08x} not found")));
        }
        let resource = resource.ok_or(Error::Unsupported("successful load requires resource"))?;
        host.call(Call::ImportResource(resource, self.collection))?;
        host.call(Call::Release(resource))?;
        Ok(())
    }
    pub fn preview(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        let manager = self.manager.ok_or(Error::Unsupported("preview manager required"))?;
        self.positional(tokens, 1, locale)?;
        let name = self.value(tokens, 0).to_vec();
        let mut position = [512., 100., 512.];
        if self.scratch.len() > 1 {
            let bytes = self.value(tokens, 1);
            let count = match host.call(Call::CountArguments(bytes.to_vec()))? {
                Reply::Uint(v) => v,
                _ => return Err(Error::Unsupported("argument count reply")),
            };
            if count == 3 {
                position = vector(host, bytes)?;
            } else {
                let [x, z] = match host.call(Call::Vector2(bytes.to_vec()))? {
                    Reply::Vector2(v) => v,
                    _ => return Err(Error::Unsupported("vector2 reply")),
                };
                let height = match host.call(Call::Terrain(x, z))? {
                    Reply::Float(v) => v,
                    _ => return Err(Error::Unsupported("terrain reply")),
                };
                position = [x, (height + 4.) as f32, z];
            }
        }
        let mut source_scale = 1.;
        if self.switch(tokens, b"sourceScale", 1, locale)? {
            source_scale = float(host, self.value(tokens, 0))?;
        }
        let mut scale = 1.;
        if self.switch(tokens, b"scale", 1, locale)? {
            scale = float(host, self.value(tokens, 0))?;
        }
        let mut translation = [0.; 3];
        if self.switch(tokens, b"trans", 1, locale)? {
            translation = vector(host, self.value(tokens, 0))?;
        }
        if !boolean(host.call(Call::Exists(manager, name.clone()))?)? {
            return Err(Error::Parse("No such effect".into()));
        }
        let (status, instance) = status_object(host.call(Call::CreatePreview(manager, name))?)?;
        if status {
            let instance = instance.ok_or(Error::Unsupported("successful preview requires instance"))?;
            host.call(Call::Retain(instance))?;
            self.previews.push(instance);
            let mut transform = Transform::new(position, source_scale);
            if let Reply::Transform(modified) = host.call(Call::SourceTransform(instance, transform.clone()))? {
                transform = modified;
            }
            if transform.flags != 0 {
                transform.basis = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
                transform.change = transform.change.wrapping_add(1);
            }
            transform.translation = translation;
            transform.scale = scale;
            transform.flags = if scale == 1. { 1 } else { 3 };
            transform.change = transform.change.wrapping_add(2);
            host.call(Call::InstanceTransform(instance, transform))?;
            if self.switch(tokens, b"speed", 1, locale)? {
                let speed = float(host, self.value(tokens, 0))?;
                host.call(Call::Speed(instance, speed))?;
            }
            if self.switch(tokens, b"target", 1, locale)? {
                let target = vector(host, self.value(tokens, 0))?;
                host.call(Call::Target(instance, target))?;
            }
            let hard = self.switch(tokens, b"hard", 0, locale)?;
            self.hard.push(u32::from(hard));
        }
        if let Some(instance) = instance {
            host.call(Call::Release(instance))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
