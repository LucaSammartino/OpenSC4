// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4OccupantGroupTemplate and tKeyEffect bodies, from
//! docs/specs/occupant-groups.md §§1–4,7. This is not the Lua importer or
//! script-system registry. Parent/controller handles do not acquire a COM
//! reference. Native allocation and interface offsets belong to future glue.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, RwLock, Weak};

use crate::rng::{RzRandom, CLOCK_SEED};
use crate::seeding::{clocks, SeedKey};
use crate::stream::ResKey;

pub const OCCUPANT_GROUPS_PROPERTY: u32 = 0xAA1D_D396;
pub const GZCLSID: u32 = 0xA9C7_4DC9;

/// tKeyEffect's three independently owned strings and unmodified flag word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEffect {
    pub fx: String,
    pub bone: String,
    pub controller: String,
    pub flags: u32,
}

impl KeyEffect {
    /// The published copy helper (§7), including unknown flag bits.
    pub fn copy_to(&self, out: &mut Self) {
        out.clone_from(self);
    }
}

/// Non-owning controller-template identity: destruction must not Release it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerHandle(pub u64);

/// Foreign parameter payload only; the group owns and drops the block.
/// Parameter interpretation is the script system's contract (§2).
pub trait ParameterBlock: Send + Sync {}
impl<const N: usize> ParameterBlock for [u8; N] {}

/// Fields filled by the script system. No import ordering or Lua policy is
/// inferred here. Ordinary construction leaves kph unset, literally (§2).
pub struct GroupState {
    pub parent: Weak<OccupantGroupTemplate>,
    pub group_id: u32,
    pub class_id: u32,
    pub percentage: f32,
    pub controllers: Vec<ControllerHandle>,
    pub anims: BTreeMap<String, u32>,
    pub models: Vec<ResKey>,
    pub own_ref: i16,
    pub parent_ref: i16,
    pub trailer_ids: Vec<u32>,
    pub trailer_names: Vec<String>,
    pub kph: Option<(f32, f32)>,
    pub drive_params: Option<Box<dyn ParameterBlock>>,
    pub player_drive_params: Option<Box<dyn ParameterBlock>>,
    pub csi_image: u32,
    pub source_building: u32,
    pub key_effects: BTreeMap<u8, KeyEffect>,
}

impl Default for GroupState {
    fn default() -> Self {
        Self {
            parent: Weak::new(),
            group_id: 0,
            class_id: 0,
            percentage: 1.0,
            controllers: Vec::new(),
            anims: BTreeMap::new(),
            models: Vec::new(),
            own_ref: -2,
            parent_ref: -2,
            trailer_ids: Vec::new(),
            trailer_names: Vec::new(),
            kph: None,
            drive_params: None,
            player_drive_params: None,
            csi_image: 0,
            source_building: 0,
            key_effects: BTreeMap::new(),
        }
    }
}

impl GroupState {
    fn destroy(&mut self) {
        // §4.8's order, rather than Rust's declaration-order field drops.
        self.drive_params.take();
        self.player_drive_params.take();
        self.key_effects.clear();
        self.trailer_ids.clear();
        self.trailer_names.clear();
        self.models.clear();
        self.anims.clear();
        self.controllers.clear();
    }
}

/// Primitive property access, never a delegated group/filter answer.
pub trait OccupantProperties {
    fn u32_property(&self, id: u32) -> Option<Vec<u32>>;
}

/// Parent links are weak because script-system containers, not children,
/// hold references (§3.2). The host must retain every live parent.
/// Chains must be acyclic: the published walks have no cycle-stop policy.
pub struct OccupantGroupTemplate {
    state: RwLock<GroupState>,
    id_list: Mutex<Vec<u32>>,
    session_seed: u32,
    draws: AtomicU32,
    references: AtomicU32,
    destroyed: AtomicBool,
}

impl OccupantGroupTemplate {
    /// Constructor (§§2,4.8). IDs are assigned subsequently by the importer.
    pub fn new(session_seed: u32) -> Self {
        Self {
            state: RwLock::new(GroupState::default()),
            id_list: Mutex::new(Vec::new()),
            session_seed,
            draws: AtomicU32::new(0),
            // Literal shared-base initial zero (occupant-manager §4.4).
            // OG4 requests an explicit group-base confirmation.
            references: AtomicU32::new(0),
            destroyed: AtomicBool::new(false),
        }
    }

