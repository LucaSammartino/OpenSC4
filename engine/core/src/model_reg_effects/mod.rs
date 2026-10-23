// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4ModelRegEffects, graphics-formats §3.11 and §8.5 vectors 48–49.
//! REGP parsing belongs to formats::s3d. Model data, effects, and the current
//! CRT byte-lowercase table are foreign inputs; registration/control lives here.

use std::collections::BTreeMap;
use std::sync::Arc;

mod transform;
pub use transform::{Rotation, Transform, Vector3};
#[cfg(test)]
mod tests;

pub type Regions = BTreeMap<String, Vec<[f32; 7]>>;
pub const MODEL_EFFECT_TRANSFORM_FLAGS: u8 = 5;
pub const MOTION_PARAMETER: u32 = 15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceFrame {
    pub transform: Transform,
    pub flags: u8,
    /// None models failed animated-interface QI (frame zero). A provider must
    /// release its temporary QI reference before returning this snapshot.
    pub animated_frame: Option<u16>,
}

/// The attachment Arc owns one acquired instance reference. Missing model or
/// container is None. Returned maps/vectors must stay stable while attached.
/// Getters borrow them without acquiring a model resource.
pub trait ModelInstance: Send + Sync {
    fn regions(&self) -> Option<&Regions>;
    fn frame(&self) -> InstanceFrame;
}

/// A Box owns the created reference without AddRef. Query foreign started state,
/// including after deferred Stop(0); the owner never caches it.
pub trait VisualEffect: Send + Sync {
    fn is_started(&self) -> bool;
    fn start(&mut self, control: u32);
    fn stop(&mut self, control: u32);
    fn set_transform(&mut self, transform: Transform, flags: u8, version: u8);
    fn set_parameter(&mut self, parameter: u32, value: Vector3, count: u32);
}

/// Boolean and output pointer are independent native creation outputs.
pub struct Creation {
    pub success: bool,
    pub effect: Option<Box<dyn VisualEffect>>,
}

pub trait EffectFactory {
    /// Obtain the foreign manager before entry mutation. AddEffect requires it;
    /// RecreateEffectsNamed alone handles its absence gracefully.
    fn acquire_manager(&mut self) -> bool;
    fn create(&mut self, name: &str) -> Creation;
}

enum Records<'a> {
    Null,
    Borrowed(&'a [[f32; 7]]),
    // Borrowed-vector locator, not a copy or an extra acquired model reference.
    Region(String),
}

pub struct EffectEntry<'a> {
    name: String,
    key: String,
    records: Records<'a>,
    effect: Option<Box<dyn VisualEffect>>,
}

impl EffectEntry<'_> {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn effect(&self) -> Option<&dyn VisualEffect> {
        self.effect.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingRecord {
    pub key: String,
    pub frame: usize,
    pub record_count: usize,
}

/// Missing records diagnose undefined native indexing, not a native false result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateReport {
    pub success: bool,
    pub transformed: usize,
    pub missing_records: Vec<MissingRecord>,
}

impl Default for UpdateReport {
    fn default() -> Self {
        Self {
            success: true,
            transformed: 0,
            missing_records: Vec::new(),
        }
    }
}

pub struct ModelRegEffects<'a> {
    instance: Option<Arc<dyn ModelInstance>>,
    entries: BTreeMap<String, EffectEntry<'a>>,
    lowercase: [u8; 256],
}

impl Default for ModelRegEffects<'_> {
    fn default() -> Self {
        let mut lowercase = [0; 256];
        for (byte, mapped) in lowercase.iter_mut().enumerate() {
            *mapped = (byte as u8).to_ascii_lowercase();
        }
        Self {
            instance: None,
            entries: BTreeMap::new(),
            lowercase,
        }
    }
}

