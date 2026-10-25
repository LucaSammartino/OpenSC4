// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use fc4_core::regional_city::{ConnectionHandle, DealHandle, Lifecycle};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering::SeqCst};
use std::sync::Arc;

#[derive(Default)]
struct Object {
    refs: AtomicI32,
    retains: AtomicU32,
    releases: AtomicU32,
    x: AtomicI32,
    z: AtomicI32,
    flags: AtomicU32,
    repairs: AtomicU32,
}
unsafe fn object<'a>(p: *mut c_void) -> &'a Object {
    &*p.cast::<Object>()
}
unsafe extern "C" fn retain(p: *mut c_void) {
    object(p).refs.fetch_add(1, SeqCst);
    object(p).retains.fetch_add(1, SeqCst);
}
unsafe extern "C" fn release(p: *mut c_void) {
    object(p).refs.fetch_sub(1, SeqCst);
    object(p).releases.fetch_add(1, SeqCst);
}
unsafe extern "C" fn kind(_: *mut c_void) -> u32 {
    2
}
unsafe extern "C" fn pair(p: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = object(p).x.load(SeqCst);
    *z = object(p).z.load(SeqCst);
}
unsafe extern "C" fn set_pair(p: *mut c_void, x: i32, z: i32) {
    object(p).x.store(x, SeqCst);
    object(p).z.store(z, SeqCst);
}
unsafe extern "C" fn repair(p: *mut c_void, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32) {
    object(p).repairs.fetch_add(1, SeqCst);
}
unsafe extern "C" fn amount(_: *mut c_void) -> i32 {
    25
}
unsafe extern "C" fn flags(p: *mut c_void) -> u32 {
    object(p).flags.load(SeqCst)
}
unsafe extern "C" fn set_flags(p: *mut c_void, flags: u32, on: bool) -> bool {
    if on {
        object(p).flags.fetch_or(flags, SeqCst);
    } else {
        object(p).flags.fetch_and(!flags, SeqCst);
    }
    true
}
const CONNECTION: ConnectionCallbacks = ConnectionCallbacks {
    retain,
    release,
    conn_type: kind,
    cell: pair,
    set_cell: set_pair,
    repair,
};
const DEAL: DealCallbacks = DealCallbacks {
    retain,
    release,
    city_side: pair,
    other_side: pair,
    set_city_side: set_pair,
    set_other_side: set_pair,
    deal_type: kind,
    amount,
    flags,
    set_flags,
};
struct Life;
impl Lifecycle for Life {
    fn subscribe(&mut self, _: u32, _: bool) {}
}

#[test]
fn host_references_are_live_cloned_and_released_exactly_once() {
    let connection = Object::default();
    let deal = Object::default();
    let cp = &connection as *const Object as *mut c_void;
    let dp = &deal as *const Object as *mut c_void;
    unsafe {
        let owner = fc4_regionalcity_owner_create(2);
        assert!(fc4_regionalcity_owner_connections(owner, &cp, 1, &CONNECTION));
        assert!(fc4_regionalcity_owner_deals(owner, false, &dp, 1, &DEAL));
        assert!(fc4_regionalcity_owner_deals(owner, true, &dp, 1, &DEAL));
        assert_eq!(connection.refs.load(SeqCst), 1);
        assert_eq!(deal.refs.load(SeqCst), 2);
        // A host change is visible through the existing handle, with no snapshot.
        connection.x.store(7, SeqCst);
        {
            let mut city = (*owner).0.lock().unwrap();
            let found = city.find_connection(2, 7, 0).unwrap();
            found.set_cell(-1, 9);
            assert_eq!(found.cell(), (-1, 9));
            let mut list = Vec::new();
            city.get_all_connections(&mut list);
            assert_eq!(connection.refs.load(SeqCst), 2);
            drop(list);
            let mut deals = Vec::new();
            city.get_local_deals(&mut deals);
            assert_eq!(deal.refs.load(SeqCst), 3);
            assert_eq!(deals[0].amount(), 25);
            assert!(deals[0].set_flags(4, true));
            assert_eq!(deals[0].flags(), 4);
            drop(deals);
            city.init(&mut Life);
            city.set_position(1, 2, true);
            assert_eq!(connection.repairs.load(SeqCst), 1);
            city.shutdown(&mut Life);
        }
        assert_eq!(connection.refs.load(SeqCst), 0);
        assert_eq!(deal.refs.load(SeqCst), 0);
        fc4_regionalcity_owner_destroy(owner);
        assert_eq!(connection.retains.load(SeqCst), connection.releases.load(SeqCst));
        assert_eq!(deal.retains.load(SeqCst), deal.releases.load(SeqCst));
    }
}

#[test]
fn list_rejection_rolls_back_and_replacement_releases_old_handles() {
    let obj = Object::default();
    let p = &obj as *const Object as *mut c_void;
    unsafe {
        let owner = fc4_regionalcity_owner_create(0);
        assert!(fc4_regionalcity_owner_connections(owner, &p, 1, &CONNECTION));
        assert!(!fc4_regionalcity_owner_connections(
            owner,
            [p, std::ptr::null_mut()].as_ptr(),
            2,
            &CONNECTION
        ));
        assert_eq!(obj.refs.load(SeqCst), 1);
        assert_eq!((*owner).0.lock().unwrap().connections.len(), 1);
        assert!(fc4_regionalcity_owner_connections(
            owner,
            std::ptr::null(),
            0,
            &CONNECTION
        ));
        assert_eq!(obj.refs.load(SeqCst), 0);
        assert!(fc4_regionalcity_owner_deals(owner, true, &p, 1, &DEAL));
        assert!(!fc4_regionalcity_owner_deals(
            owner,
            false,
            [p, std::ptr::null_mut()].as_ptr(),
            2,
            &DEAL
        ));
        assert_eq!(obj.refs.load(SeqCst), 1);
        fc4_regionalcity_owner_destroy(owner);
        assert_eq!(obj.refs.load(SeqCst), 0);
        assert_eq!(obj.retains.load(SeqCst), obj.releases.load(SeqCst));
    }
}

#[test]
fn getters_and_setters_share_a_thread_safe_owner() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Owner>();
    let owner: Arc<Owner> = unsafe { Arc::from(Box::from_raw(fc4_regionalcity_owner_create(1))) };
    let other = owner.clone();
    let writer = std::thread::spawn(move || {
        for i in 0..1000 {
            assert!(unsafe { fc4_regionalcity_owner_set(Arc::as_ptr(&other).cast_mut(), 17, 0, i as f64) });
        }
    });
    for _ in 0..1000 {
        let mut value = 0.0;
        assert!(unsafe { fc4_regionalcity_owner_get(Arc::as_ptr(&owner), 17, 0, &mut value) });
        assert!((0.0..=999.0).contains(&value));
    }
    writer.join().unwrap();
}
