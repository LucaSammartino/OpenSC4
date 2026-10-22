// SPDX-License-Identifier: GPL-3.0-or-later
use super::{table::Table, Filter, Key, KeyList, Lock, Object, Primitives, CACHE_IID};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheId {
    Builtin(u64),
    Foreign(Object),
    Null,
}

/// Owner-local cache, including the deliberately different AddRef return and
/// destructor-vs-Shutdown behavior of persistence-manager §8.1.
#[derive(Debug)]
pub struct BuiltinCache {
    id: CacheId,
    references: u32,
    counter: u32,
    automatic_purge: u8,
    filter: Option<Filter>,
    entries: Table<Key, Option<Object>>,
    deletion: Option<bool>,
}

impl BuiltinCache {
    fn new(id: CacheId) -> Self {
        Self {
            id,
            references: 0,
            counter: 0,
            automatic_purge: 0,
            filter: None,
            entries: Table::new(),
            deletion: None,
        }
    }
    pub fn reference_count(&self) -> u32 {
        self.references
    }
    pub fn maintenance_counter(&self) -> u32 {
        self.counter
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
    pub fn bucket_count(&self) -> usize {
        self.entries.bucket_count()
    }
    pub fn allocation_freed(&self) -> Option<bool> {
        self.deletion
    }
    pub fn filter(&self) -> Option<Filter> {
        self.filter
    }
    pub fn initialize(&mut self) {}

    pub fn query_interface(&mut self, host: &mut impl Primitives, iid: u32, out: &mut Option<CacheId>) -> bool {
        if iid != 1 && iid != CACHE_IID {
            return false;
        }
        *out = Some(self.id);
        self.add_ref(host);
        true
    }
    pub fn add_ref(&mut self, host: &mut impl Primitives) -> u32 {
        host.lock(Lock::CacheReferences(self.id), true);
        let old = self.references;
        self.references = old.wrapping_add(1);
        host.lock(Lock::CacheReferences(self.id), false);
        old
    }
    pub fn release(&mut self, host: &mut impl Primitives) -> u32 {
        host.lock(Lock::CacheReferences(self.id), true);
        let destroy = self.references < 2;
        let result = if destroy {
            0
        } else {
            self.references -= 1;
            self.references
        };
        host.lock(Lock::CacheReferences(self.id), false);
        if destroy {
            self.destroy(host, 1);
        }
        result
    }
    pub fn destroy(&mut self, host: &mut impl Primitives, flags: u32) {
        if self.deletion.is_some() {
            return;
        }
        if let Some(filter) = self.filter.take() {
            filter.release(host);
        }
        // §8.1: node/member destruction is NOT Clear; raw resource references
        // are deliberately not released here. Shutdown is the cleanup path.
        self.entries.destroy();
        self.deletion = Some(flags & 1 != 0);
    }
    pub fn set_filter(&mut self, host: &mut impl Primitives, filter: Option<Filter>) {
        if self.filter == filter {
            return;
        }
        if let Some(new) = filter {
            new.add_ref(host);
        }
        let old = std::mem::replace(&mut self.filter, filter);
        if let Some(old) = old {
            old.release(host);
        }
    }
    pub fn get(&mut self, host: &mut impl Primitives, key: Key, out: &mut Option<Object>) -> bool {
        host.lock(Lock::CacheEntries(self.id), true);
        let result = if let Some(resource) = self.entries.get(key) {
            let resource = resource.expect("successful cache Get requires a resource");
            *out = Some(resource);
            host.add_ref(resource);
            true
        } else {
            false
        };
        host.lock(Lock::CacheEntries(self.id), false);
        result
    }
    pub fn insert(&mut self, host: &mut impl Primitives, key: Key, resource: Object) -> bool {
        host.lock(Lock::CacheEntries(self.id), true);
        let result = if self.entries.get(key).is_some() {
            false
        } else {
            host.add_ref(resource);
            host.resource_set_key(resource, key);
            self.entries.insert_unique(key, Some(resource))
        };
        host.lock(Lock::CacheEntries(self.id), false);
        result
    }
    pub fn erase(&mut self, host: &mut impl Primitives, key: Key) -> bool {
        host.lock(Lock::CacheEntries(self.id), true);
        let removed = self.entries.remove(key);
        if let Some(Some(resource)) = removed {
            host.release(resource);
        }
        host.lock(Lock::CacheEntries(self.id), false);
        removed.is_some()
    }
    pub fn contains(&mut self, host: &mut impl Primitives, key: Key) -> bool {
        host.lock(Lock::CacheEntries(self.id), true);
        let result = self.entries.get(key).is_some();
        host.lock(Lock::CacheEntries(self.id), false);
        result
    }
    pub fn accepts(&self, host: &mut impl Primitives, key: Key) -> bool {
        self.filter.map_or(true, |filter| filter.accepts(host, key))
    }
    pub fn enumerate(
        &mut self,
        host: &mut impl Primitives,
        mut list: Option<&mut KeyList>,
        filter: Option<Filter>,
    ) -> u32 {
        host.lock(Lock::CacheEntries(self.id), true);
        let mut count = 0u32;
        for (_, resource) in self.entries.entries() {
            // PR5: one current-key copy per entry, even without output or on
            // rejection. Callbacks may SetKey; predicates and output keep this copy.
            let resource = resource.expect("enumerated resource must be valid");
            let key = host.resource_key(resource);
            if self.filter.is_some_and(|f| !f.accepts(host, key)) {
                continue;
            }
            if filter.is_some_and(|f| !f.accepts(host, key)) {
                continue;
            }
            count = count.wrapping_add(1);
            if let Some(list) = list.as_mut() {
                list.insert(key);
            }
        }
        host.lock(Lock::CacheEntries(self.id), false);
        count
    }
    pub fn enumerate_type(&mut self, host: &mut impl Primitives, list: Option<&mut KeyList>, ty: u32) -> u32 {
        // The owned Type variant is a temporary referenced type-equality
        // filter; Rust frees its storage after the forwarded enumeration.
        self.enumerate(host, list, Some(Filter::Type(ty)))
    }
    pub fn automatic_purge(&self, host: &mut impl Primitives) -> u8 {
        host.lock(Lock::CacheReferences(self.id), true);
        let result = self.automatic_purge;
        host.lock(Lock::CacheReferences(self.id), false);
        result
    }
    pub fn set_automatic_purge(&mut self, host: &mut impl Primitives, value: u8) {
        host.lock(Lock::CacheReferences(self.id), true);
        self.automatic_purge = value;
        host.lock(Lock::CacheReferences(self.id), false);
    }
    pub fn update(&mut self, host: &mut impl Primitives, argument: u32) -> bool {
        self.update_with(host, argument, |cache, host| cache.purge_one(host))
    }
    /// Virtual slot 20 can be overridden without delegating the owner's tick
    /// or bounded-repeat algorithms to a foreign cache implementation.
    pub fn update_with<P: Primitives>(
        &mut self,
        host: &mut P,
        _argument: u32,
        mut purge: impl FnMut(&mut Self, &mut P) -> bool,
    ) -> bool {
        if self.automatic_purge != 0 {
            self.counter = self.counter.wrapping_add(1);
            if (self.counter as i32) > 128 {
                self.counter = 0;
                purge(self, host);
            }
        }
        true
    }
    pub fn purge(&mut self, host: &mut impl Primitives) -> bool {
        self.purge_with(host, |cache, host| cache.purge_one(host))
    }
    pub fn purge_with<P: Primitives>(&mut self, host: &mut P, mut one: impl FnMut(&mut Self, &mut P) -> bool) -> bool {
        let mut any = false;
        for _ in 0..8 {
            if !one(self, host) {
                break;
            }
            any = true;
        }
        any
    }
    pub fn clear(&mut self, host: &mut impl Primitives) {
        for (_, resource) in self.entries.entries() {
            if let Some(resource) = resource {
                host.release(*resource);
            }
        }
        self.entries.clear();
    }
    pub fn shutdown(&mut self, host: &mut impl Primitives) {
        self.shutdown_with(host, |cache, host| cache.clear(host));
    }
    pub fn shutdown_with<P: Primitives>(&mut self, host: &mut P, mut clear: impl FnMut(&mut Self, &mut P)) {
        host.lock(Lock::CacheEntries(self.id), true);
        clear(self, host);
        self.clear(host); // Sweep anything a virtual Clear left behind.
        host.lock(Lock::CacheEntries(self.id), false);
    }
    pub fn purge_one(&mut self, host: &mut impl Primitives) -> bool {
        host.lock(Lock::CacheEntries(self.id), true);
        let mut purged = false;
        self.entries.retain(|_, resource| {
            let Some(resource) = resource else {
                return false;
            };
            if host.resource_count(*resource) > 1 {
                return true;
            }
            host.release(*resource);
            purged = true;
            false
        });
        host.lock(Lock::CacheEntries(self.id), false);
        purged
    }
}

/// Handle arena keeps symbolic object identity separate from logical COM
/// references. Copies of handles never cause implicit host calls.
pub struct CacheStore<P: Primitives> {
    pub host: P,
    builtins: BTreeMap<u64, BuiltinCache>,
    next: u64,
}

impl<P: Primitives> CacheStore<P> {
    pub fn new(host: P) -> Self {
        Self {
            host,
            builtins: BTreeMap::new(),
            next: 0,
        }
    }
    pub fn create(&mut self) -> CacheId {
        let id = self.next;
        self.next += 1;
        self.builtins.insert(id, BuiltinCache::new(CacheId::Builtin(id)));
        CacheId::Builtin(id)
    }
    pub fn builtin(&self, id: CacheId) -> Option<&BuiltinCache> {
        match id {
            CacheId::Builtin(id) => self.builtins.get(&id),
            _ => None,
        }
    }
    pub fn with_builtin<R>(&mut self, id: CacheId, call: impl FnOnce(&mut BuiltinCache, &mut P) -> R) -> R {
        let CacheId::Builtin(id) = id else {
            panic!("expected built-in cache");
        };
        let cache = self.builtins.get_mut(&id).expect("valid built-in cache identity");
        assert!(cache.deletion.is_none(), "valid cache lifetime required");
        call(cache, &mut self.host)
    }
    pub fn add_ref(&mut self, id: CacheId) -> u32 {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.add_ref(h)),
            CacheId::Foreign(id) => self.host.add_ref(id),
            CacheId::Null => 0,
        }
    }
    pub fn release(&mut self, id: CacheId) -> u32 {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.release(h)),
            CacheId::Foreign(id) => self.host.release(id),
            CacheId::Null => 0,
        }
    }
    pub fn get(&mut self, id: CacheId, key: Key, out: &mut Option<Object>) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.get(h, key, out)),
            CacheId::Foreign(id) => self.host.cache_get(id, key, out),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn insert(&mut self, id: CacheId, key: Key, resource: Object) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.insert(h, key, resource)),
            CacheId::Foreign(id) => self.host.cache_insert(id, key, resource),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn contains(&mut self, id: CacheId, key: Key) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.contains(h, key)),
            CacheId::Foreign(id) => self.host.cache_contains(id, key),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn erase(&mut self, id: CacheId, key: Key) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.erase(h, key)),
            CacheId::Foreign(id) => self.host.cache_erase(id, key),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn accepts(&mut self, id: CacheId, key: Key) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.accepts(h, key)),
            CacheId::Foreign(id) => self.host.cache_accepts(id, key),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn enumerate(&mut self, id: CacheId, list: &mut KeyList, filter: Option<Filter>) -> u32 {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.enumerate(h, Some(list), filter)),
            CacheId::Foreign(id) => self.host.cache_enumerate(id, list, filter),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn update(&mut self, id: CacheId, argument: u32) -> bool {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.update(h, argument)),
            CacheId::Foreign(id) => self.host.cache_update(id, argument),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn automatic_purge(&mut self, id: CacheId) -> u8 {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.automatic_purge(h)),
            CacheId::Foreign(id) => self.host.cache_auto_purge(id),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn set_automatic_purge(&mut self, id: CacheId, value: u8) {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.set_automatic_purge(h, value)),
            CacheId::Foreign(id) => self.host.cache_set_auto_purge(id, value),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn purge(&mut self, id: CacheId) -> u8 {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| u8::from(c.purge(h))),
            CacheId::Foreign(id) => self.host.cache_purge(id),
            CacheId::Null => panic!("null cache callback"),
        }
    }
    pub fn shutdown(&mut self, id: CacheId) {
        match id {
            CacheId::Builtin(_) => self.with_builtin(id, |c, h| c.shutdown(h)),
            CacheId::Foreign(id) => self.host.cache_shutdown(id),
            CacheId::Null => panic!("null cache callback"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_host::{Event, Host, Predicate};
    use super::*;

    const K: Key = Key::new(1, 2, 194);
    const J: Key = Key::new(1, 2, 1);
    const L: Key = Key::new(9, 7, 8);
    const M: Key = Key::new(10, 7, 9);
    const SENTINEL: Key = Key::new(99, 98, 97);
    const R: Object = Object(1);
    const S: Object = Object(2);
    const STORED: Object = Object(3);
    const SUPPLIED: Object = Object(4);

    fn pr5_fixture() -> (BuiltinCache, Host) {
        let mut cache = BuiltinCache::new(CacheId::Builtin(1));
        let mut host = Host::default();
        for object in [R, S, STORED, SUPPLIED] {
            host.counts.insert(object, 1);
        }
        assert!(cache.insert(&mut host, K, R));
        assert!(cache.insert(&mut host, J, S));
        (cache, host)
    }

    fn predicate(host: &mut Host, object: Object, key: Key, set_key: Option<(Object, Key)>) {
        host.filters.insert(object, Predicate { key, set_key });
    }

    fn assert_enumeration_events(cache: &BuiltinCache, host: &Host, inner: Vec<Event>) {
        let mut expected = vec![Event::Lock(Lock::CacheEntries(cache.id), true)];
        expected.extend(inner);
        expected.push(Event::Lock(Lock::CacheEntries(cache.id), false));
        // Includes GetKey on rejected entries and excludes any refcount calls.
        assert_eq!(host.events, expected);
    }

    fn finish_pr5(cache: &mut BuiltinCache, host: &mut Host) {
        cache.shutdown(host);
        cache.destroy(host, 1);
        for object in [R, S, STORED, SUPPLIED] {
            assert_eq!(host.counts[&object], 1);
        }
    }

    #[test]
    fn pr5_a_through_d_filters_use_current_key_with_or_without_output() {
        use Event::{Filter as F, GetKey as G};
        let cases = [
            // A: unchanged resource. B: stored-key acceptance must be rejected.
            (
                K,
                Some(K),
                K,
                1,
                vec![G(S), F(STORED, J), G(R), F(STORED, K), F(SUPPLIED, K)],
            ),
            (L, Some(K), K, 0, vec![G(S), F(STORED, J), G(R), F(STORED, L)]),
            // C: current-key acceptance. D: supplied filter independently rejects.
            (
                L,
                Some(L),
                L,
                1,
                vec![G(S), F(STORED, J), G(R), F(STORED, L), F(SUPPLIED, L)],
            ),
            (
                L,
                Some(L),
                K,
                0,
                vec![G(S), F(STORED, J), G(R), F(STORED, L), F(SUPPLIED, L)],
            ),
            // Supplied predicate alone must accept L and reject K after SetKey.
            (L, None, L, 1, vec![G(S), F(SUPPLIED, J), G(R), F(SUPPLIED, L)]),
            (L, None, K, 0, vec![G(S), F(SUPPLIED, J), G(R), F(SUPPLIED, L)]),
        ];
        for (current, stored, supplied, accepted, trace) in cases {
            for output in [true, false] {
                let (mut cache, mut host) = pr5_fixture();
                host.resource_set_key(R, current);
                if let Some(key) = stored {
                    predicate(&mut host, STORED, key, None);
                    cache.set_filter(&mut host, Some(Filter::Foreign(STORED)));
                }
                predicate(&mut host, SUPPLIED, supplied, None);
                let counts = host.counts.clone();
                host.events.clear();
                let mut list = KeyList::new();
                list.insert(SENTINEL);
                let out = if output { Some(&mut list) } else { None };
                assert_eq!(
                    cache.enumerate(&mut host, out, Some(Filter::Foreign(SUPPLIED))),
                    accepted
                );
                assert_enumeration_events(&cache, &host, trace.clone());
                let expected = if output && accepted == 1 {
                    vec![SENTINEL, current]
                } else {
                    vec![SENTINEL]
                };
                assert_eq!(list.keys(), expected);
                assert_eq!(host.counts, counts);

                // External SetKey changes enumeration, never the table index.
                let mut found = Some(STORED);
                assert!(cache.get(&mut host, K, &mut found));
                assert_eq!(found, Some(R));
                host.release(R);
                found = Some(STORED);
                assert!(!cache.get(&mut host, L, &mut found));
                assert_eq!(found, Some(STORED));
                assert_eq!(host.counts, counts);
                finish_pr5(&mut cache, &mut host);
            }
        }
    }

    #[test]
    fn pr5_e_duplicate_current_keys_and_type_filter_keep_stored_entries() {
        use Event::GetKey as G;
        for output in [true, false] {
            let (mut cache, mut host) = pr5_fixture();
            host.resource_set_key(R, L);
            host.resource_set_key(S, L);
            let counts = host.counts.clone();
            let mut list = KeyList::new();
            list.insert(SENTINEL);
            host.events.clear();
            let out = if output { Some(&mut list) } else { None };
            assert_eq!(cache.enumerate(&mut host, out, None), 2);
            assert_enumeration_events(&cache, &host, vec![G(S), G(R)]);
            assert_eq!(list.keys(), if output { vec![SENTINEL, L, L] } else { vec![SENTINEL] });
            assert_eq!(cache.len(), 2);
            for (ty, count) in [(1, 0), (9, 2)] {
                host.events.clear();
                assert_eq!(cache.enumerate_type(&mut host, None, ty), count);
                assert_enumeration_events(&cache, &host, vec![G(S), G(R)]);
            }
            assert_eq!(host.counts, counts);
            assert!(cache.contains(&mut host, K));
            assert!(cache.contains(&mut host, J));
            assert!(!cache.contains(&mut host, L));
            finish_pr5(&mut cache, &mut host);
        }
    }

    #[test]
    fn pr5_f_either_filter_can_set_key_without_changing_this_calls_copy() {
        use Event::{Filter as F, GetKey as G, SetKey as Set};
        // PR5-F mutates through the stored filter; also exercise supplied-filter mutation.
        for mutator in [STORED, SUPPLIED] {
            for output in [true, false] {
                let (mut cache, mut host) = pr5_fixture();
                host.resource_set_key(R, L);
                for filter in [STORED, SUPPLIED] {
                    let mutation = if filter == mutator { Some((R, M)) } else { None };
                    predicate(&mut host, filter, L, mutation);
                }
                cache.set_filter(&mut host, Some(Filter::Foreign(STORED)));
                let counts = host.counts.clone();
                host.events.clear();
                let mut list = KeyList::new();
                list.insert(SENTINEL);
                let out = if output { Some(&mut list) } else { None };
                assert_eq!(cache.enumerate(&mut host, out, Some(Filter::Foreign(SUPPLIED))), 1);
                let mut trace = vec![G(S), F(STORED, J), G(R), F(STORED, L)];
                if mutator == STORED {
                    trace.push(Set(R, M));
                }
                trace.push(F(SUPPLIED, L));
                if mutator == SUPPLIED {
                    trace.push(Set(R, M));
                }
                assert_enumeration_events(&cache, &host, trace);
                assert_eq!(list.keys(), if output { vec![SENTINEL, L] } else { vec![SENTINEL] });
                assert_eq!(host.keys[&R], M);
                assert_eq!(host.counts, counts);

                // Next call reads M once; rejection still reads both resources.
                host.events.clear();
                assert_eq!(cache.enumerate(&mut host, None, Some(Filter::Foreign(SUPPLIED))), 0);
                assert_enumeration_events(&cache, &host, vec![G(S), F(STORED, J), G(R), F(STORED, M)]);
                assert_eq!(host.counts, counts);
                finish_pr5(&mut cache, &mut host);
            }
        }
    }

    #[test]
    fn pm19_null_only_entries_are_removed_without_a_successful_resource_purge() {
        let mut store = CacheStore::new(Host::default());
        let id = store.create();
        store.with_builtin(id, |cache, host| {
            cache.entries.insert_unique(Key::new(1, 2, 3), None);
            assert!(!cache.purge_one(host));
            assert!(cache.is_empty());
        });
    }

    #[test]
    fn tick_compares_wrapped_counter_as_signed_and_ignores_disabled_ticks() {
        let mut store = CacheStore::new(Host::default());
        let id = store.create();
        store.with_builtin(id, |cache, host| {
            cache.set_automatic_purge(host, 1);
            let mut purges = 0;
            for (before, after) in [(0x7fff_ffff, 0x8000_0000), (u32::MAX, 0), (128, 0)] {
                cache.counter = before;
                assert!(cache.update_with(host, u32::MAX, |_, _| {
                    purges += 1;
                    true
                }));
                assert_eq!(cache.counter, after);
            }
            assert_eq!(purges, 1);
            cache.set_automatic_purge(host, 0);
            cache.counter = u32::MAX;
            cache.update_with(host, 7, |_, _| panic!("disabled tick purged"));
            assert_eq!(cache.counter, u32::MAX);
        });
    }

    #[test]
    fn shutdown_sweeps_virtual_clear_remainders_and_unsigned_purge_keeps_large_counts() {
        let mut store = CacheStore::new(Host::default());
        let id = store.create();
        store.with_builtin(id, |cache, host| {
            cache.insert(host, Key::new(1, 2, 3), Object(1));
            cache.insert(host, Key::new(1, 2, 4), Object(2));
            host.counts.insert(Object(2), u32::MAX);
            assert!(cache.purge_one(host));
            assert_eq!(cache.len(), 1);
            assert_eq!(host.counts[&Object(2)], u32::MAX);
            cache.shutdown_with(host, |_, _| {});
            assert!(cache.is_empty());
            assert_eq!(host.counts[&Object(2)], u32::MAX - 1);
        });
    }
}
