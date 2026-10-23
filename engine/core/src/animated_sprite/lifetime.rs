// SPDX-License-Identifier: GPL-3.0-or-later
//! Â§1.1 reference semantics in a safe arena (no native pointer ABI).

use super::{AnimatedSprite, BASIC_INSTANCE_IID, SPRITE_INSTANCE_IID};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpriteHandle(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceError {
    Missing,
    ZeroInitialCount,
    Overflow,
    UnsupportedInterface,
    HandlesExhausted,
}

#[derive(Debug)]
struct Entry<O> {
    references: u16,
    sprite: AnimatedSprite<O>,
}
#[derive(Debug)]
pub struct SpriteArena<O> {
    next: u64,
    entries: BTreeMap<SpriteHandle, Entry<O>>,
}

impl<O> Default for SpriteArena<O> {
    fn default() -> Self {
        Self {
            next: 0,
            entries: BTreeMap::new(),
        }
    }
}

impl<O> SpriteArena<O> {
    /// Explicit count avoids guessing constructor state. Arena handles are
    /// monotonic so a stale handle cannot acquire a later instance's reference.
    pub fn insert(&mut self, sprite: AnimatedSprite<O>, references: u16) -> Result<SpriteHandle, ReferenceError> {
        let next = self.next.checked_add(1).ok_or(ReferenceError::HandlesExhausted)?;
        let handle = SpriteHandle(self.next);
        self.next = next;
        self.entries.insert(handle, Entry { references, sprite });
        Ok(handle)
    }

    pub fn get(&self, handle: SpriteHandle) -> Option<&AnimatedSprite<O>> {
        self.entries.get(&handle).map(|e| &e.sprite)
    }
    pub fn get_mut(&mut self, handle: SpriteHandle) -> Option<&mut AnimatedSprite<O>> {
        self.entries.get_mut(&handle).map(|e| &mut e.sprite)
    }
    pub fn references(&self, handle: SpriteHandle) -> Option<u16> {
        self.entries.get(&handle).map(|e| e.references)
    }

    pub fn add_ref(&mut self, handle: SpriteHandle) -> Result<u16, ReferenceError> {
        let entry = self.entries.get_mut(&handle).ok_or(ReferenceError::Missing)?;
        entry.references = entry.references.wrapping_add(1);
        Ok(entry.references)
    }

    pub fn query_basic_interface(&mut self, handle: SpriteHandle, iid: u32) -> Result<SpriteHandle, ReferenceError> {
        if iid != 1 && iid != BASIC_INSTANCE_IID && iid != SPRITE_INSTANCE_IID {
            return Err(ReferenceError::UnsupportedInterface);
        }
        self.add_ref(handle)?;
        Ok(handle)
    }

    pub fn release(&mut self, handle: SpriteHandle) -> Result<u16, ReferenceError> {
        let entry = self.entries.get_mut(&handle).ok_or(ReferenceError::Missing)?;
        let count = entry.references.saturating_sub(1);
        if count == 0 {
            self.entries.remove(&handle);
        } else {
            entry.references = count;
        }
        Ok(count)
    }
}
