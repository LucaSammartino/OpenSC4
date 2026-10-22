// SPDX-License-Identifier: GPL-3.0-or-later
//! Resource-only transport to the production persistence manager. Inputs are
//! fixture keys/commands, never original-manager answers. This host implements
//! only foreign resource primitives; registry/cache decisions remain in core.
use fc4_core::persistence_manager::{Filter, Key, KeyList, Object, PersistManager, Primitives, RESOURCE_IID};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct fc4_pm_key {
    pub ty: u32,
    pub group: u32,
    pub instance: u32,
}
impl From<fc4_pm_key> for Key {
    fn from(k: fc4_pm_key) -> Self {
        Key::new(k.ty, k.group, k.instance)
    }
}
#[repr(C)]
#[derive(Default)]
pub struct fc4_pm_result {
    pub status: u32,
    pub identity: u32,
    pub references: [u32; 3],
    pub unexpected: u32,
    pub current_keys: [fc4_pm_key; 3],
}
struct Resource {
    key: Key,
    references: u32,
}
struct Resources {
    resources: [Resource; 3],
    unexpected: u32,
}
// Unsupported foreign operations are explicitly counted. They are never
// forwarded to the original manager, silently supplied, or treated as credit.
macro_rules! unsupported {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?)) => {
        fn $name(&mut self, $($arg: $ty),*) {
            $(let _ = $arg;)*
            self.unexpected += 1;
        }
    };
    ($name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty, $value:expr) => {
        fn $name(&mut self, $($arg: $ty),*) -> $ret {
            $(let _ = $arg;)*
            self.unexpected += 1;
            $value
        }
    };
}
impl Primitives for Resources {
    fn add_ref(&mut self, object: Object) -> u32 {
        let r = &mut self.resources[object.0 as usize - 1];
        r.references += 1;
        r.references
    }
    fn release(&mut self, object: Object) -> u32 {
        let r = &mut self.resources[object.0 as usize - 1];
        r.references -= 1;
        r.references
    }
    fn resource_key(&mut self, resource: Object) -> Key {
        self.resources[resource.0 as usize - 1].key
    }
    fn resource_set_key(&mut self, resource: Object, key: Key) {
        self.resources[resource.0 as usize - 1].key = key;
    }
    fn resource_type(&mut self, resource: Object) -> u32 {
        self.resource_key(resource).ty
    }
    fn resource_count(&mut self, resource: Object) -> u32 {
        self.resources[resource.0 as usize - 1].references
    }
    fn resource_query(&mut self, resource: Object, iid: u32, out: &mut Option<Object>) -> bool {
        if iid != RESOURCE_IID && iid != 1 {
            return false;
        }
        self.add_ref(resource);
        *out = Some(resource);
        true
    }
    unsupported!(
        com_create_factory(class: u32, iid: u32, out: &mut Option<Object>) -> bool,
        false
    );
    unsupported!(
        factory_create_type(factory: Object, ty: u32, iid: u32, out: &mut Option<Object>, args: [u32; 2]) -> bool,
        false
    );
    unsupported!(
        factory_create_record(
            factory: Object,
            record: Object,
            iid: u32,
            out: &mut Option<Object>,
            args: [u32; 2],
        ) -> bool,
        false
    );
    unsupported!(
        factory_write(factory: Object, resource: Object, record: Object) -> bool,
        false
    );
    unsupported!(segment_id(segment: Object) -> u32, 0);
    unsupported!(segment_contains(segment: Object, key: Key) -> bool, false);
    unsupported!(
        segment_enumerate(segment: Object, list: &mut KeyList, filter: Option<Filter>) -> u32,
        0
    );
    unsupported!(
        segment_open(segment: Object, key: Key, out: &mut Option<Object>, flags: u32) -> bool,
        false
    );
    unsupported!(
        segment_create(segment: Object, key: Key, out: &mut Option<Object>) -> bool,
        false
    );
    unsupported!(
        segment_commit(segment: Object, record: &mut Option<Object>) -> bool,
        false
    );
    unsupported!(segment_close(segment: Object, record: Object) -> bool, false);
    unsupported!(
        segment_abort(segment: Object, record: &mut Option<Object>) -> bool,
        false
    );
    unsupported!(record_close(record: Object) -> bool, false);
    unsupported!(filter_accepts(filter: Object, key: Key) -> bool, false);
    unsupported!(
        cache_get(cache: Object, key: Key, out: &mut Option<Object>) -> bool,
        false
    );
    unsupported!(cache_insert(cache: Object, key: Key, resource: Object) -> bool, false);
    unsupported!(cache_erase(cache: Object, key: Key) -> bool, false);
    unsupported!(cache_contains(cache: Object, key: Key) -> bool, false);
    unsupported!(cache_accepts(cache: Object, key: Key) -> bool, false);
    unsupported!(
        cache_enumerate(cache: Object, list: &mut KeyList, filter: Option<Filter>) -> u32,
        0
    );
    unsupported!(cache_update(cache: Object, argument: u32) -> bool, false);
    unsupported!(cache_auto_purge(cache: Object) -> u8, 0);
    unsupported!(cache_set_auto_purge(cache: Object, value: u8));
    unsupported!(cache_purge(cache: Object) -> u8, 0);
    unsupported!(cache_shutdown(cache: Object));
}
#[allow(non_camel_case_types)]
pub struct fc4_pm_session {
    manager: PersistManager<Resources>,
}

