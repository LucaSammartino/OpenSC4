// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable owners from model-instances §§1–6,10. No native layouts or game services.
//! References own their pointees, and shared getters use RwLock rather than RefCell.
mod animation;
mod geometry;
mod instance;
mod processor;
mod resources;

pub use animation::*;
pub use geometry::*;
pub use instance::*;
pub use processor::*;
pub use resources::*;

use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

struct Allocation<T> {
    count: AtomicU16,
    value: RwLock<T>,
}

/// A counted owned reference. Dropping the final reference destroys the entire owner.
/// MI10 [choice]: a newly returned Rust handle already owns one reference. The published
/// 16-bit counter wraps; Arc keeps overflow from causing a memory safety failure.
pub struct Reference<T>(Arc<Allocation<T>>, bool);

impl<T> Reference<T> {
    pub fn new(value: T) -> Self {
        Self(
            Arc::new(Allocation {
                count: AtomicU16::new(1),
                value: RwLock::new(value),
            }),
            true,
        )
    }
    pub fn reference_count(&self) -> u16 {
        self.0.count.load(Ordering::Relaxed)
    }
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.value.read().unwrap_or_else(|e| e.into_inner())
    }
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.value.write().unwrap_or_else(|e| e.into_inner())
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn release(mut self) -> u16 {
        self.1 = false;
        let count = self.0.count.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
        drop(self);
        count
    }
}
impl<T> Clone for Reference<T> {
    fn clone(&self) -> Self {
        self.0.count.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(&self.0), true)
    }
}
impl<T> Drop for Reference<T> {
    fn drop(&mut self) {
        if self.1 {
            self.0.count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
impl<T> std::fmt::Debug for Reference<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reference")
            .field("count", &self.reference_count())
            .finish_non_exhaustive()
    }
}

pub const IID_UNKNOWN: u32 = 1;
pub const IID_BASIC: u32 = 0x26A68449;
pub const IID_MESH_CONTAINER: u32 = 0x26A68448;
pub const IID_ANIMATED: u32 = 0xA818EDD8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfacePart {
    Basic,
    MeshContainer,
    Animated,
    Resource,
}

pub trait Interfaces {
    fn interface(&self, iid: u32) -> Option<InterfacePart>;
}
pub struct InterfaceReference<T> {
    pub owner: Reference<T>,
    pub part: InterfacePart,
}
impl<T: Interfaces> Reference<T> {
    pub fn query_interface(&self, iid: u32) -> Option<InterfaceReference<T>> {
        let part = self.read().interface(iid)?;
        Some(InterfaceReference {
            owner: self.clone(),
            part,
        })
    }
}

#[cfg(test)]
mod tests;