    /// Import-time state access. Does not invalidate the ID cache (§4.6).
    /// Callbacks must not reenter this object while holding its lock.
    pub fn edit<R>(&self, f: impl FnOnce(&mut GroupState) -> R) -> R {
        f(&mut self.state.write().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn inspect<R>(&self, f: impl FnOnce(&GroupState) -> R) -> R {
        f(&self.state.read().unwrap_or_else(|e| e.into_inner()))
    }

    /// §4.1: read the property even for ID zero, and make a fresh generator
    /// only after matching the object's own ID and testing percentage >= 1.
    pub fn is_occupant_included(&self, occupant: &dyn OccupantProperties) -> bool {
        let groups = occupant.u32_property(OCCUPANT_GROUPS_PROPERTY);
        self.is_occupant_included_with_draw(groups.as_deref(), || {
            let n = self.draws.fetch_add(1, Ordering::Relaxed);
            let seeds = clocks(self.session_seed, SeedKey::Temporary { clsid: GZCLSID, n });
            RzRandom::new(CLOCK_SEED, &seeds).uniform_f64()
        })
    }

    /// Same body with an injected uniform draw primitive (§10.7). The
    /// comparison stays f64, including equality, NaN and out-of-range shares.
    pub fn is_occupant_included_with_draw(
        &self,
        groups: Option<&[u32]>,
        draw: impl FnOnce() -> f64,
    ) -> bool {
        let (id, percentage) = self.inspect(|s| (s.group_id, s.percentage));
        if id == 0 || !groups.is_some_and(|ids| ids.contains(&id)) {
            return false;
        }
        if percentage >= 1.0 {
            return true;
        }
        f64::from(percentage) >= draw()
    }

    pub fn is_occupant_type_included(&self, _kind: u32) -> bool {
        true
    }

    pub fn is_property_holder_included<T: ?Sized>(&self, _holder: &T) -> bool {
        true
    }

    /// Visit state here, without delegating an ancestor's owner-output method.
    fn walk(&self, mut visit: impl FnMut(&GroupState) -> bool) {
        let next = self.inspect(|s| (visit(s), s.parent.upgrade()));
        let mut parent = if next.0 { next.1 } else { None };
        while let Some(group) = parent {
            let next = group.inspect(|s| (visit(s), s.parent.upgrade()));
            parent = if next.0 { next.1 } else { None };
        }
    }

    pub fn is_occupant_group(&self, id: u32) -> bool {
        let mut found = false;
        self.walk(|s| {
            found = s.group_id == id;
            !found
        });
        found
    }

    pub fn get_class_id(&self) -> u32 {
        let mut result = 0;
        self.walk(|s| {
            result = s.class_id;
            result == 0
        });
        result
    }

    pub fn get_source_building_group(&self) -> u32 {
        let mut result = 0;
        self.walk(|s| {
            result = s.source_building;
            result == 0
        });
        result
    }

    pub fn get_group_hierarchy_depth(group: Option<&Self>) -> u32 {
        let mut count = 0u32;
        if let Some(group) = group {
            group.walk(|_| {
                count = count.wrapping_add(1);
                true
            });
        }
        count
    }

    /// Portable snapshot of the variant's u32 array. The internal cache
    /// persists and is never refreshed once nonempty (§4.6).
    pub fn get_group_id_variant(&self) -> Vec<u32> {
        let mut ids = self.id_list.lock().unwrap_or_else(|e| e.into_inner());
        if ids.is_empty() {
            self.walk(|s| {
                if s.group_id != 0 {
                    ids.push(s.group_id);
                }
                true
            });
        }
        ids.clone()
    }

    pub fn get_key_effect(&self, command: u32, out: &mut KeyEffect) -> bool {
        self.inspect(|s| match s.key_effects.get(&(command as u8)) {
            Some(effect) => {
                effect.copy_to(out);
                true
            }
            None => false,
        })
    }

    /// Published map-clear helper (§7). No ancestor fallback.
    pub fn clear_key_effects(&self) {
        self.edit(|s| s.key_effects.clear());
    }

    /// Shared AddRef body: unsigned wrapping increment (public SDK).
    pub fn add_ref(&self) -> u32 {
        self.references.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }

    /// §1: unlike a saturating decrement, zero also destroys. Rust retains
    /// an inert shell for observation; hosts must not use it after release 0.
    pub fn release(&self) -> u32 {
        let old = self.references.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            Some(if n <= 1 { 0 } else { n - 1 })
        }).unwrap();
        if old > 1 {
            old - 1
        } else {
            if !self.destroyed.swap(true, Ordering::AcqRel) {
                self.edit(GroupState::destroy);
                self.id_list.lock().unwrap_or_else(|e| e.into_inner()).clear();
            }
            0
        }
    }
}

impl Drop for OccupantGroupTemplate {
    fn drop(&mut self) {
        self.state.get_mut().unwrap_or_else(|e| e.into_inner()).destroy();
        self.id_list.get_mut().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

/// Portable deleting-destructor protocol: with bit 0 clear the allocation
/// remains as a destroyed shell; with it set, ownership of that shell ends.
pub fn deleting_destroy(allocation: &mut Option<Box<Option<OccupantGroupTemplate>>>, flags: u32) {
    if let Some(shell) = allocation.as_mut() {
        shell.take();
    }
    if flags & 1 != 0 {
        allocation.take();
    }
}

#[cfg(test)]
mod tests;
