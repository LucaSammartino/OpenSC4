// SPDX-License-Identifier: GPL-3.0-or-later
//! Instrumented foreign primitives. All manager/cache owner logic runs in core.
use fc4_conformance::json::Json;
use fc4_core::persistence_manager::*;
use std::collections::BTreeMap;

pub fn n(j: &Json, name: &str) -> u32 {
    j.get(name).and_then(Json::as_i64).unwrap() as u32
}
pub fn number(j: &Json) -> u32 {
    j.as_i64().unwrap() as u32
}
pub fn flag(j: &Json, name: &str, default: bool) -> bool {
    j.get(name).and_then(Json::as_bool).unwrap_or(default)
}
pub fn array<'a>(j: &'a Json, name: &str) -> &'a [Json] {
    j.get(name).and_then(Json::as_array).unwrap_or(&[])
}
pub fn key(j: &Json) -> Key {
    let a = j.as_array().unwrap();
    Key::new(number(&a[0]), number(&a[1]), number(&a[2]))
}
pub fn keys(j: &Json, name: &str) -> Vec<Key> {
    array(j, name).iter().map(key).collect()
}
pub fn object(j: &Json, name: &str) -> Object {
    Object(u64::from(n(j, name)))
}
pub fn filter(j: &Json) -> Option<Filter> {
    if j == &Json::Null {
        None
    } else if let Some(ty) = j.get("type") {
        Some(Filter::Type(number(ty)))
    } else {
        Some(Filter::Foreign(object(j, "foreign")))
    }
}

