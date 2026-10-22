// SPDX-License-Identifier: GPL-3.0-or-later

//! Portable cSC4FeatureManager, docs/specs/feature-manager.md §§1–4, 7, 10.
//! A [`FeatureProcess`] represents one process's static storage, explicitly shared
//! by its managers. Resource lookup is a foreign primitive; pack decisions, INI
//! traversal, entry application, lifetime and queries remain here.
//!
//! An adapter supplies the highest-priority raw file for each resource key (no
//! resource cache). File bytes must be available for the scan; native streaming
//! FM3 streaming failures cannot be injected by this complete-byte provider. An
//! adapter must refuse an incomplete capture, never silently map it to a miss.

use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use crate::number_text::rule_n;
use crate::rz_files::ini::{parse_line, IniLine};

pub const IID_UNKNOWN: u32 = 1;
pub const IID_FEATURE_MANAGER: u32 = 0x0B56_2673;
pub const PACK_GROUP: u32 = 0x8A59_71C5;
pub const FIRST_PACK_INSTANCE: u32 = 0x2B56_3701;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub type_id: u32,
    pub group_id: u32,
    pub instance_id: u32,
}

/// Foreign primitives only; no provider can answer a feature or pack query.
pub trait PackResources {
    type File: AsRef<[u8]>;

    /// Register the process's static shutdown callback with context 0. The caller
    /// supplies the app-owned callback binding; the callback calls static_shutdown.
    fn register_static_shutdown(&mut self, process: &FeatureProcess);
    /// Acquire one raw resource file. None means acquisition failed; merely
    /// locating a record does not prove acquisition succeeds (FM3).
    fn open_resource(&mut self, key: ResourceKey) -> Option<Self::File>;
    /// Release precisely the reference returned by open_resource, after parsing.
    fn release_file(&mut self, file: Self::File);
}

#[derive(Debug, Default)]
struct StaticState {
    scan_done: bool,
    installed_packs: u32,
    enabled_features: BTreeSet<u32>,
    mentioned_features: BTreeSet<u32>,
}

/// Explicit process scope permits isolated tests without altering process-wide
/// semantics: every manager in one application must share this handle.
#[derive(Clone, Debug, Default)]
pub struct FeatureProcess(Arc<RwLock<StaticState>>);

impl FeatureProcess {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static scan (§4.1). The guard is published before the registration callback;
    /// no lock is held across foreign calls, so callback getters cannot deadlock.
    pub fn scan(&self, resources: &mut impl PackResources) -> bool {
        {
            let mut state = self.0.write().unwrap_or_else(|e| e.into_inner());
            if state.scan_done {
                return true;
            }
            state.scan_done = true;
        }
        resources.register_static_shutdown(self);
        {
            let mut state = self.0.write().unwrap_or_else(|e| e.into_inner());
            state.installed_packs = 0;
            state.enabled_features.clear();
            state.mentioned_features.clear();
        }
        for k in 0..32 {
            let key = ResourceKey {
                type_id: 0,
                group_id: PACK_GROUP,
                instance_id: FIRST_PACK_INSTANCE + k,
            };
            if let Some(file) = resources.open_resource(key) {
                self.0.write().unwrap_or_else(|e| e.into_inner()).installed_packs |= 1 << k;
                self.apply_ini(file.as_ref());
                resources.release_file(file);
            }
        }
        true
    }

    fn apply_ini(&self, bytes: &[u8]) {
        let mut in_features = false;
        for line in bytes.split(|&b| b == b'\r' || b == b'\n') {
            match parse_line(line) {
                IniLine::Header(name) => {
                    if in_features {
                        break; // Only the first matching section, never merge repeats.
                    }
                    in_features = name.is_some_and(|n| n.eq_ignore_ascii_case(b"Features"));
                }
                IniLine::Entry { key, value } if in_features && !key.is_empty() && !value.is_empty() => {
                    self.apply_entry(&key, &value);
                }
                _ => {}
            }
        }
    }

    /// Entry callback (§4.2), with the INI reader's already-trimmed nonempty inputs.
    /// Its native return is ignored; the portable callback has no result.
    pub fn apply_entry(&self, key: &[u8], value: &[u8]) {
        let id = rule_n(key);
        let mut state = self.0.write().unwrap_or_else(|e| e.into_inner());
        state.mentioned_features.insert(id);
        if value.eq_ignore_ascii_case(b"off") {
            state.enabled_features.remove(&id);
        } else {
            state.enabled_features.insert(id);
        }
    }

    /// Application callback (§3): no unregister call and no object-state changes.
    pub fn static_shutdown(&self) -> bool {
        let mut state = self.0.write().unwrap_or_else(|e| e.into_inner());
        if state.scan_done {
            state.scan_done = false;
            state.installed_packs = 0;
            state.enabled_features.clear();
            state.mentioned_features.clear();
        }
        true
    }

    pub fn is_feature_enabled(&self, id: u32) -> bool {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .enabled_features
            .contains(&id)
    }

    /// Verification query domain: every parsed key, including disabled IDs.
    /// Collected during the single scan, independently of native query answers.
    pub fn mentioned_features(&self) -> Vec<u32> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .mentioned_features
            .iter()
            .copied()
            .collect()
    }

    pub fn is_expansion_pack_installed(&self, n: u32) -> bool {
        let bit = 1u32 << (n.wrapping_sub(1) & 31);
        self.0.read().unwrap_or_else(|e| e.into_inner()).installed_packs & bit != 0
    }
}

