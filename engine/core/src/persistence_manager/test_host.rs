// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal foreign primitive ledger for private numeric/storage edge tests.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    AddRef(Object),
    Release(Object),
    GetKey(Object),
    SetKey(Object, Key),
    Filter(Object, Key),
    Lock(Lock, bool),
}

pub(super) struct Predicate {
    pub key: Key,
    pub set_key: Option<(Object, Key)>,
}

#[derive(Default)]
pub(super) struct Host {
    pub counts: BTreeMap<Object, u32>,
    pub keys: BTreeMap<Object, Key>,
    pub filters: BTreeMap<Object, Predicate>,
    pub events: Vec<Event>,
}
macro_rules! unused {
    ($(fn $name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)*) => {$(
        fn $name(&mut self, $($arg: $ty),*) $(-> $ret)? {
            $(let _ = $arg;)*
            panic!("unexpected foreign primitive: {}", stringify!($name))
        }
    )*};
}
impl Primitives for Host {
    fn lock(&mut self, lock: Lock, enter: bool) {
        self.events.push(Event::Lock(lock, enter));
    }
    fn add_ref(&mut self, object: Object) -> u32 {
        self.events.push(Event::AddRef(object));
        let count = self.counts.entry(object).or_default();
        *count = count.wrapping_add(1);
        *count
    }
    fn release(&mut self, object: Object) -> u32 {
        self.events.push(Event::Release(object));
        let count = self.counts.get_mut(&object).unwrap();
        assert!(*count > 0);
        *count -= 1;
        *count
    }
    fn resource_count(&mut self, object: Object) -> u32 {
        self.counts[&object]
    }
    fn resource_set_key(&mut self, object: Object, key: Key) {
        self.events.push(Event::SetKey(object, key));
        self.keys.insert(object, key);
    }
    fn resource_key(&mut self, object: Object) -> Key {
        self.events.push(Event::GetKey(object));
        self.keys[&object]
    }
    fn filter_accepts(&mut self, filter: Object, key: Key) -> bool {
        self.events.push(Event::Filter(filter, key));
        let predicate = &self.filters[&filter];
        let accepted = predicate.key == key;
        if accepted {
            if let Some((object, key)) = predicate.set_key {
                self.resource_set_key(object, key);
            }
        }
        accepted
    }
    unused! {
        fn resource_type(r: Object) -> u32;
        fn resource_query(r: Object, iid: u32, out: &mut Option<Object>) -> bool;
        fn com_create_factory(class: u32, iid: u32, out: &mut Option<Object>) -> bool;
        fn factory_create_type(f: Object, ty: u32, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool;
        fn factory_create_record(f: Object, r: Object, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool;
        fn factory_write(f: Object, r: Object, record: Object) -> bool;
        fn segment_id(s: Object) -> u32;
        fn segment_contains(s: Object, key: Key) -> bool;
        fn segment_enumerate(s: Object, list: &mut KeyList, filter: Option<Filter>) -> u32;
        fn segment_open(s: Object, key: Key, out: &mut Option<Object>, flags: u32) -> bool;
        fn segment_create(s: Object, key: Key, out: &mut Option<Object>) -> bool;
        fn segment_commit(s: Object, r: &mut Option<Object>) -> bool;
        fn segment_close(s: Object, r: Object) -> bool;
        fn segment_abort(s: Object, r: &mut Option<Object>) -> bool;
        fn record_close(r: Object) -> bool;
        fn cache_get(c: Object, key: Key, out: &mut Option<Object>) -> bool;
        fn cache_insert(c: Object, key: Key, r: Object) -> bool;
        fn cache_erase(c: Object, key: Key) -> bool;
        fn cache_contains(c: Object, key: Key) -> bool;
        fn cache_accepts(c: Object, key: Key) -> bool;
        fn cache_enumerate(c: Object, list: &mut KeyList, filter: Option<Filter>) -> u32;
        fn cache_update(c: Object, argument: u32) -> bool;
        fn cache_auto_purge(c: Object) -> u8;
        fn cache_set_auto_purge(c: Object, value: u8);
        fn cache_purge(c: Object) -> u8;
        fn cache_shutdown(c: Object);
    }
}
