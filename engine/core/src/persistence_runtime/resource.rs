// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Key, RESOURCE_IID, UNKNOWN_IID};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    RwLock,
};

/// §4.1. Getter state uses a reader lock, never RefCell. Interlocked lifetime
/// is distinct from record/factory destroy-at-entry behaviour.
pub struct Resource {
    references: AtomicU32,
    key: RwLock<Key>,
    ty: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceInterface {
    Resource,
    BufferResource,
    Buffer,
}
impl Resource {
    pub fn new(ty: u32) -> Self {
        Self {
            references: AtomicU32::new(0),
            key: RwLock::new(Key::default()),
            ty,
        }
    }
    pub fn reference_count(&self) -> u32 {
        self.references.load(Ordering::SeqCst)
    }
    pub fn add_ref(&self) -> u32 {
        self.references
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1)
    }
    pub fn release(&self) -> u32 {
        self.references
            .fetch_sub(1, Ordering::SeqCst)
            .wrapping_sub(1)
    }
    pub fn query_interface(&self, iid: u32) -> bool {
        if iid == RESOURCE_IID || iid == UNKNOWN_IID {
            self.add_ref();
            true
        } else {
            false
        }
    }
    pub fn key(&self) -> Key {
        *self.key.read().unwrap_or_else(|e| e.into_inner())
    }
    pub fn set_key(&self, key: Key) {
        *self.key.write().unwrap_or_else(|e| e.into_inner()) = key;
    }
    pub fn resource_type(&self) -> u32 {
        self.ty
    }
}