/// The slot-7 algorithm calls slot 6 on this same object in descending order.
/// An implementation overriding slot 6 inherits this algorithm unchanged.
pub trait ExpansionPackQuery {
    fn is_expansion_pack_installed(&self, n: u32) -> bool;

    fn get_current_expansion_pack(&self) -> u32 {
        (1..=32)
            .rev()
            .find(|&n| self.is_expansion_pack_installed(n))
            .unwrap_or(0)
    }
}

/// Inert identity token, not a native pointer or an owning GZCOM reference.
#[derive(Clone, Debug)]
pub struct FeatureInterface(Arc<()>);

impl PartialEq for FeatureInterface {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FeatureInterface {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deletion {
    pub returned_this: FeatureInterface,
    pub allocation_freed: bool,
    pub count_at_destruction: i32,
    pub initialized_at_destruction: bool,
}

#[derive(Debug, Default)]
struct ObjectState {
    initialized: bool,
    references: i32,
}

/// Complete portable owner. Option results distinguish terminal Rust handles from
/// live native calls; use after native deletion is outside the published domain.
/// Deletion retains an observation, never a usable interface pointer. Flag-bit-0
/// destruction drops the object allocation; other flags retain it until Rust drop.
#[derive(Debug)]
pub struct FeatureManager {
    process: FeatureProcess,
    object: Option<Box<ObjectState>>,
    identity: FeatureInterface,
    deletion: Option<Deletion>,
}

impl FeatureManager {
    /// Constructor: initialized=false and signed reference count=0 (§2).
    pub fn new(process: FeatureProcess) -> Self {
        Self {
            process,
            object: Some(Box::default()),
            identity: FeatureInterface(Arc::new(())),
            deletion: None,
        }
    }

    fn object(&self) -> Option<&ObjectState> {
        if self.deletion.is_some() {
            None
        } else {
            self.object.as_deref()
        }
    }

    fn object_mut(&mut self) -> Option<&mut ObjectState> {
        if self.deletion.is_some() {
            None
        } else {
            self.object.as_deref_mut()
        }
    }

    /// Borrowed identity, analogous to the app getter: does not AddRef.
    pub fn interface(&self) -> Option<FeatureInterface> {
        self.object()?;
        Some(self.identity.clone())
    }

    pub fn reference_count(&self) -> Option<i32> {
        Some(self.object()?.references)
    }

    pub fn initialized(&self) -> Option<bool> {
        Some(self.object()?.initialized)
    }

    /// Slot 0: supported IIDs return this exact same identity and AddRef once.
    /// Unsupported IDs leave the output cell entirely untouched.
    pub fn query_interface(&mut self, iid: u32, output: &mut Option<FeatureInterface>) -> bool {
        if !matches!(iid, IID_UNKNOWN | IID_FEATURE_MANAGER) || self.object().is_none() {
            return false;
        }
        *output = Some(self.identity.clone());
        self.add_ref();
        true
    }

    /// Slot 1: return the incremented count's 32-bit register representation.
    pub fn add_ref(&mut self) -> Option<u32> {
        let state = self.object_mut()?;
        // FM4: preserve the incremented register bits; Release compares signed.
        state.references = state.references.wrapping_add(1);
        Some(state.references as u32)
    }

    /// Slot 2: signed >1 decrements; all other counts run Shutdown then delete,
    /// without decrementing the count observed by the destructor.
    pub fn release(&mut self) -> Option<u32> {
        let state = self.object_mut()?;
        if state.references > 1 {
            state.references -= 1;
            return Some(state.references as u32);
        }
        self.shutdown();
        self.deleting_destructor(1)?;
        Some(0)
    }

    pub fn init(&mut self, resources: &mut impl PackResources) -> Option<bool> {
        let state = self.object_mut()?;
        if !state.initialized {
            state.initialized = true;
            self.process.scan(resources);
        }
        Some(true)
    }

    pub fn shutdown(&mut self) -> Option<bool> {
        self.object_mut()?.initialized = false;
        Some(true)
    }

    pub fn is_feature_enabled(&self, id: u32) -> Option<bool> {
        self.object()?;
        Some(self.process.is_feature_enabled(id))
    }

    pub fn is_expansion_pack_installed(&self, n: u32) -> Option<bool> {
        self.object()?;
        Some(self.process.is_expansion_pack_installed(n))
    }

    pub fn get_current_expansion_pack(&self) -> Option<u32> {
        self.object()?;
        Some(ExpansionPackQuery::get_current_expansion_pack(self))
    }

    /// Slot 8 alone does not call Shutdown or change static state (§3).
    pub fn deleting_destructor(&mut self, flags: u32) -> Option<Deletion> {
        let state = self.object()?;
        let deletion = Deletion {
            returned_this: self.identity.clone(),
            allocation_freed: flags & 1 != 0,
            count_at_destruction: state.references,
            initialized_at_destruction: state.initialized,
        };
        if deletion.allocation_freed {
            drop(self.object.take());
        }
        self.deletion = Some(deletion.clone());
        Some(deletion)
    }

    pub fn deletion(&self) -> Option<&Deletion> {
        self.deletion.as_ref()
    }
}

impl ExpansionPackQuery for FeatureManager {
    fn is_expansion_pack_installed(&self, n: u32) -> bool {
        self.process.is_expansion_pack_installed(n)
    }
}

#[cfg(test)]
mod tests;