impl<'a> ModelRegEffects<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current CRT unsigned-byte tolower mapping; default is the C-locale ASCII
    /// path. Never replace this comparison with Unicode string lower-casing.
    pub fn set_locale_lowercase(&mut self, lowercase: [u8; 256]) {
        self.lowercase = lowercase;
    }

    pub fn set_model_instance(&mut self, instance: Option<Arc<dyn ModelInstance>>) {
        let same = match (&self.instance, &instance) {
            (Some(old), Some(new)) => Arc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.shutdown();
            self.instance = instance;
        }
    }

    pub fn instance(&self) -> Option<&dyn ModelInstance> {
        self.instance.as_deref()
    }

    pub fn init(&mut self, factory: &mut dyn EffectFactory) -> bool {
        if self.instance.is_some() {
            self.init_effects_list(factory);
        }
        true
    }

    pub fn shutdown(&mut self) -> bool {
        self.clear();
        self.instance = None;
        true
    }

    pub fn init_effects_list(&mut self, factory: &mut dyn EffectFactory) {
        // Copy names only; resolve the original immutable vector when needed.
        let keys: Vec<_> = self
            .instance
            .as_ref()
            .and_then(|i| i.regions())
            .into_iter()
            .flat_map(|r| r.keys())
            .filter(|k| effect_name(k).is_some())
            .cloned()
            .collect();
        for key in keys {
            let name = effect_name(&key).unwrap().to_owned();
            self.add_inner(&key, &name, Records::Region(key.clone()), factory);
        }
    }

    pub fn add_effect(
        &mut self,
        key: &str,
        name: &str,
        records: Option<&'a [[f32; 7]]>,
        factory: &mut dyn EffectFactory,
    ) -> Option<&dyn VisualEffect> {
        self.add_inner(key, name, records.map_or(Records::Null, Records::Borrowed), factory)
    }

    fn add_inner(
        &mut self,
        key: &str,
        name: &str,
        records: Records<'a>,
        factory: &mut dyn EffectFactory,
    ) -> Option<&dyn VisualEffect> {
        // Native dereferences an unavailable manager. This explicit core
        // precondition does not fabricate a graceful creation-failure result.
        assert!(factory.acquire_manager(), "AddEffect requires a city effects manager");
        self.remove(key);
        let creation = factory.create(name);
        if !creation.success {
            // No Release of a nonnull output returned alongside false.
            if let Some(effect) = creation.effect {
                std::mem::forget(effect);
            }
            return None;
        }
        self.entries.insert(
            key.to_owned(),
            EffectEntry {
                key: key.to_owned(),
                name: name.to_owned(),
                records,
                effect: creation.effect,
            },
        );
        self.get_effect(key)
    }

    pub fn add_effect_at_region(
        &mut self,
        key: &str,
        name: &str,
        factory: &mut dyn EffectFactory,
    ) -> Option<&dyn VisualEffect> {
        let records = if self
            .instance
            .as_ref()
            .and_then(|i| i.regions())
            .is_some_and(|r| r.contains_key(key))
        {
            Records::Region(key.to_owned())
        } else {
            Records::Null
        };
        self.add_inner(key, name, records, factory)
    }

    pub fn get_effect(&self, key: &str) -> Option<&dyn VisualEffect> {
        self.entries.get(key).and_then(EffectEntry::effect)
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = &EffectEntry<'a>> {
        self.entries.values()
    }

    pub fn records(&self, key: &str) -> Option<&[[f32; 7]]> {
        resolve(&self.entries.get(key)?.records, self.instance.as_deref())
    }

    pub fn start_all(&mut self, control: u32) {
        for effect in self.entries.values_mut().filter_map(|e| e.effect.as_mut()) {
            if !effect.is_started() {
                effect.start(control);
            }
        }
    }

    pub fn stop_all(&mut self, control: u32) {
        for effect in self.entries.values_mut().filter_map(|e| e.effect.as_mut()) {
            if effect.is_started() {
                effect.stop(control);
            }
        }
    }

    pub fn start_named(
        &mut self,
        name: &str,
        control: u32,
        create_if_missing: bool,
        factory: &mut dyn EffectFactory,
    ) -> bool {
        let mut matched = false;
        for entry in self.entries.values_mut() {
            if let Some(effect) = entry.effect.as_mut() {
                if names_equal(&self.lowercase, &entry.name, name) {
                    matched = true;
                    effect.start(control);
                }
            }
        }
        matched || self.start_missing(name, control, create_if_missing, factory)
    }

    fn start_missing(&mut self, name: &str, control: u32, create: bool, factory: &mut dyn EffectFactory) -> bool {
        if create {
            self.add_effect_at_region(name, name, factory);
            if let Some(effect) = self.entries.get_mut(name).and_then(|e| e.effect.as_mut()) {
                effect.start(control);
                return true;
            }
        }
        false
    }

    pub fn stop_named(&mut self, name: &str, control: u32) {
        for entry in self.entries.values_mut() {
            if let Some(effect) = entry.effect.as_mut() {
                if effect.is_started() && names_equal(&self.lowercase, &entry.name, name) {
                    effect.stop(control);
                }
            }
        }
    }

    pub fn toggle_named(&mut self, name: &str, control: u32, create_if_missing: bool, factory: &mut dyn EffectFactory) {
        let mut matched = false;
        for entry in self.entries.values_mut() {
            if let Some(effect) = entry.effect.as_mut() {
                if names_equal(&self.lowercase, &entry.name, name) {
                    matched = true;
                    if effect.is_started() {
                        effect.stop(control);
                    } else {
                        effect.start(control);
                    }
                }
            }
        }
        if !matched {
            self.start_missing(name, control, create_if_missing, factory);
        }
    }

    pub fn recreate_named(
        &mut self,
        name: &str,
        control: u32,
        create_if_missing: bool,
        factory: &mut dyn EffectFactory,
    ) {
        if !factory.acquire_manager() {
            return;
        }
        let mut matched = false;
        for entry in self.entries.values_mut() {
            if entry.effect.is_some() && names_equal(&self.lowercase, &entry.name, name) {
                matched = true;
                drop(entry.effect.take()); // Release without Stop.
                let creation = factory.create(name);
                if creation.success {
                    entry.effect = creation.effect;
                    // Native true/null would dereference null on this branch.
                    entry
                        .effect
                        .as_mut()
                        .expect("successful recreation requires an effect")
                        .start(control);
                } else if let Some(effect) = creation.effect {
                    std::mem::forget(effect);
                }
            }
        }
        if !matched {
            self.start_missing(name, control, create_if_missing, factory);
        }
    }

    pub fn update(&mut self) -> UpdateReport {
        self.update_inner(None)
    }

    /// Slot 4's first word is unused. Direction is not normalized or rotated.
    pub fn update_with_motion(&mut self, _unused: usize, direction: Vector3, speed: f32) -> UpdateReport {
        self.update_inner(Some(direction.map(|component| speed * component)))
    }

    fn update_inner(&mut self, motion: Option<Vector3>) -> UpdateReport {
        let mut report = UpdateReport::default();
        let instance = match self.instance.as_deref() {
            Some(instance) if !self.entries.is_empty() => instance,
            _ => return report,
        };
        let frame = instance.frame();
        let index = usize::from(frame.animated_frame.unwrap_or(0));
        for entry in self.entries.values_mut() {
            let Some(effect) = entry.effect.as_mut() else {
                continue;
            };
            if !effect.is_started() {
                continue;
            }
            let (local, local_flags) = match resolve(&entry.records, Some(instance)) {
                None => (Transform::IDENTITY, 0),
                Some(records) => {
                    let Some(record) = records.get(index) else {
                        // §3.11.2: no defined native safe result. Never clamp,
                        // wrap, or use identity for a nonnull vector.
                        report.missing_records.push(MissingRecord {
                            key: entry.key.clone(),
                            frame: index,
                            record_count: records.len(),
                        });
                        continue;
                    };
                    (Transform::from_record(*record), MODEL_EFFECT_TRANSFORM_FLAGS)
                }
            };
            let (world, flags) = if local_flags == 0 {
                (frame.transform, frame.flags)
            } else if frame.flags == 0 {
                (local, local_flags)
            } else {
                (frame.transform.compose(local), frame.flags | local_flags)
            };
            effect.set_transform(world, flags, 0);
            if let Some(value) = motion {
                effect.set_parameter(MOTION_PARAMETER, value, 1);
            }
            report.transformed += 1;
        }
        report
    }

    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(mut entry) = self.entries.remove(key) {
            if let Some(effect) = entry.effect.as_mut() {
                effect.stop(1);
            }
            drop(entry);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        // Null/release all payloads before destroying any node (§3.11.1).
        for entry in self.entries.values_mut() {
            if let Some(mut effect) = entry.effect.take() {
                effect.stop(0);
                drop(effect);
            }
        }
        self.entries.clear();
    }
}

