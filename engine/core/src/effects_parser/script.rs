// SPDX-License-Identifier: GPL-3.0-or-later
//! Tokenized script boundary, from effects-parser §§9–12. No lexer or file
//! recovery policy is implied. Errors preserve all completed owner mutations.

use super::{Descriptor, Identity};
use std::collections::BTreeMap;

pub mod commands;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Parse(String),
    Unsupported(&'static str),
    Foreign(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Recorded bytes and original index survive switch marking. The tokenizer
/// owns valid record construction; readers never split a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptToken {
    pub bytes: Vec<u8>,
    pub index: u32,
}
impl ScriptToken {
    pub fn new(index: u32, text: impl AsRef<[u8]>) -> Self {
        Self {
            bytes: text.as_ref().to_vec(),
            index,
        }
    }
}

/// CRT locale is deliberately foreign, including the non-ASCII domain.
pub trait CharacterRules {
    fn is_alpha(&mut self, byte: u8) -> bool;
    fn lower(&mut self, byte: u8) -> u8;
}
#[derive(Default)]
pub struct Ascii;
impl CharacterRules for Ascii {
    fn is_alpha(&mut self, byte: u8) -> bool {
        byte.is_ascii_alphabetic()
    }
    fn lower(&mut self, byte: u8) -> u8 {
        byte.to_ascii_lowercase()
    }
}
fn terminates(token: &ScriptToken, locale: &mut impl CharacterRules) -> bool {
    token.bytes.len() > 1 && matches!(token.bytes[0], b'-' | b'+') && locale.is_alpha(token.bytes[1])
}
fn collect(tokens: &[ScriptToken], start: usize, scratch: &mut Vec<usize>, locale: &mut impl CharacterRules) {
    for (i, token) in tokens.iter().enumerate().skip(start) {
        if terminates(token, locale) {
            break;
        }
        scratch.push(i);
    }
}
pub fn find_switch(
    tokens: &mut [ScriptToken],
    key: &[u8],
    minimum: usize,
    scratch: &mut Vec<usize>,
    locale: &mut impl CharacterRules,
) -> Result<bool> {
    scratch.clear();
    let Some(at) = tokens
        .iter()
        .position(|t| t.bytes.len() == key.len() + 1 && t.bytes[0] == b'-' && t.bytes[1..] == *key)
    else {
        return Ok(false);
    };
    tokens[at].bytes[0] = b'+';
    collect(tokens, at + 1, scratch, locale);
    if scratch.len() < minimum {
        return Err(Error::Parse(format!(
            "Expecting at least {minimum} arguments for -{}",
            String::from_utf8_lossy(key)
        )));
    }
    Ok(true)
}
pub fn find_positional(
    tokens: &[ScriptToken],
    minimum: usize,
    scratch: &mut Vec<usize>,
    locale: &mut impl CharacterRules,
) -> Result<()> {
    scratch.clear();
    collect(tokens, 1, scratch, locale);
    if scratch.len() < minimum {
        return Err(Error::Parse(format!("Expecting at least {minimum} arguments")));
    }
    Ok(())
}
pub fn check_switches(tokens: &[ScriptToken], locale: &mut impl CharacterRules) -> Result<()> {
    for token in tokens {
        if token.bytes.first() == Some(&b'-') && terminates(token, locale) {
            return Err(Error::Parse(format!(
                "Unexpected switch: {}",
                String::from_utf8_lossy(&token.bytes)
            )));
        }
    }
    Ok(())
}
pub fn fold(bytes: &[u8], locale: &mut impl CharacterRules) -> Vec<u8> {
    bytes.iter().map(|&b| locale.lower(b)).collect()
}

pub trait DescriptorHooks {
    fn register(&mut self, descriptor: Descriptor);
    fn destroy(&mut self, descriptor: Descriptor);
    fn command(&mut self, descriptor: Descriptor, tokens: &mut [ScriptToken]) -> Result<()>;
    fn end(&mut self, descriptor: Descriptor) -> Result<()>;
}
/// Literal keys are borrowed from persistent registration storage (§12.1).
#[derive(Default)]
pub struct Dictionary {
    pub roots: Vec<Descriptor>,
    pub root_names: BTreeMap<&'static [u8], usize>,
    pub children: BTreeMap<u32, BTreeMap<&'static [u8], Descriptor>>,
    pub stack: Vec<Descriptor>,
}
impl Dictionary {
    pub fn root(&mut self, name: &'static [u8], descriptor: Descriptor, host: &mut impl DescriptorHooks) {
        self.root_names.insert(name, self.roots.len());
        self.roots.push(descriptor);
        host.register(descriptor);
    }
    pub fn child(
        &mut self,
        block: Descriptor,
        name: &'static [u8],
        descriptor: Descriptor,
        host: &mut impl DescriptorHooks,
    ) {
        self.children.entry(block.0).or_default().insert(name, descriptor);
        host.register(descriptor);
    }
    pub fn start(&mut self, block: Descriptor) {
        self.stack.push(block);
    }
    pub fn dispatch(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl DescriptorHooks,
    ) -> Result<()> {
        let name = tokens
            .first()
            .ok_or_else(|| Error::Parse("Unknown command ''".into()))?
            .bytes
            .clone();
        // Macro capture and variable substitution must already have returned.
        if name == b"end" {
            let block = *self
                .stack
                .last()
                .ok_or_else(|| Error::Parse("Not inside a block!".into()))?;
            host.end(block)?;
            self.stack.pop();
            return Ok(());
        }
        for block in self.stack.iter().rev() {
            if let Some(descriptor) = self.children.get(&block.0).and_then(|d| d.get(name.as_slice())) {
                host.command(*descriptor, tokens)?;
                return check_switches(tokens, locale);
            }
        }
        let index = self
            .root_names
            .get(name.as_slice())
            .ok_or_else(|| Error::Parse(format!("Unknown command '{}'", String::from_utf8_lossy(&name))))?;
        host.command(self.roots[*index], tokens)
    }
    pub fn destroy_root(&mut self, host: &mut impl DescriptorHooks) {
        for descriptor in self.roots.drain(..) {
            host.destroy(descriptor);
        }
        self.root_names.clear();
    }
    pub fn destroy_block(&mut self, block: Descriptor, host: &mut impl DescriptorHooks) {
        if let Some(children) = self.children.remove(&block.0) {
            for descriptor in children.into_values() {
                host.destroy(descriptor);
            }
        }
    }
}

pub trait ObjectReferences {
    fn retain(&mut self, object: Identity);
    fn release(&mut self, object: Identity);
}
#[derive(Default)]
pub struct Bucket {
    pub entries: Vec<Identity>,
    pub names: BTreeMap<Vec<u8>, u32>,
}
#[derive(Default)]
pub struct Directory {
    pub buckets: BTreeMap<u32, Bucket>,
}
impl Directory {
    pub fn lookup(
        &self,
        kind: u32,
        name: &[u8],
        out: Option<&mut Option<Identity>>,
        locale: &mut impl CharacterRules,
        refs: &mut impl ObjectReferences,
    ) -> bool {
        let key = fold(name, locale);
        let Some(bucket) = self.buckets.get(&kind) else {
            return false;
        };
        let Some(&index) = bucket.names.get(&key) else {
            return false;
        };
        if let Some(out) = out {
            let object = bucket.entries[index as usize];
            *out = Some(object);
            refs.retain(object);
        }
        true
    }
    /// kind is the returning result of the object's type getter, not inferred
    /// from the declaration name. Only valid directory types are admissible.
    pub fn register(
        &mut self,
        kind: u32,
        name: Option<&[u8]>,
        object: Identity,
        locale: &mut impl CharacterRules,
        refs: &mut impl ObjectReferences,
    ) -> u32 {
        let bucket = self.buckets.entry(kind).or_default();
        let key = name.map(|n| fold(n, locale));
        if let Some(&index) = key.as_ref().and_then(|k| bucket.names.get(k)) {
            let old = bucket.entries[index as usize];
            if old != object {
                refs.retain(object);
                bucket.entries[index as usize] = object;
                refs.release(old);
            }
            return index;
        }
        let index = bucket.entries.len() as u32;
        // Spare-capacity append's temporary and stored references (§11.1).
        refs.retain(object);
        refs.retain(object);
        bucket.entries.push(object);
        refs.release(object);
        if let Some(key) = key {
            bucket.names.insert(key, index);
        }
        index
    }
}

pub trait Destruction {
    fn dispose_names_and_temporaries(&mut self);
    fn release_component(&mut self, component: Identity);
    fn dispose_descriptions_and_id_maps(&mut self);
    fn release_interface(&mut self, interface: Identity);
    fn base_destroy(&mut self);
    fn free_allocation(&mut self);
}
pub struct Lifetime {
    pub count: u32,
    pub component: Option<Identity>,
    /// Renderer properties, resource, collection, manager, in release order.
    pub interfaces: [Option<Identity>; 4],
}
impl Lifetime {
    pub fn add_ref(&mut self) -> u32 {
        self.count = self.count.wrapping_add(1);
        self.count
    }
    pub fn release(&mut self, host: &mut impl Destruction) -> u32 {
        if self.count > 1 {
            self.count -= 1;
            return self.count;
        }
        // Final Release does not first store count=0 or clear held slots.
        self.destroy(true, host);
        0
    }
    pub fn destroy(&self, delete: bool, host: &mut impl Destruction) {
        host.dispose_names_and_temporaries();
        if let Some(component) = self.component {
            host.release_component(component);
        }
        host.dispose_descriptions_and_id_maps();
        for interface in self.interfaces.into_iter().flatten() {
            host.release_interface(interface);
        }
        host.base_destroy();
        if delete {
            host.free_allocation();
        }
    }
}

#[cfg(test)]
mod tests;
