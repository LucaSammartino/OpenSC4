// SPDX-License-Identifier: GPL-3.0-or-later
//! Foreign interface primitives only. No manager decisions are supplied here.
use super::{CacheId, Filter, Key, KeyList};

/// Symbolic foreign pointer identity; Rust copies do not AddRef/Release it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Object(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lock { Factory, Segment, CacheRegistry, Key, CacheEntries(CacheId), CacheReferences(CacheId) }

/// The lock events identify the recursive native critical-section scopes.
/// Portable calls require exclusive mutable access, and getters borrow state;
/// no RefCell or racing mutable getters are used. A native embedding must map
/// these scopes to its recursive locks. Stable registries during callbacks are
/// the spec's precondition, not a promise of reentrant-mutation safety.
///
/// Primitive implementations must honor referenced outputs on success, and may
/// mutate outputs on failure where their own interface contract permits it.
pub trait Primitives {
    fn lock(&mut self, _lock: Lock, _enter: bool) {}
    fn add_ref(&mut self, object: Object) -> u32;
    fn release(&mut self, object: Object) -> u32;
    fn resource_key(&mut self, resource: Object) -> Key;
    fn resource_set_key(&mut self, resource: Object, key: Key);
    fn resource_type(&mut self, resource: Object) -> u32;
    fn resource_count(&mut self, resource: Object) -> u32;
    fn resource_query(&mut self, resource: Object, iid: u32, out: &mut Option<Object>) -> bool;
    fn com_create_factory(&mut self, class: u32, iid: u32, out: &mut Option<Object>) -> bool;
    fn factory_create_type(&mut self, factory: Object, ty: u32, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool;
    fn factory_create_record(&mut self, factory: Object, record: Object, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool;
    fn factory_write(&mut self, factory: Object, resource: Object, record: Object) -> bool;
    fn segment_id(&mut self, segment: Object) -> u32;
    fn segment_contains(&mut self, segment: Object, key: Key) -> bool;
    fn segment_enumerate(&mut self, segment: Object, list: &mut KeyList, filter: Option<Filter>) -> u32;
    fn segment_open(&mut self, segment: Object, key: Key, out: &mut Option<Object>, flags: u32) -> bool;
    fn segment_create(&mut self, segment: Object, key: Key, out: &mut Option<Object>) -> bool;
    fn segment_commit(&mut self, segment: Object, record: &mut Option<Object>) -> bool;
    fn segment_close(&mut self, segment: Object, record: Object) -> bool;
    fn segment_abort(&mut self, segment: Object, record: &mut Option<Object>) -> bool;
    fn record_close(&mut self, record: Object) -> bool;
    fn filter_accepts(&mut self, filter: Object, key: Key) -> bool;
    fn cache_get(&mut self, cache: Object, key: Key, out: &mut Option<Object>) -> bool;
    fn cache_insert(&mut self, cache: Object, key: Key, resource: Object) -> bool;
    fn cache_erase(&mut self, cache: Object, key: Key) -> bool;
    fn cache_contains(&mut self, cache: Object, key: Key) -> bool;
    fn cache_accepts(&mut self, cache: Object, key: Key) -> bool;
    fn cache_enumerate(&mut self, cache: Object, list: &mut KeyList, filter: Option<Filter>) -> u32;
    fn cache_update(&mut self, cache: Object, argument: u32) -> bool;
    fn cache_auto_purge(&mut self, cache: Object) -> u8;
    fn cache_set_auto_purge(&mut self, cache: Object, value: u8);
    fn cache_purge(&mut self, cache: Object) -> u8;
    fn cache_shutdown(&mut self, cache: Object);
}