/// Three valid independent fixture keys; no native state is accepted here.
#[no_mangle]
pub unsafe extern "C" fn fc4_pm_create(keys: *const fc4_pm_key) -> *mut fc4_pm_session {
    catch_unwind(AssertUnwindSafe(|| {
        if keys.is_null() {
            return std::ptr::null_mut();
        }
        let keys = std::slice::from_raw_parts(keys, 3);
        let resources = std::array::from_fn(|i| Resource {
            key: keys[i].into(),
            references: 1,
        });
        Box::into_raw(Box::new(fc4_pm_session {
            manager: PersistManager::new(Resources {
                resources,
                unexpected: 0,
            }),
        }))
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// op: 0 register (index 3 = null), 1 register-as, 2 contains, 3 remove,
/// 4 cached load/QI, 5 ownership snapshot, 6 uncached load. identity is 1..3, with 3 also
/// the independent sentinel on a failed QI. Caller supplies cached-hit loads.
#[no_mangle]
pub unsafe extern "C" fn fc4_pm_step(
    session: *mut fc4_pm_session,
    op: u32,
    index: u32,
    key: fc4_pm_key,
    iid: u32,
    out: *mut fc4_pm_result,
) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        if session.is_null() || out.is_null() || op > 6 || index > 3 || (op == 1 && index == 3) {
            return 0;
        }
        let m = &mut (*session).manager;
        let object = (index < 3).then_some(Object(u64::from(index) + 1));
        let mut identity = 0;
        let status = match op {
            0 => m.register_resource(object),
            1 => m.register_resource_as(object.unwrap(), key.into()),
            2 => m.is_cached(key.into()),
            3 => m.unregister_resource(key.into()),
            4 | 6 => {
                let mut result = Some(Object(3));
                let ok = if op == 4 {
                    m.load(key.into(), iid, &mut result, [0, 0])
                } else {
                    m.load_uncached(key.into(), iid, &mut result, [0, 0])
                };
                identity = result.map_or(0, |o| o.0 as u32);
                if ok {
                    if let Some(o) = result {
                        m.objects.host.release(o);
                    }
                }
                ok
            }
            _ => true,
        };
        *out = fc4_pm_result {
            status: u32::from(status),
            identity,
            references: std::array::from_fn(|i| m.objects.host.resources[i].references),
            unexpected: m.objects.host.unexpected,
            current_keys: std::array::from_fn(|i| {
                let k = m.objects.host.resources[i].key;
                fc4_pm_key {
                    ty: k.ty,
                    group: k.group,
                    instance: k.instance,
                }
            }),
        };
        1
    }))
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_pm_destroy(session: *mut fc4_pm_session) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !session.is_null() {
            let mut session = Box::from_raw(session);
            session.manager.shutdown(); // Our isolated Rust manager only.
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_ownership_and_failed_query_sentinel() {
        let keys = [fc4_pm_key {
            ty: 9,
            group: 8,
            instance: 7,
        }; 3];
        unsafe {
            let s = fc4_pm_create(keys.as_ptr());
            let mut out = fc4_pm_result::default();
            assert_eq!(fc4_pm_step(s, 0, 0, keys[0], 0, &mut out), 1);
            assert_eq!(out.references, [2, 1, 1]);
            fc4_pm_step(s, 1, 1, keys[0], 0, &mut out);
            assert_eq!(out.references, [2, 1, 1]);
            fc4_pm_step(s, 4, 0, keys[0], 0xdeadbeef, &mut out);
            assert_eq!((out.status, out.identity, out.unexpected), (0, 3, 0));
            assert_eq!(out.references, [2, 1, 1]);
            fc4_pm_step(s, 3, 0, keys[0], 0, &mut out);
            assert_eq!(out.references, [1, 1, 1]);
            fc4_pm_destroy(s);
        }
    }
}
