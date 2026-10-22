// SPDX-License-Identifier: GPL-3.0-or-later
//! Published operations of the eight-owner subset in model-instances §0.1.
//! Excludes animated convenience slots 0–8, processor assignment, generic
//! persistence and sprite-instance binding/rendering. No whole-owner claim.
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
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountPolicy {
    Basic16,
    Instance32,
    Resource32,
    Factory32,
    /// Foreign services/vertex blocks have no count contract in this spec.
    Portable32,
}
pub trait ReferenceTarget {
    const COUNT_POLICY: CountPolicy;
}
impl<T: ?Sized> ReferenceTarget for Box<T> {
    const COUNT_POLICY: CountPolicy = CountPolicy::Portable32;
}
impl<T> ReferenceTarget for Vec<T> {
    const COUNT_POLICY: CountPolicy = CountPolicy::Portable32;
}
impl ReferenceTarget for VertexBlock {
    const COUNT_POLICY: CountPolicy = CountPolicy::Portable32;
}
impl ReferenceTarget for BasicInstance {
    const COUNT_POLICY: CountPolicy = CountPolicy::Basic16;
}
impl ReferenceTarget for StaticInstance {
    const COUNT_POLICY: CountPolicy = CountPolicy::Instance32;
}
impl ReferenceTarget for AnimatedInstance {
    const COUNT_POLICY: CountPolicy = CountPolicy::Instance32;
}
impl ReferenceTarget for SpriteResource {
    const COUNT_POLICY: CountPolicy = CountPolicy::Resource32;
}
impl ReferenceTarget for SpriteAnimation {
    const COUNT_POLICY: CountPolicy = CountPolicy::Resource32;
}
impl ReferenceTarget for SpriteAnimationTable {
    const COUNT_POLICY: CountPolicy = CountPolicy::Resource32;
}
impl ReferenceTarget for SpriteAnimationFactory {
    const COUNT_POLICY: CountPolicy = CountPolicy::Factory32;
}

/// MI10 native count state, independent of the safety lifetime of Rust handles.
pub struct NativeCount {
    count: AtomicU32,
    destroyed: AtomicBool,
    policy: CountPolicy,
    factory_lock: Mutex<()>,
}
impl NativeCount {
    pub fn new(policy: CountPolicy) -> Self {
        Self {
            count: AtomicU32::new(0),
            destroyed: AtomicBool::new(false),
            policy,
            factory_lock: Mutex::new(()),
        }
    }
    pub fn get(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }
    pub fn is_destroyed(&self) -> bool {
        self.destroyed.load(Ordering::SeqCst)
    }
    // Keep the atomic update compatible with the crate's Rust 1.75 MSRV.
    fn update_count(&self, operation: impl Fn(u32) -> u32) -> u32 {
        let mut previous = self.count.load(Ordering::SeqCst);
        loop {
            match self
                .count
                .compare_exchange_weak(previous, operation(previous), Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(old) => return old,
                Err(current) => previous = current,
            }
        }
    }
    pub fn add_ref(&self) -> u32 {
        let _lock = (self.policy == CountPolicy::Factory32)
            .then(|| self.factory_lock.lock().unwrap_or_else(|e| e.into_inner()));
        assert!(!self.is_destroyed(), "AddRef after native destruction");
        let mask = if self.policy == CountPolicy::Basic16 {
            0xFFFF
        } else {
            u32::MAX
        };
        let old = self.update_count(|n| n.wrapping_add(1) & mask);
        old.wrapping_add(1) & mask
    }
    pub fn release(&self) -> u32 {
        let _lock = (self.policy == CountPolicy::Factory32)
            .then(|| self.factory_lock.lock().unwrap_or_else(|e| e.into_inner()));
        assert!(!self.is_destroyed(), "Release after native destruction");
        let resource = self.policy == CountPolicy::Resource32;
        let old = self.update_count(|n| {
            if resource {
                n.wrapping_sub(1)
            } else {
                n.saturating_sub(1)
            }
        });
        let result = if resource {
            old.wrapping_sub(1)
        } else {
            old.saturating_sub(1)
        };
        if result == 0 {
            self.destroyed.store(true, Ordering::SeqCst);
        }
        result
    }
}
struct Allocation<T> {
    count: NativeCount,
    value: RwLock<Option<T>>,
}

/// Counted handle: `new` returns count 1; `constructed` exposes native count 0.
/// [choice] Arc protects memory only. Native destruction removes the value, so
/// overflow never gives surviving handles access to a natively destroyed owner.
/// Dropping the last Rust handle also reclaims memory, even if native resource
/// underflow would leak it. NativeCount separately exposes that native state.
pub struct Reference<T>(Arc<Allocation<T>>, bool);

impl<T: ReferenceTarget> Reference<T> {
    pub fn constructed(value: T) -> Self {
        Self(
            Arc::new(Allocation {
                count: NativeCount::new(T::COUNT_POLICY),
                value: RwLock::new(Some(value)),
            }),
            false,
        )
    }
    pub fn new(value: T) -> Self {
        let mut result = Self::constructed(value);
        result.0.count.add_ref();
        result.1 = true;
        result
    }
}
pub struct ReferenceRead<'a, T>(RwLockReadGuard<'a, Option<T>>);
impl<T> std::ops::Deref for ReferenceRead<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("access after native destruction")
    }
}
pub struct ReferenceWrite<'a, T>(RwLockWriteGuard<'a, Option<T>>);
impl<T> std::ops::Deref for ReferenceWrite<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("access after native destruction")
    }
}
impl<T> std::ops::DerefMut for ReferenceWrite<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("access after native destruction")
    }
}
pub(crate) struct BorrowedReference<T>(Weak<Allocation<T>>);
impl<T> BorrowedReference<T> {
    pub(crate) fn access(&self) -> Option<Reference<T>> {
        let allocation = self.0.upgrade()?;
        if allocation.count.is_destroyed() {
            return None;
        }
        Some(Reference(allocation, false))
    }
}
impl<T> Reference<T> {
    pub fn reference_count(&self) -> u32 {
        self.0.count.get()
    }
    pub fn read(&self) -> ReferenceRead<'_, T> {
        ReferenceRead(self.0.value.read().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn write(&self) -> ReferenceWrite<'_, T> {
        ReferenceWrite(self.0.value.write().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn downgrade(&self) -> BorrowedReference<T> {
        BorrowedReference(Arc::downgrade(&self.0))
    }
    fn release_native(&self) -> u32 {
        let count = self.0.count.release();
        if self.0.count.is_destroyed() {
            let value = self.0.value.write().unwrap_or_else(|e| e.into_inner()).take();
            drop(value);
        }
        count
    }
    pub fn release(mut self) -> u32 {
        self.1 = false;
        self.release_native()
    }
}
impl<T> Clone for Reference<T> {
    fn clone(&self) -> Self {
        self.0.count.add_ref();
        Self(Arc::clone(&self.0), true)
    }
}
impl<T> Drop for Reference<T> {
    fn drop(&mut self) {
        if self.1 && !self.0.count.is_destroyed() {
            self.release_native();
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
    pub fn query_interface_into(&self, iid: u32, output: &mut Option<InterfaceReference<T>>) -> bool {
        let Some(found) = self.query_interface(iid) else {
            return false;
        };
        *output = Some(found);
        true
    }
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
