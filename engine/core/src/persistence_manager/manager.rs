// SPDX-License-Identifier: GPL-3.0-or-later
use super::{table::{PointerDeque, Table}, CacheId, CacheStore, Filter, Key, KeyList, Lock, Object,
    Primitives, FACTORY_IID, MANAGER_IID, RESOURCE_IID, SERVICE_IID};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceView { Manager, Service, Unknown }
impl InterfaceView {
    pub fn offset(self) -> u32 {
        match self { Self::Manager => 0, Self::Service => 4, Self::Unknown => 8 }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceState { pub id: u32, pub priorities: [u32; 2], pub running: bool }

/// All owner decisions reside here (spec §§1–11). Host calls are primitive
/// foreign vtable operations, never answers from an original manager.
/// Getters borrow and mutations require exclusive access; symbolic Lock events
/// retain native scope/order (including deliberately mismatched locks).
pub struct PersistManager<P: Primitives> {
    pub objects: CacheStore<P>,
    factories: Table<u32, Object>,
    types: Table<u32, u32>,
    segments: PointerDeque<Object>,
    caches: Vec<CacheId>,
    default_cache: CacheId,
    group: u32,
    next_instance: u32,
    references: u32,
    service: ServiceState,
    deletion: Option<bool>,
}

impl<P: Primitives> PersistManager<P> {
    pub fn new(host: P) -> Self {
        let mut objects = CacheStore::new(host);
        let default_cache = objects.create();
        objects.add_ref(default_cache); // One net auto-reference in the list.
        Self { objects, factories: Table::new(), types: Table::new(), segments: PointerDeque::new(),
            caches: vec![default_cache], default_cache, group: 0, next_instance: 0xffff,
            references: 0, service: ServiceState { id: 0x056b906e, priorities: [1_000_000; 2], running: false },
            deletion: None }
    }
    fn lock(&mut self, lock: Lock, enter: bool) { self.objects.host.lock(lock, enter); }
    pub fn default_cache(&self) -> CacheId { self.default_cache }
    pub fn service(&self) -> ServiceState { self.service }
    pub fn reference_count(&self) -> u32 { self.references }
    pub fn allocation_freed(&self) -> Option<bool> { self.deletion }
    pub fn initialize(&mut self) -> bool { true }

    /// Main QI delegates service/unknown views to the inherited service QI;
    /// all views share one plain count. Unsupported requests preserve output.
    pub fn query_interface(&mut self, _from: InterfaceView, iid: u32, out: &mut Option<InterfaceView>) -> bool {
        let view = match iid { MANAGER_IID => InterfaceView::Manager, SERVICE_IID => InterfaceView::Service,
            1 => InterfaceView::Unknown, _ => return false };
        *out = Some(view);
        self.add_ref();
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self, _from: InterfaceView) -> u32 {
        if self.references > 1 { self.references -= 1; self.references }
        else { self.destroy(1); 0 }
    }
    /// Ordinary/deleting destruction is intentionally not Shutdown. Only the
    /// cache list owns auto-references; raw factory/segment entries do not
    /// release themselves on member destruction (§9).
    pub fn destroy(&mut self, flags: u32) {
        if self.deletion.is_some() { return; }
        for cache in self.caches.drain(..) { self.objects.release(cache); }
        self.factories.clear();
        self.types.clear();
        self.segments.clear();
        self.deletion = Some(flags & 1 != 0);
    }
    pub fn shutdown(&mut self) -> bool {
        self.lock(Lock::CacheRegistry, true);
        self.lock(Lock::Factory, true);
        self.lock(Lock::Segment, true);
        for &cache in &self.caches { self.objects.shutdown(cache); }
        for cache in self.caches.drain(..) { self.objects.release(cache); }
        self.lock(Lock::CacheRegistry, false);
        for (_, factory) in self.factories.entries() { self.objects.host.release(*factory); }
        self.factories.clear(); // Keep type associations, group, next ID, service state.
        self.lock(Lock::Factory, false);
        for segment in self.segments.iter() { self.objects.host.release(segment); }
        self.segments.clear();
        self.lock(Lock::Segment, false);
        true
    }
    /// PR4 §9.2 chosen policy: initialized logical OR, every callback executed.
    /// Native's uninitialized byte is NOT modeled as a false/true baseline.
    pub fn update(&mut self, argument: u32) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let mut result = false;
        for &cache in &self.caches {
            let current = self.objects.update(cache, argument);
            result |= current;
        }
        self.lock(Lock::CacheRegistry, false);
        result
    }

    // Slots 19–25: two independently growing, duplicate-preserving maps.
    pub fn register_factory(&mut self, id: u32, ty: u32, supplied: Option<Object>) -> bool {
        let mut factory = supplied;
        if let Some(factory) = factory { self.objects.host.add_ref(factory); }
        else { self.objects.host.com_create_factory(id, FACTORY_IID, &mut factory); }
        let Some(factory) = factory else { return false; };
        self.lock(Lock::Factory, true);
        self.factories.insert_duplicate(id, factory);
        self.types.insert_duplicate(ty, id);
        self.lock(Lock::Factory, false);
        true
    }
    pub fn remove_factory(&mut self, id: u32) -> bool {
        self.lock(Lock::Factory, true);
        let mut result = false;
        while let Some(factory) = self.factories.remove(id) {
            self.objects.host.release(factory);
            result = true;
        }
        self.lock(Lock::Factory, false);
        result
    }
    pub fn factory_for_type(&mut self, ty: u32, out: &mut Option<Object>) -> bool {
        self.lock(Lock::Factory, true);
        let found = self.types.get(ty).and_then(|id| self.factories.get(*id)).copied();
        if let Some(factory) = found { *out = Some(factory); self.objects.host.add_ref(factory); }
        self.lock(Lock::Factory, false);
        found.is_some()
    }
    pub fn factory_for_key(&mut self, key: Key, out: &mut Option<Object>) -> bool { self.factory_for_type(key.ty, out) }
    pub fn factory_for_resource(&mut self, resource: Object, out: &mut Option<Object>) -> bool {
        let ty = self.objects.host.resource_type(resource);
        self.factory_for_type(ty, out)
    }
    pub fn factory_count(&mut self) -> u32 {
        self.lock(Lock::Factory, true);
        let result = self.factories.len() as u32;
        self.lock(Lock::Factory, false);
        result
    }
    pub fn factory_at(&mut self, index: u32) -> Option<Object> {
        self.lock(Lock::Factory, true);
        let result = self.factories.entries().nth(index as usize).map(|(_, value)| *value);
        self.lock(Lock::Factory, false);
        result
    }

    // Slots 26–38: ordered, pointer-identical segment registry/direct records.
    pub fn register_segment(&mut self, segment: Object) -> bool { self.prepend_segment(segment) }
    pub fn prepend_segment(&mut self, segment: Object) -> bool { self.move_segment(segment, true) }
    pub fn append_segment(&mut self, segment: Object) -> bool { self.move_segment(segment, false) }
    fn move_segment(&mut self, segment: Object, front: bool) -> bool {
        self.objects.host.add_ref(segment);
        self.lock(Lock::Segment, true);
        self.remove_segment(segment); // Recursive acquisition, release old ref.
        if front { self.segments.push_front(segment); } else { self.segments.push_back(segment); }
        self.lock(Lock::Segment, false);
        true
    }
    pub fn remove_segment(&mut self, segment: Object) -> bool {
        self.lock(Lock::Segment, true);
        let index = self.segments.iter().position(|s| s == segment);
        if let Some(index) = index {
            self.objects.host.release(segment);
            self.segments.erase(index);
        }
        self.lock(Lock::Segment, false);
        index.is_some()
    }
    pub fn has_segment(&mut self, segment: Object) -> bool {
        self.lock(Lock::Segment, true);
        let found = self.segments.iter().any(|s| s == segment);
        self.lock(Lock::Segment, false);
        found
    }
    pub fn segment_by_id(&mut self, id: u32, out: &mut Option<Object>) -> bool {
        self.lock(Lock::Segment, true);
        *out = None;
        for segment in self.segments.iter() {
            if self.objects.host.segment_id(segment) == id { *out = Some(segment); self.objects.host.add_ref(segment); break; }
        }
        self.lock(Lock::Segment, false);
        out.is_some()
    }
    pub fn segment_for_key(&mut self, key: Key, out: &mut Option<Object>) -> bool {
        self.lock(Lock::Segment, true);
        *out = None;
        for segment in self.segments.iter() {
            if self.objects.host.segment_contains(segment, key) { *out = Some(segment); self.objects.host.add_ref(segment); break; }
        }
        self.lock(Lock::Segment, false);
        out.is_some()
    }
    pub fn segment_count(&mut self) -> u32 {
        self.lock(Lock::Segment, true);
        let count = self.segments.len() as u32;
        self.lock(Lock::Segment, false);
        count
    }
    pub fn segment_at(&mut self, index: u32) -> Option<Object> {
        self.lock(Lock::Segment, true);
        let found = self.segments.get(index as usize);
        self.lock(Lock::Segment, false);
        found
    }
    pub fn visit_segments(&mut self, mut callback: Option<&mut dyn FnMut(Object, &mut P) -> bool>) {
        if let Some(callback) = callback.as_mut() {
            for segment in self.segments.iter() { if !callback(segment, &mut self.objects.host) { break; } }
        }
    }
    pub fn segment_array(&mut self, out: Option<&mut [Option<Object>]>, capacity: &mut u32) -> u32 {
        let total = self.segments.len() as u32;
        if let Some(out) = out {
            if *capacity != 0 {
                let written = (*capacity).min(total) as usize;
                assert!(out.len() >= written, "array has the advertised writable capacity");
                for (i, cell) in out.iter_mut().enumerate().take(written) {
                    let segment = self.segments.get(i).unwrap();
                    *cell = Some(segment);
                    self.objects.host.add_ref(segment);
                }
                *capacity = written as u32;
            }
        }
        total
    }
    pub fn open_record(&mut self, key: Key, out: &mut Option<Object>, writable: u32) -> bool {
        let mut segment = None;
        if !self.segment_for_key(key, &mut segment) { return false; }
        let segment = segment.unwrap();
        let result = self.objects.host.segment_open(segment, key, out, if writable == 0 { 1 } else { 3 });
        self.objects.host.release(segment);
        result
    }
    pub fn close_record(&mut self, _unused: Option<Object>, record_pointer: &mut Option<Object>) -> bool {
        let record = record_pointer.expect("CloseRecord requires an owned record");
        let result = self.objects.host.record_close(record);
        self.objects.host.release(record);
        result // Leave the pointer cell intact, now borrowed/dangling.
    }

    // Slots 39–44: registry and cache acceptance are separate from insertion.
    pub fn register_cache(&mut self, cache: CacheId) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let result = if self.caches.contains(&cache) { false } else {
            self.objects.add_ref(cache);
            self.caches.push(cache);
            true
        };
        self.lock(Lock::CacheRegistry, false);
        result
    }
    pub fn remove_cache(&mut self, cache: CacheId) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let found = self.caches.iter().position(|c| *c == cache);
        if let Some(index) = found { self.objects.release(cache); self.caches.remove(index); }
        self.lock(Lock::CacheRegistry, false);
        found.is_some()
    }
    pub fn automatic_purge(&mut self) -> u8 {
        self.lock(Lock::Key, true);
        let mut value = 0;
        for &cache in &self.caches { value |= self.objects.automatic_purge(cache); }
        self.lock(Lock::Key, false);
        value
    }
    pub fn set_automatic_purge(&mut self, value: u8) {
        self.lock(Lock::Key, true);
        for &cache in &self.caches { self.objects.set_automatic_purge(cache, value); }
        self.lock(Lock::Key, false);
    }
    pub fn purge(&mut self) -> u8 {
        self.lock(Lock::CacheRegistry, true);
        let mut value = 0;
        for &cache in &self.caches { value |= self.objects.purge(cache); }
        self.lock(Lock::CacheRegistry, false);
        value
    }
    pub fn insert_cache(&mut self, key: Key, resource: Object) -> bool {
        for &cache in &self.caches {
            if self.objects.accepts(cache, key) {
                self.objects.insert(cache, key, resource);
                return true;
            }
        }
        false
    }
    pub fn is_cached(&mut self, key: Key) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let mut result = false;
        for &cache in &self.caches { if self.objects.contains(cache, key) { result = true; break; } }
        self.lock(Lock::CacheRegistry, false);
        result
    }
    pub fn unregister_resource(&mut self, key: Key) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let mut result = false;
        for &cache in &self.caches {
            if self.objects.contains(cache, key) && self.objects.erase(cache, key) { result = true; break; }
        }
        self.lock(Lock::CacheRegistry, false);
        result
    }
    pub fn register_resource(&mut self, resource: Option<Object>) -> bool {
        let Some(resource) = resource else { return false; };
        self.lock(Lock::CacheRegistry, true);
        let key = self.objects.host.resource_key(resource);
        let result = self.is_cached(key) || self.insert_cache(key, resource);
        self.lock(Lock::CacheRegistry, false);
        result
    }
    pub fn register_resource_as(&mut self, resource: Object, key: Key) -> bool {
        self.objects.host.resource_set_key(resource, key);
        self.register_resource(Some(resource))
    }
    pub fn is_in_segments(&mut self, key: Key) -> bool {
        self.lock(Lock::Segment, true);
        let mut result = false;
        for segment in self.segments.iter() {
            if self.objects.host.segment_contains(segment, key) { result = true; break; }
        }
        self.lock(Lock::Segment, false);
        result
    }
    pub fn cached_keys(&mut self, filter: Option<Filter>) -> (u32, KeyList) {
        let mut list = KeyList::new();
        let mut count = 0u32;
        self.lock(Lock::CacheRegistry, true);
        for &cache in &self.caches { count = count.wrapping_add(self.objects.enumerate(cache, &mut list, filter)); }
        self.lock(Lock::CacheRegistry, false);
        (count, list)
    }
    pub fn cached_keys_of_type(&mut self, ty: u32) -> (u32, KeyList) { self.cached_keys(Some(Filter::Type(ty))) }
    pub fn segment_keys(&mut self, filter: Option<Filter>) -> (u32, KeyList) {
        let mut list = KeyList::new();
        let mut count = 0u32;
        self.lock(Lock::CacheRegistry, true); // §3: deliberately NOT segment lock.
        for segment in self.segments.iter() {
            count = count.wrapping_add(self.objects.host.segment_enumerate(segment, &mut list, filter));
        }
        self.lock(Lock::CacheRegistry, false);
        (count, list)
    }
    pub fn segment_keys_of_type(&mut self, ty: u32) -> (u32, KeyList) { self.segment_keys(Some(Filter::Type(ty))) }

    // Internal slots 45–47, no cache operations or failure memoization.
    pub fn create_underlying(&mut self, key: Key, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let mut factory = None;
        if !self.factory_for_key(key, &mut factory) { return false; }
        let factory = factory.unwrap();
        let result = self.objects.host.factory_create_type(factory, key.ty, iid, out, args);
        self.objects.host.release(factory);
        result
    }
    pub fn load_underlying(&mut self, key: Key, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let mut factory = None;
        if !self.factory_for_key(key, &mut factory) { return false; }
        let factory = factory.unwrap();
        let mut segment = None;
        let result = if self.segment_for_key(key, &mut segment) {
            let segment = segment.unwrap();
            let result = self.load_selected(key, factory, segment, iid, out, args);
            self.objects.host.release(segment);
            result
        } else { false };
        self.objects.host.release(factory);
        result
    }
    pub fn load_selected(&mut self, key: Key, factory: Object, segment: Object, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let mut record = None;
        if !self.objects.host.segment_open(segment, key, &mut record, 1) {
            if record.is_some() { self.objects.host.segment_abort(segment, &mut record); }
            return false;
        }
        let mut resource = None;
        if !self.objects.host.factory_create_record(factory, record.expect("open success supplies record"), RESOURCE_IID, &mut resource, args) {
            self.objects.host.segment_abort(segment, &mut record);
            // False factory output is not released by this owner (§6).
            return false;
        }
        let resource = resource.expect("factory success supplies resource");
        self.objects.host.resource_set_key(resource, key);
        let result = self.objects.host.resource_query(resource, iid, out);
        let record = record.unwrap();
        self.objects.host.segment_close(segment, record);
        self.objects.host.release(record);
        self.objects.host.release(resource);
        result
    }

    // Public load/create entries 3–6. Keep the different QI/cache ordering.
    pub fn load(&mut self, key: Key, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        self.lock(Lock::CacheRegistry, true);
        let mut resource = None;
        let mut found = false;
        for &cache in &self.caches {
            if self.objects.get(cache, key, &mut resource) { found = true; break; }
        }
        if !found {
            if !self.load_underlying(key, RESOURCE_IID, &mut resource, args) {
                self.lock(Lock::CacheRegistry, false);
                return false;
            }
            self.insert_cache(key, resource.expect("load success supplies resource"));
        }
        let resource = resource.expect("cache/load success supplies resource");
        let result = self.objects.host.resource_query(resource, iid, out);
        self.objects.host.release(resource);
        self.lock(Lock::CacheRegistry, false);
        result
    }
    pub fn load_uncached(&mut self, key: Key, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let mut resource = None;
        if !self.load_underlying(key, RESOURCE_IID, &mut resource, args) { return false; }
        let resource = resource.expect("load success supplies resource");
        let result = self.objects.host.resource_query(resource, iid, out);
        self.objects.host.release(resource);
        result
    }
    pub fn current_group(&mut self) -> u32 {
        self.lock(Lock::Key, true);
        let group = self.group;
        self.lock(Lock::Key, false);
        group
    }
    pub fn allocate_instance(&mut self) -> u32 {
        self.lock(Lock::Key, true);
        let result = self.next_instance;
        self.next_instance = result.wrapping_add(1);
        self.lock(Lock::Key, false);
        result
    }
    pub fn create(&mut self, ty: u32, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let group = self.current_group();
        let instance = self.allocate_instance(); // Consume even on failed creation/QI.
        self.create_as(Key::new(ty, group, instance), iid, out, args)
    }
    pub fn create_as(&mut self, key: Key, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        let mut resource = None;
        if !self.create_underlying(key, RESOURCE_IID, &mut resource, args) { return false; }
        let resource = resource.expect("factory success supplies resource");
        self.objects.host.resource_set_key(resource, key);
        let result = self.objects.host.resource_query(resource, iid, out);
        if result { self.insert_cache(key, resource); }
        self.objects.host.release(resource);
        result
    }

    // Slots 11–13: nontransactional writes with preserved open/create result.
    pub fn write_keys(&mut self, list: &KeyList, segment: Object) -> bool {
        let count = list.size();
        let mut result = true;
        for index in 0..count {
            let current = self.write_key(list.keys()[index as usize], segment);
            result &= current;
        }
        result
    }
    pub fn write_key(&mut self, key: Key, segment: Object) -> bool {
        let mut resource = None;
        if !self.load(key, RESOURCE_IID, &mut resource, [0, 0]) { return false; }
        let resource = resource.expect("load success supplies resource");
        let result = self.write_resource(resource, key, segment);
        self.objects.host.release(resource);
        result
    }
    pub fn write_resource(&mut self, resource: Object, creation_key: Key, segment: Object) -> bool {
        let key = self.objects.host.resource_key(resource);
        let mut record = None;
        let opened = if self.objects.host.segment_contains(segment, key) {
            self.objects.host.segment_open(segment, key, &mut record, 3)
        } else { self.objects.host.segment_create(segment, creation_key, &mut record) };
        if opened {
            let mut factory = None;
            if self.factory_for_resource(resource, &mut factory) {
                let factory = factory.unwrap();
                if self.objects.host.factory_write(factory, resource, record.expect("successful open/create supplies record"))
                    && self.objects.host.segment_commit(segment, &mut record) {
                    record = None;
                }
                self.objects.host.release(factory);
            }
        }
        if record.is_some() { self.objects.host.segment_abort(segment, &mut record); }
        opened // Write/commit failure does NOT change this return.
    }
}

impl<P: Primitives> Drop for PersistManager<P> {
    fn drop(&mut self) { self.destroy(0); }
}