impl Drop for ModelRegEffects<'_> {
    fn drop(&mut self) {
        // Native destruction frees nodes/strings, but does NOT stop/release
        // effects. Clients must explicitly Shutdown for that cleanup sequence.
        for entry in self.entries.values_mut() {
            if let Some(effect) = entry.effect.take() {
                std::mem::forget(effect);
            }
        }
        self.entries.clear();
        self.instance = None;
    }
}

fn resolve<'b>(records: &'b Records<'_>, instance: Option<&'b dyn ModelInstance>) -> Option<&'b [[f32; 7]]> {
    match records {
        Records::Null => None,
        Records::Borrowed(records) => Some(records),
        Records::Region(key) => Some(instance?.regions()?.get(key)?.as_slice()),
    }
}

fn names_equal(lowercase: &[u8; 256], a: &str, b: &str) -> bool {
    a.bytes()
        .take_while(|b| *b != 0)
        .map(|b| lowercase[usize::from(b)])
        .eq(b.bytes().take_while(|b| *b != 0).map(|b| lowercase[usize::from(b)]))
}

/// Vector 48. Input is already lower-cased by the REGP resource reader.
pub fn effect_name(key: &str) -> Option<&str> {
    let name = key.strip_prefix("fx_")?;
    if name.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        if let Some((_, rest)) = name.split_once('_') {
            return Some(rest);
        }
    }
    Some(name)
}
