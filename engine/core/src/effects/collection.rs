// SPDX-License-Identifier: GPL-3.0-or-later
//! Value publication and reload lifecycle (effects §18.6).

use super::directory::EffectDescription;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CameraParams {
    pub curve: Vec<f32>,
    pub scalars: [u32; 4],
}

/// Individual message-service operations; no precomputed collection reply.
pub trait Messages {
    fn subscribe(&mut self, message: u32);
    fn unsubscribe(&mut self, message: u32);
}

#[derive(Default)]
pub struct Collection {
    pub descriptions: Vec<EffectDescription>,
    /// Name entries retain stable description indices and insertion order.
    pub names: Vec<(String, usize)>,
    pub components: BTreeMap<u8, BTreeMap<String, i32>>,
    pub shakes: BTreeMap<String, u32>,
    pub lights: BTreeMap<String, u32>,
    pub bindings: Vec<(String, u32, u32)>,
    pub triggers: Vec<(u32, String)>,
    pub ids: BTreeMap<(u32, u32), usize>,
    pub trigger_lookup: BTreeMap<u32, usize>,
    pub camera: CameraParams,
    pub camera_present: bool,
    pub manager_camera: Option<CameraParams>,
}

impl Collection {
    pub fn begin(&mut self, messages: Option<&mut dyn Messages>) {
        if let Some(messages) = messages {
            for (id, _) in &self.triggers {
                messages.unsubscribe(*id);
            }
        }
        self.ids.clear();
        self.trigger_lookup.clear();
        self.bindings.clear();
        self.triggers.clear();
        self.camera_present = false;
    }

    pub fn set_camera(&mut self, camera: &CameraParams) {
        self.camera = camera.clone();
        self.camera_present = true;
    }

    pub fn add_description(&mut self, name: &str, description: &EffectDescription) -> usize {
        let name = name.to_ascii_lowercase();
        let slot = self
            .names
            .iter()
            .find_map(|(n, i)| (n == &name && self.descriptions[*i].priority == description.priority).then_some(*i));
        let mut copy = description.clone();
        copy.text.make_ascii_lowercase();
        for component in &mut copy.components {
            if (component.index as i32) >= 0 {
                continue;
            }
            component.name.make_ascii_lowercase();
            let index = if component.kind == 2 {
                Some(0)
            } else if matches!(component.kind, 0 | 1 | 3..=16) {
                self.components
                    .get(&component.kind)
                    .and_then(|m| m.get(&component.name))
                    .copied()
            } else {
                None
            };
            if let Some(index) = index {
                component.index = index as u32;
            }
        }
        for binding in &mut copy.bindings {
            binding.name.make_ascii_lowercase();
            let table = if binding.index & 1 != 0 {
                Some(&self.shakes)
            } else if binding.index & 12 != 0 {
                Some(&self.lights)
            } else {
                None
            };
            if let Some(index) = table.and_then(|m| m.get(&binding.name)) {
                binding.extra = *index;
            }
        }
        if let Some(index) = slot {
            self.descriptions[index] = copy;
            index
        } else {
            let index = self.descriptions.len();
            self.descriptions.push(copy);
            self.names.push((name, index));
            index
        }
    }

    /// No destination write on miss; None is an existence query. This lookup
    /// intentionally has no manager max-priority filter (§18.6).
    pub fn get_description(&self, name: &str, out: Option<&mut EffectDescription>) -> bool {
        let name = name.to_ascii_lowercase();
        let Some((_, index)) = self.names.iter().find(|(n, _)| n == &name) else {
            return false;
        };
        if let Some(out) = out {
            *out = self.descriptions[*index].clone();
        }
        true
    }

    pub fn add_binding(&mut self, name: &str, group: u32, instance: u32) {
        self.bindings.push((name.to_owned(), group, instance));
    }
    pub fn add_trigger(&mut self, message: u32, name: &str) {
        self.triggers.push((message, name.to_owned()));
    }

    pub fn end(&mut self, mut messages: Option<&mut dyn Messages>) {
        self.manager_camera = self.camera_present.then(|| self.camera.clone());
        for (name, group, instance) in &self.bindings {
            // Native name-map traversal determines the winner for multiple
            // priorities; the explicit order is retained in `names`.
            for (entry, index) in &self.names {
                if entry == name {
                    self.ids.entry((*group, *instance)).or_insert(*index);
                }
            }
        }
        for (index, (id, _)) in self.triggers.iter().enumerate() {
            self.trigger_lookup.insert(*id, index);
            if let Some(messages) = messages.as_deref_mut() {
                messages.subscribe(*id);
            }
        }
    }
}

#[cfg(test)]
mod tests;