#[derive(Clone)]
struct Resource {
    key: Key,
    ty: u32,
    reject: Vec<u32>,
}
#[derive(Clone)]
struct Factory {
    resource: Object,
    success: bool,
    write: bool,
    false_output: bool,
}
#[derive(Clone)]
struct Segment {
    id: u32,
    keys: Vec<Key>,
    record: Object,
    open: bool,
    create: bool,
    commit: bool,
    false_record: bool,
}
#[derive(Clone)]
struct Cache {
    entries: Vec<(Key, Object)>,
    accepts: bool,
    insert: bool,
    erase: bool,
    update: bool,
    auto: u8,
    purge: u8,
}
#[derive(Default)]
pub struct Host {
    pub refs: BTreeMap<Object, u32>,
    resources: BTreeMap<Object, Resource>,
    factories: BTreeMap<Object, Factory>,
    segments: BTreeMap<Object, Segment>,
    caches: BTreeMap<Object, Cache>,
    filters: BTreeMap<Object, u32>,
    pub calls: Vec<String>,
    pub events: Vec<String>,
    pub locks: Vec<(Lock, bool)>,
    pub created_types: Vec<u32>,
    pub arguments: Vec<[u32; 2]>,
    pub opened: Vec<(Object, Key, u32)>,
    pub created_records: Vec<Key>,
    pub updates: Vec<(Object, u32)>,
    pub com: Option<Object>,
    pub com_result: bool,
    pub record_close_result: bool,
}
impl Host {
    pub fn from_case(j: &Json) -> Self {
        let mut h = Self {
            record_close_result: flag(j, "record_close", true),
            ..Self::default()
        };
        for r in array(j, "resources") {
            let id = object(r, "id");
            let k = key(r.get("key").unwrap());
            h.refs.insert(id, r.get("refs").map_or(0, number));
            h.resources.insert(
                id,
                Resource {
                    key: k,
                    ty: r.get("type").map_or(k.ty, number),
                    reject: array(r, "reject").iter().map(number).collect(),
                },
            );
        }
        for f in array(j, "factories") {
            let id = object(f, "id");
            h.refs.insert(id, f.get("refs").map_or(0, number));
            h.factories.insert(
                id,
                Factory {
                    resource: object(f, "resource"),
                    success: flag(f, "success", true),
                    write: flag(f, "write", true),
                    false_output: flag(f, "false_output", false),
                },
            );
        }
        for s in array(j, "segments") {
            let id = object(s, "id");
            h.refs.insert(id, 0);
            h.segments.insert(
                id,
                Segment {
                    id: n(s, "identifier"),
                    keys: keys(s, "keys"),
                    record: object(s, "record"),
                    open: flag(s, "open", true),
                    create: flag(s, "create", true),
                    commit: flag(s, "commit", true),
                    false_record: flag(s, "false_record", false),
                },
            );
        }
        for c in array(j, "caches") {
            let id = object(c, "id");
            h.refs.insert(id, c.get("refs").map_or(0, number));
            let entries = array(c, "entries")
                .iter()
                .map(|e| {
                    (
                        key(&e.as_array().unwrap()[0]),
                        Object(u64::from(number(&e.as_array().unwrap()[1]))),
                    )
                })
                .collect();
            h.caches.insert(
                id,
                Cache {
                    entries,
                    accepts: flag(c, "accepts", true),
                    insert: flag(c, "insert", true),
                    erase: flag(c, "erase", true),
                    update: flag(c, "update", true),
                    auto: c.get("auto").map_or(0, number) as u8,
                    purge: c.get("purge").map_or(0, number) as u8,
                },
            );
        }
        for f in array(j, "filters") {
            h.filters.insert(object(f, "id"), n(f, "type"));
        }
        h
    }
    fn call(&mut self, text: String) {
        self.events.push(text.clone());
        self.calls.push(text);
    }
    pub fn set(&mut self, j: &Json) {
        let id = object(j, "object");
        let field = j.get("field").unwrap().as_str().unwrap();
        let value = flag(j, "value", true);
        match field {
            "open" => self.segments.get_mut(&id).unwrap().open = value,
            "create" => self.segments.get_mut(&id).unwrap().create = value,
            "commit" => self.segments.get_mut(&id).unwrap().commit = value,
            "false_record" => self.segments.get_mut(&id).unwrap().false_record = value,
            "success" => self.factories.get_mut(&id).unwrap().success = value,
            "write" => self.factories.get_mut(&id).unwrap().write = value,
            _ => panic!("unknown primitive setting {field}"),
        }
    }
    pub fn reset(&mut self) {
        self.calls.clear();
        self.events.clear();
        self.locks.clear();
        self.updates.clear();
    }
}
impl Primitives for Host {
    fn lock(&mut self, lock: Lock, enter: bool) {
        self.events
            .push(format!("{}{:?}", if enter { "lock+" } else { "lock-" }, lock));
        self.locks.push((lock, enter));
    }
    fn add_ref(&mut self, o: Object) -> u32 {
        self.events.push(format!("add:{}", o.0));
        let count = self.refs.entry(o).or_default();
        *count = count.wrapping_add(1);
        *count
    }
    fn release(&mut self, o: Object) -> u32 {
        self.events.push(format!("release:{}", o.0));
        let count = self.refs.entry(o).or_default();
        assert!(*count > 0, "Release without reference: {o:?}");
        *count -= 1;
        *count
    }
    fn resource_key(&mut self, r: Object) -> Key {
        self.resources[&r].key
    }
    fn resource_set_key(&mut self, r: Object, k: Key) {
        self.resources.get_mut(&r).unwrap().key = k;
    }
    fn resource_type(&mut self, r: Object) -> u32 {
        self.resources[&r].ty
    }
    fn resource_count(&mut self, r: Object) -> u32 {
        self.refs[&r]
    }
    fn resource_query(&mut self, r: Object, iid: u32, out: &mut Option<Object>) -> bool {
        self.call(format!("query:{}:{iid:#x}", r.0));
        if self.resources[&r].reject.contains(&iid) {
            return false;
        }
        *out = Some(r);
        self.add_ref(r);
        true
    }
    fn com_create_factory(&mut self, class: u32, iid: u32, out: &mut Option<Object>) -> bool {
        assert_eq!(iid, FACTORY_IID);
        self.call(format!("com:{class}"));
        if let Some(f) = self.com {
            *out = Some(f);
            self.add_ref(f);
        }
        self.com_result
    }
    fn factory_create_type(&mut self, f: Object, ty: u32, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool {
        assert_eq!(iid, RESOURCE_IID);
        self.call(format!("type:{}:{ty}", f.0));
        self.created_types.push(ty);
        self.arguments.push(args);
        let f = self.factories[&f].clone();
        if f.success || f.false_output {
            *out = Some(f.resource);
            self.add_ref(f.resource);
        }
        f.success
    }
    fn factory_create_record(
        &mut self,
        f: Object,
        record: Object,
        iid: u32,
        out: &mut Option<Object>,
        args: [u32; 2],
    ) -> bool {
        assert_eq!(iid, RESOURCE_IID);
        self.call(format!("record:{}:{}", f.0, record.0));
        self.arguments.push(args);
        let f = self.factories[&f].clone();
        if f.success || f.false_output {
            *out = Some(f.resource);
            self.add_ref(f.resource);
        }
        f.success
    }
    fn factory_write(&mut self, f: Object, r: Object, record: Object) -> bool {
        self.call(format!("write:{}:{}:{}", f.0, r.0, record.0));
        self.factories[&f].write
    }
    fn segment_id(&mut self, s: Object) -> u32 {
        self.segments[&s].id
    }
    fn segment_contains(&mut self, s: Object, k: Key) -> bool {
        self.call(format!("contains:{}", s.0));
        self.segments[&s].keys.contains(&k)
    }
    fn segment_enumerate(&mut self, s: Object, list: &mut KeyList, f: Option<Filter>) -> u32 {
        self.call(format!("segment_enum:{}", s.0));
        let mut count = 0;
        for k in self.segments[&s].keys.clone() {
            if f.map_or(true, |f| f.accepts(self, k)) {
                list.insert(k);
                count += 1;
            }
        }
        count
    }
    fn segment_open(&mut self, s: Object, k: Key, out: &mut Option<Object>, flags: u32) -> bool {
        self.call(format!("open:{}:{flags}", s.0));
        self.opened.push((s, k, flags));
        let s = self.segments[&s].clone();
        if s.open || s.false_record {
            *out = Some(s.record);
            self.add_ref(s.record);
        }
        s.open
    }
    fn segment_create(&mut self, s: Object, k: Key, out: &mut Option<Object>) -> bool {
        self.call(format!("create_record:{}", s.0));
        self.created_records.push(k);
        let s = self.segments[&s].clone();
        if s.create || s.false_record {
            *out = Some(s.record);
            self.add_ref(s.record);
        }
        s.create
    }
    fn segment_commit(&mut self, s: Object, record: &mut Option<Object>) -> bool {
        self.call(format!("commit:{}", s.0));
        let result = self.segments[&s].commit;
        if result {
            self.release(record.unwrap());
        } // Deliberately leave cell: manager must null it.
        result
    }
    fn segment_close(&mut self, s: Object, _record: Object) -> bool {
        self.call(format!("segment_close:{}", s.0));
        false
    }
    fn segment_abort(&mut self, s: Object, record: &mut Option<Object>) -> bool {
        self.call(format!("abort:{}", s.0));
        self.release(record.take().unwrap());
        true
    }
    fn record_close(&mut self, r: Object) -> bool {
        self.call(format!("close:{}", r.0));
        self.record_close_result
    }
    fn filter_accepts(&mut self, f: Object, k: Key) -> bool {
        self.call(format!("filter:{}", f.0));
        self.filters[&f] == k.ty
    }
    fn cache_get(&mut self, c: Object, k: Key, out: &mut Option<Object>) -> bool {
        self.call(format!("get:{}", c.0));
        let r = self.caches[&c]
            .entries
            .iter()
            .find(|(key, _)| *key == k)
            .map(|(_, r)| *r);
        if let Some(r) = r {
            *out = Some(r);
            self.add_ref(r);
            true
        } else {
            false
        }
    }
    fn cache_insert(&mut self, c: Object, k: Key, r: Object) -> bool {
        self.call(format!("insert:{}", c.0));
        let result = self.caches[&c].insert;
        if result {
            self.add_ref(r);
            self.caches.get_mut(&c).unwrap().entries.push((k, r));
        }
        result
    }
    fn cache_erase(&mut self, c: Object, k: Key) -> bool {
        self.call(format!("erase:{}", c.0));
        if !self.caches[&c].erase {
            return false;
        }
        let i = self.caches[&c].entries.iter().position(|(key, _)| *key == k).unwrap();
        let r = self.caches.get_mut(&c).unwrap().entries.remove(i).1;
        self.release(r);
        true
    }
    fn cache_contains(&mut self, c: Object, k: Key) -> bool {
        self.call(format!("cache_contains:{}", c.0));
        self.caches[&c].entries.iter().any(|(key, _)| *key == k)
    }
    fn cache_accepts(&mut self, c: Object, _k: Key) -> bool {
        self.call(format!("accept:{}", c.0));
        self.caches[&c].accepts
    }
    fn cache_enumerate(&mut self, c: Object, list: &mut KeyList, f: Option<Filter>) -> u32 {
        self.call(format!("cache_enum:{}", c.0));
        let mut count = 0;
        for (k, _) in self.caches[&c].entries.clone() {
            if f.map_or(true, |f| f.accepts(self, k)) {
                list.insert(k);
                count += 1;
            }
        }
        count
    }
    fn cache_update(&mut self, c: Object, argument: u32) -> bool {
        self.call(format!("update:{}:{argument}", c.0));
        self.updates.push((c, argument));
        self.caches[&c].update
    }
    fn cache_auto_purge(&mut self, c: Object) -> u8 {
        self.call(format!("auto:{}", c.0));
        self.caches[&c].auto
    }
    fn cache_set_auto_purge(&mut self, c: Object, v: u8) {
        self.call(format!("set_auto:{}:{v}", c.0));
        self.caches.get_mut(&c).unwrap().auto = v;
    }
    fn cache_purge(&mut self, c: Object) -> u8 {
        self.call(format!("purge:{}", c.0));
        self.caches[&c].purge
    }
    fn cache_shutdown(&mut self, c: Object) {
        self.call(format!("shutdown:{}", c.0));
    }
}
