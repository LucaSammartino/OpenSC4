// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The occupant manager through the C ABI (docs/specs/occupant-manager.md):
//! a fake, thread-safe occupant world behind `fc4_occmgr_host`.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::occupant_manager::*;
use crate::test_streams::{istream, ostream, In, Out};

const INSERT: u32 = 0x99EF_1142;
const REMOVE: u32 = 0x99EF_1143;
const CREATE: u32 = 0x99EF_1140;
const FLAG_INSERTED: u32 = 1;
const IID_OCCUPANT: u32 = 0x26BB_767E;

#[derive(Clone, Default)]
struct Occ {
    bbox: fc4_occ_bbox,
    om: Option<fc4_occ_cell_box>,
    flags: u32,
    typ: u32,
    no_holder: bool,
    inited: bool,
}

#[derive(Default)]
struct State {
    occs: BTreeMap<u64, Occ>,
    refs: BTreeMap<u64, i32>,
    /// Events with the manager's lock depth on the calling thread.
    log: Vec<(String, u32)>,
    /// Occupants the host filter 0xF1 includes.
    allowed: BTreeSet<u64>,
    resolved: Vec<(u64, u32)>,
}

#[derive(Default)]
struct World {
    s: Mutex<State>,
    m: AtomicPtr<fc4_occmgr>,
}

impl World {
    fn st(&self) -> std::sync::MutexGuard<'_, State> {
        self.s.lock().unwrap()
    }
    fn depth(&self) -> u32 {
        // SAFETY: the pointer is a live manager (or NULL) during the test.
        unsafe { fc4_occmgr_lock_depth(self.m.load(Ordering::SeqCst)) }
    }
    fn ev(&self, e: String) {
        let d = self.depth();
        self.st().log.push((e, d));
    }
    fn take(&self) -> Vec<(String, u32)> {
        std::mem::take(&mut self.st().log)
    }
    fn names(&self) -> Vec<String> {
        self.take().into_iter().map(|e| e.0).collect()
    }
    fn refs(&self, o: u64) -> i32 {
        self.st().refs.get(&o).copied().unwrap_or(0)
    }
    /// An occupant with the box x0..x1, z0..z1 (y 0..10).
    fn add(&self, id: u64, typ: u32, x: (f32, f32), z: (f32, f32)) {
        self.st().occs.insert(
            id,
            Occ {
                bbox: fc4_occ_bbox {
                    min: [x.0, 0.0, z.0],
                    max: [x.1, 10.0, z.1],
                },
                typ,
                ..Occ::default()
            },
        );
    }
}

fn w<'a>(u: *mut c_void) -> &'a World {
    // SAFETY: `user` is the test's World, alive for the test.
    unsafe { &*(u as *const World) }
}

unsafe extern "C" fn add_ref(u: *mut c_void, o: u64) {
    *w(u).st().refs.entry(o).or_default() += 1;
}
unsafe extern "C" fn release(u: *mut c_void, o: u64) {
    *w(u).st().refs.entry(o).or_default() -= 1;
}
unsafe extern "C" fn init(u: *mut c_void, o: u64) {
    let first = {
        let mut s = w(u).st();
        let e = s.occs.entry(o).or_default();
        !std::mem::replace(&mut e.inited, true)
    };
    w(u).ev(format!("init {o}"));
    if first {
        w(u).ev(format!("post {o} {CREATE:08X} 0"));
    }
}
unsafe extern "C" fn shutdown(u: *mut c_void, o: u64) {
    w(u).ev(format!("shutdown {o}"));
}
unsafe extern "C" fn has_property_holder(u: *mut c_void, o: u64) -> bool {
    w(u).st().occs.get(&o).is_some_and(|x| !x.no_holder)
}
unsafe extern "C" fn occupant_type(u: *mut c_void, o: u64) -> u32 {
    w(u).st().occs.get(&o).map_or(0, |x| x.typ)
}
unsafe extern "C" fn bounding_box(u: *mut c_void, o: u64, out: *mut fc4_occ_bbox) {
    if let Some(x) = w(u).st().occs.get(&o) {
        *out = x.bbox;
    }
}
unsafe extern "C" fn post_message(u: *mut c_void, o: u64, t: u32, d4: u32) {
    w(u).ev(format!("post {o} {t:08X} {d4}"));
}
unsafe extern "C" fn manager_bbox(u: *mut c_void, o: u64, out: *mut fc4_occ_cell_box) -> bool {
    match w(u).st().occs.get(&o).and_then(|x| x.om) {
        Some(b) => {
            *out = b;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn set_manager_bbox(u: *mut c_void, o: u64, b: *const fc4_occ_cell_box) {
    let b = *b;
    w(u).st().occs.entry(o).or_default().om = Some(b);
    w(u).ev(format!("set_box {o} {} {} {} {}", b.x0, b.z0, b.x1, b.z1));
}
unsafe extern "C" fn set_flag(u: *mut c_void, o: u64, f: u32) {
    w(u).st().occs.entry(o).or_default().flags |= f;
    w(u).ev(format!("set_flag {o} {f}"));
}
unsafe extern "C" fn clear_flag(u: *mut c_void, o: u64, f: u32) {
    w(u).st().occs.entry(o).or_default().flags &= !f;
    w(u).ev(format!("clear_flag {o} {f}"));
}
unsafe extern "C" fn is_flag_set(u: *mut c_void, o: u64, f: u32) -> bool {
    w(u).st().occs.get(&o).is_some_and(|x| x.flags & f != 0)
}
unsafe extern "C" fn filter_includes(u: *mut c_void, filter: u64, o: u64) -> bool {
    filter == 0xF1 && w(u).st().allowed.contains(&o)
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, token: u64, iid: u32) -> u64 {
    let mut s = w(u).st();
    s.resolved.push((token, iid));
    if s.occs.contains_key(&token) {
        *s.refs.entry(token).or_default() += 1;
        token
    } else {
        0
    }
}

fn host(world: &World) -> fc4_occmgr_host {
    fc4_occmgr_host {
        struct_size: std::mem::size_of::<fc4_occmgr_host>() as u32,
        user: world as *const World as *mut c_void,
        add_ref: Some(add_ref),
        release: Some(release),
        init: Some(init),
        shutdown: Some(shutdown),
        has_property_holder: Some(has_property_holder),
        occupant_type: Some(occupant_type),
        bounding_box: Some(bounding_box),
        post_message: Some(post_message),
        manager_bbox: Some(manager_bbox),
        set_manager_bbox: Some(set_manager_bbox),
        set_flag: Some(set_flag),
        clear_flag: Some(clear_flag),
        is_flag_set: Some(is_flag_set),
        filter_includes: Some(filter_includes),
        resolve_ref: Some(resolve_ref),
    }
}

/// A manager over `world`, sized 1024 m and initialised.
fn make(world: &World) -> *mut fc4_occmgr {
    let h = host(world);
    // SAFETY: valid host table; the World outlives the manager.
    unsafe {
        let m = fc4_occmgr_create(&h);
        assert!(!m.is_null());
        world.m.store(m, Ordering::SeqCst);
        assert!(fc4_occmgr_set_city_size(m, 1024.0, 1024.0));
        assert!(fc4_occmgr_init(m));
        m
    }
}

fn destroy(world: &World, m: *mut fc4_occmgr) {
    world.m.store(std::ptr::null_mut(), Ordering::SeqCst);
    // SAFETY: a live manager, no other call running.
    unsafe { fc4_occmgr_destroy(m) }
}

fn cell(m: *const fc4_occmgr, x: i32, z: i32) -> Option<Vec<(u64, bool)>> {
    let mut o = [0u64; 32];
    let mut s = [0u8; 32];
    // SAFETY: buffers of 32.
    let n = unsafe { fc4_occmgr_cell(m, x, z, o.as_mut_ptr(), s.as_mut_ptr(), 32) };
    (n >= 0).then(|| (0..n as usize).map(|i| (o[i], s[i] != 0)).collect())
}

unsafe extern "C" fn push(ctx: *mut c_void, o: u64) {
    (*(ctx as *mut Vec<u64>)).push(o);
}

#[test]
fn layouts_and_identity() {
    assert_eq!(std::mem::size_of::<fc4_occ_bbox>(), 24);
    assert_eq!(std::mem::size_of::<fc4_occ_cell_box>(), 4);
    assert_eq!(std::mem::size_of::<fc4_occ_filter>(), 16);
    assert_eq!(std::mem::size_of::<fc4_occmgr_state>(), 56);
    assert_eq!(fc4_occmgr_query_interface(1), 0);
    assert_eq!(fc4_occmgr_query_interface(0xC6BB_8334), 0);
    assert_eq!(fc4_occmgr_query_interface(0xE4FD_A3D4), 4);
    assert_eq!(fc4_occmgr_query_interface(0x4522_94AA), 8);
    assert_eq!(fc4_occmgr_query_interface(0x1234), -1);
    let world = World::default();
    let mut h = host(&world);
    h.struct_size -= 1;
    // SAFETY: a valid (too small) table; NULL handles are allowed.
    unsafe {
        assert!(fc4_occmgr_create(&h).is_null());
        assert!(fc4_occmgr_create(std::ptr::null()).is_null());
        assert!(!fc4_occmgr_insert(std::ptr::null_mut(), 1, 0));
        assert_eq!(
            fc4_occmgr_cell(std::ptr::null(), 64, 64, std::ptr::null_mut(), std::ptr::null_mut(), 0),
            -1
        );
    }
    let m = make(&world);
    // SAFETY: live manager and valid outputs.
    unsafe {
        let mut st = fc4_occmgr_state::default();
        assert!(fc4_occmgr_get_state(m, &mut st));
        assert_eq!(st.initialized, 1);
        assert_eq!(st.counts, [16, 16, 64, 64]);
        assert_eq!(st.rows, 192);
        assert_eq!((st.size_x, st.size_z), (1024.0, 1024.0));
        let mut b = [0i32; 4];
        assert!(fc4_occmgr_get_cell_bounds(m, b.as_mut_ptr()));
        assert_eq!(b, [64, 64, 79, 79]);
        let (mut x, mut z) = (0i32, 0i32);
        assert!(fc4_occmgr_get_world_cell_count(m, &mut x, &mut z));
        assert_eq!((x, z), (192, 192));
        assert!(fc4_occmgr_get_standard_cell_count(m, &mut x, std::ptr::null_mut()));
        assert_eq!(x, 64);
        let (mut fx, mut fz) = (0f32, 0f32);
        assert!(fc4_occmgr_get_cell_sizes(m, &mut fx, &mut fz));
        assert_eq!((fx, fz), (64.0, 64.0));
        assert_eq!(fc4_occmgr_add_ref(m), 1);
        assert_eq!(fc4_occmgr_release(m), 0);
        assert!(fc4_occmgr_get_state(m, &mut st));
        assert_eq!((st.destroyed, st.initialized), (1, 0));
    }
    destroy(&world, m);
}

#[test]
fn conversions() {
    // SAFETY: valid outputs (NULL where noted).
    unsafe {
        let (mut x, mut z) = (0i32, 5i32);
        // Slot 12 [quirk]: x ignored, out z = 4 * the value already there.
        fc4_occ_world_cell_to_standard_city_cell(70, 66, &mut x, &mut z);
        assert_eq!((x, z), (8, 20));
        fc4_occ_world_cell_to_standard_city_cell(70, 66, &mut x, std::ptr::null_mut());
        assert_eq!(x, 8);
        fc4_occ_standard_city_cell_to_world_cell(-1, 5, &mut x, &mut z);
        assert_eq!((x, z), (64, 65));
        fc4_occ_world_cell_to_manager_cell(70, 64, &mut x, &mut z);
        assert_eq!((x, z), (6, 0));
        fc4_occ_manager_cell_to_world_cell(6, 0, &mut x, &mut z);
        assert_eq!((x, z), (70, 64));
        fc4_occ_position_to_world_cell(-1.0, 63.0, &mut x, &mut z);
        assert_eq!((x, z), (63, 64));
        // The 0.001 bias: −0.064 is already cell 64.
        fc4_occ_position_to_world_cell(-0.064, 63.99, &mut x, &mut z);
        assert_eq!((x, z), (64, 65));
        fc4_occ_position_to_manager_cell(64.0, -64.0, &mut x, &mut z);
        assert_eq!((x, z), (1, -1));
        fc4_occ_position_to_standard_city_cell(16.0, 15.999, &mut x, &mut z);
        assert_eq!((x, z), (1, 0));
        let (mut fx, mut fz) = (0f32, 0f32);
        fc4_occ_world_cell_to_position(65, 63, &mut fx, &mut fz);
        assert_eq!((fx, fz), (64.0, -64.0));
        fc4_occ_manager_cell_to_position(2, 3, &mut fx, &mut fz);
        assert_eq!((fx, fz), (128.0, 192.0));
        fc4_occ_standard_city_cell_to_position(2, 3, &mut fx, &mut fz);
        assert_eq!((fx, fz), (32.0, 48.0));
        let b = fc4_occ_bbox {
            min: [64.0, 0.0, 0.0],
            max: [128.0, 1.0, 0.5],
        };
        let mut c = fc4_occ_cell_box::default();
        assert!(fc4_occ_cell_box_of(&b, &mut c));
        assert_eq!((c.x0, c.z0, c.x1, c.z1), (65, 64, 65, 64));
        let mut cells = [0i32; 16];
        assert_eq!(fc4_occ_bounding_cells(0, 0, cells.as_mut_ptr()), 3);
        assert_eq!(&cells[..6], &[0, 1, 1, 0, 1, 1]);
        assert_eq!(fc4_occ_bounding_cells(5, 5, std::ptr::null_mut()), 8);
    }
}

#[test]
fn insert_remove_message_order() {
    let world = World::default();
    world.add(1, 0x10, (0.0, 128.0), (0.0, 32.0));
    let m = make(&world);
    // SAFETY: live manager.
    unsafe {
        assert!(fc4_occmgr_insert(m, 1, 7));
        assert_eq!(
            world.take(),
            vec![
                ("init 1".into(), 1),
                (format!("post 1 {CREATE:08X} 0"), 1),
                ("set_box 1 64 64 65 64".into(), 1),
                ("set_flag 1 1".into(), 0),
                (format!("post 1 {INSERT:08X} 7"), 0),
            ]
        );
        assert_eq!(world.refs(1), 1);
        assert_eq!(world.st().occs[&1].flags & FLAG_INSERTED, FLAG_INSERTED);
        assert_eq!(cell(m, 64, 64), Some(vec![(1, false)]));
        assert_eq!(cell(m, 65, 64), Some(vec![(1, false)]));
        assert_eq!(cell(m, 66, 64), None);
        assert!(!fc4_occmgr_insert(m, 1, 7));
        assert!(world.take().is_empty());

        let (mut x, mut z) = (-1, -1);
        assert!(fc4_occmgr_find_occupant(m, 1, true, &mut x, &mut z));
        assert_eq!((x, z), (-1, -1), "fast path writes nothing");

        assert!(fc4_occmgr_remove(m, 1, true, 9));
        assert_eq!(
            world.names(),
            vec![
                format!("post 1 {REMOVE:08X} 9"),
                "clear_flag 1 1".into(),
                "shutdown 1".into()
            ]
        );
        assert_eq!(world.refs(1), 0);
        assert_eq!(world.st().occs[&1].flags & FLAG_INSERTED, 0);
        assert!(fc4_occmgr_is_cell_empty(m, 64, 64));
        assert!(!fc4_occmgr_find_occupant(m, 1, true, &mut x, &mut z));
    }
    destroy(&world, m);
}

#[test]
fn queries_filters_and_lists() {
    let world = World::default();
    world.add(1, 0x10, (0.0, 32.0), (0.0, 32.0));
    world.add(2, 0x20, (16.0, 48.0), (0.0, 32.0));
    world.add(3, 0x10, (0.0, 128.0), (0.0, 32.0));
    world.st().allowed.insert(2);
    let m = make(&world);
    // SAFETY: live manager, valid buffers.
    unsafe {
        for o in 1..=3 {
            assert!(fc4_occmgr_insert(m, o, 0));
        }
        world.take();
        assert_eq!(cell(m, 64, 64), Some(vec![(3, false), (2, true), (1, true)]));
        let mut out = 0u64;
        assert!(fc4_occmgr_first_in_cell(m, 64, 64, std::ptr::null(), &mut out));
        assert_eq!(out, 3);
        let host_filter = fc4_occ_filter {
            kind: FC4_OCC_FILTER_HOST,
            occ_type: 0,
            token: 0xF1,
        };
        assert!(fc4_occmgr_first_in_cell(m, 64, 64, &host_filter, &mut out));
        assert_eq!(out, 2);
        assert!(fc4_occmgr_first_in_cell_typed(m, 64, 64, 0x10, &mut out));
        assert_eq!(out, 3);
        assert!(!fc4_occmgr_first_in_cell_typed(m, 64, 64, 0x99, &mut out));
        assert_eq!(out, 0);
        assert!(fc4_occmgr_first_by_position(m, 20.0, 10.0, std::ptr::null(), &mut out));
        assert_eq!(out, 3);
        let mut st = fc4_occmgr_state::default();
        assert!(fc4_occmgr_get_state(m, &mut st));
        assert_eq!((st.filter_present, st.filter_type), (1, 0x99));
        // Each first-occupant hit holds a reference for the caller.
        assert_eq!(world.refs(3), 4);
        let mine = [3u64, 2, 3, 3];
        fc4_occmgr_release_list(m, mine.as_ptr(), mine.len());
        assert_eq!((world.refs(2), world.refs(3)), (1, 1));

        // Slot 38 over the two cells: the duplicate check before the filter.
        let mut l: Vec<u64> = Vec::new();
        let (a, b) = ([64, 64], [65, 64]);
        let ctx = &mut l as *mut Vec<u64> as *mut c_void;
        assert!(fc4_occmgr_occupants_by_cells(
            m,
            a.as_ptr(),
            b.as_ptr(),
            std::ptr::null(),
            -1,
            Some(push),
            ctx
        ));
        assert_eq!(l, vec![3, 2, 1]);
        assert!(!fc4_occmgr_occupants_by_cells(
            m,
            a.as_ptr(),
            b.as_ptr(),
            std::ptr::null(),
            0,
            Some(push),
            ctx
        ));
        assert!(fc4_occmgr_occupants_by_cells_typed(
            m,
            a.as_ptr(),
            b.as_ptr(),
            0x10,
            1,
            Some(push),
            ctx
        ));
        assert_eq!(l, vec![3, 2, 1, 3]);
        // Slot 36: strict overlap on x, y and z.
        let (qmin, qmax) = ([32.0f32, 0.0, 0.0], [40.0f32, 1.0, 1.0]);
        assert!(fc4_occmgr_occupants_by_bbox(
            m,
            qmin.as_ptr(),
            qmax.as_ptr(),
            &host_filter,
            -1,
            Some(push),
            ctx
        ));
        assert_eq!(l, vec![3, 2, 1, 3, 2]);
        fc4_occmgr_release_list(m, l.as_ptr(), l.len());
        assert_eq!((world.refs(1), world.refs(2), world.refs(3)), (1, 1, 1));

        // Slot 22 with the host filter, slot 23 with a type.
        assert!(fc4_occmgr_remove_occupants(m, &host_filter, false, 4));
        assert_eq!(
            world.names(),
            vec!["clear_flag 2 1".to_string(), format!("post 2 {REMOVE:08X} 4")]
        );
        assert!(fc4_occmgr_remove_occupants_of_type(m, 0x10, true, 5));
        assert_eq!(
            world.names(),
            vec![
                "clear_flag 3 1".to_string(),
                "clear_flag 1 1".to_string(),
                format!("post 3 {REMOVE:08X} 5"),
                "shutdown 3".to_string(),
                format!("post 1 {REMOVE:08X} 5"),
                "shutdown 1".to_string(),
            ]
        );
        assert_eq!((world.refs(1), world.refs(2), world.refs(3)), (0, 0, 0));
        assert!(fc4_occmgr_is_cell_empty(m, 64, 64));
    }
    destroy(&world, m);
}

struct Walk<'a> {
    world: &'a World,
    seen: Vec<(u64, u32)>,
    insert: u64,
}

unsafe extern "C" fn walk(ctx: *mut c_void, o: u64) -> bool {
    let c = &mut *(ctx as *mut Walk);
    c.seen.push((o, c.world.depth()));
    if c.insert != 0 {
        let m = c.world.m.load(Ordering::SeqCst);
        assert!(fc4_occmgr_insert(m, std::mem::take(&mut c.insert), 0));
    }
    true
}

#[test]
fn iteration_lock_regions_and_reentry() {
    let world = World::default();
    world.add(1, 0x10, (0.0, 32.0), (0.0, 32.0));
    world.add(2, 0x10, (0.0, 32.0), (64.0, 96.0));
    world.add(9, 0x10, (64.0, 96.0), (64.0, 96.0));
    let m = make(&world);
    // SAFETY: live manager.
    unsafe {
        assert!(fc4_occmgr_insert(m, 1, 0));
        assert!(fc4_occmgr_insert(m, 2, 0));
        world.take();
        let mut c = Walk {
            world: &world,
            seen: Vec::new(),
            insert: 0,
        };
        let ctx = &mut c as *mut Walk as *mut c_void;
        // Slot 42: callbacks run without the lock.
        let (qmin, qmax) = ([0.0f32, -1.0, 0.0], [200.0f32, 20.0, 200.0]);
        assert!(fc4_occmgr_iterate_by_bbox(
            m,
            Some(walk),
            ctx,
            qmin.as_ptr(),
            qmax.as_ptr(),
            std::ptr::null()
        ));
        assert_eq!(c.seen, vec![(1, 0), (2, 0)]);
        c.seen.clear();
        // Slot 44: the whole walk holds it (the per-cell hold is dropped
        // before the callbacks, OM7), and a callback may insert (re-entrant
        // lock).
        c.insert = 9;
        assert!(fc4_occmgr_iterate(
            m,
            Some(walk),
            ctx,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null()
        ));
        assert_eq!(c.seen[0], (1, 1));
        assert!(c.seen.iter().all(|&(_, d)| d == 1));
        assert_eq!(cell(m, 65, 65), Some(vec![(9, true)]));
        assert_eq!(fc4_occmgr_lock_depth(m), 0);
        assert!(!fc4_occmgr_iterate(
            m,
            None,
            ctx,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null()
        ));
        c.seen.clear();
        assert!(fc4_occmgr_iterate_by_standard_cells_typed(
            m,
            Some(walk),
            ctx,
            std::ptr::null(),
            std::ptr::null(),
            0x10
        ));
        assert_eq!(c.seen.iter().map(|s| s.0).collect::<Vec<_>>(), vec![1, 2, 9]);
    }
    destroy(&world, m);
}

#[test]
fn save_round_trip() {
    let world = World::default();
    world.add(1, 0x10, (0.0, 128.0), (0.0, 32.0));
    world.add(2, 0x10, (0.0, 32.0), (0.0, 32.0));
    let m = make(&world);
    let mut o = Out::default();
    // SAFETY: live managers, valid streams.
    unsafe {
        assert!(fc4_occmgr_insert(m, 1, 0));
        assert!(fc4_occmgr_insert(m, 2, 0));
        let os = ostream(&mut o);
        assert!(fc4_occmgr_write(m, &os));
        assert_eq!(&o.bytes[..2], &1u16.to_le_bytes());
        assert_eq!(o.refs.iter().map(|r| r.0).collect::<Vec<_>>(), vec![2, 1, 1]);

        let world2 = World::default();
        world2.add(1, 0x10, (0.0, 128.0), (0.0, 32.0));
        world2.add(2, 0x10, (0.0, 32.0), (0.0, 32.0));
        // The occupants keep their manager boxes (their own records).
        for id in [1, 2] {
            let om = world.st().occs[&id].om;
            world2.st().occs.get_mut(&id).unwrap().om = om;
        }
        let h = host(&world2);
        let m2 = fc4_occmgr_create(&h);
        world2.m.store(m2, Ordering::SeqCst);
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_occmgr_read(m2, &is));
        assert_eq!(
            world2.st().resolved,
            vec![(2, IID_OCCUPANT), (1, IID_OCCUPANT), (1, IID_OCCUPANT)]
        );
        assert_eq!(cell(m2, 64, 64), cell(m, 64, 64));
        assert_eq!(cell(m2, 65, 64), cell(m, 65, 64));
        let mut st = fc4_occmgr_state::default();
        assert!(fc4_occmgr_get_state(m2, &mut st));
        assert_eq!((st.was_read, st.initialized, st.counts), (1, 0, [16, 16, 64, 64]));

        // A version other than 1 fails and changes nothing but was-read.
        let mut bad = o.bytes.clone();
        bad[0] = 2;
        let m3 = fc4_occmgr_create(&h);
        let mut i = In::new(&bad);
        let is = istream(&mut i);
        assert!(!fc4_occmgr_read(m3, &is));
        assert!(fc4_occmgr_get_state(m3, &mut st));
        assert_eq!((st.was_read, st.rows, st.counts), (1, 0, [0; 4]));
        fc4_occmgr_destroy(m3);
        destroy(&world2, m2);
    }
    destroy(&world, m);
}

/// A manager pointer that may cross threads (the manager is thread-safe).
#[derive(Clone, Copy)]
struct Shared(*mut fc4_occmgr);
// SAFETY: fc4_occmgr serialises itself (module docs).
unsafe impl Send for Shared {}

struct Gate {
    inside: Mutex<Option<mpsc::Sender<()>>>,
    go: Mutex<Option<mpsc::Receiver<()>>>,
}

/// Signals "inside the callback", then waits (bounded) for "go".
unsafe extern "C" fn hold(ctx: *mut c_void, _o: u64) -> bool {
    let g = &*(ctx as *const Gate);
    if let Some(tx) = g.inside.lock().unwrap().take() {
        tx.send(()).unwrap();
        if let Some(rx) = g.go.lock().unwrap().take() {
            let _ = rx.recv_timeout(Duration::from_secs(10));
        }
    }
    true
}

/// Thread A sits in a callback of `slot` (42 or 44) while thread B calls
/// `IsCellEmpty`. Returns whether B finished while A was still inside.
fn other_thread_runs_during(slot: u32) -> (bool, u32) {
    let world = Arc::new(World::default());
    world.add(1, 0x10, (0.0, 32.0), (0.0, 32.0));
    let m = Shared(make(&world));
    // SAFETY: live manager.
    assert!(unsafe { fc4_occmgr_insert(m.0, 1, 0) });
    let (in_tx, in_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let gate = Arc::new(Gate {
        inside: Mutex::new(Some(in_tx)),
        go: Mutex::new(Some(go_rx)),
    });
    let g2 = gate.clone();
    let a = std::thread::spawn(move || {
        let m = m;
        let ctx = Arc::as_ptr(&g2) as *mut c_void;
        let (qmin, qmax) = ([0.0f32, -1.0, 0.0], [100.0f32, 20.0, 100.0]);
        // SAFETY: live manager; the gate outlives the call.
        unsafe {
            if slot == 42 {
                fc4_occmgr_iterate_by_bbox(m.0, Some(hold), ctx, qmin.as_ptr(), qmax.as_ptr(), std::ptr::null())
            } else {
                fc4_occmgr_iterate(
                    m.0,
                    Some(hold),
                    ctx,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            }
        }
    });
    in_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("thread A reached the callback");
    let (done_tx, done_rx) = mpsc::channel();
    let (started_tx, started_rx) = mpsc::channel();
    let b = std::thread::spawn(move || {
        let m = m;
        started_tx.send(()).unwrap();
        // SAFETY: live manager.
        let empty = unsafe { fc4_occmgr_is_cell_empty(m.0, 64, 64) };
        done_tx.send(empty).unwrap();
    });
    // Measure contention after B starts, not while the OS is scheduling it.
    started_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("thread B started its query");
    let during = match done_rx.recv_timeout(Duration::from_millis(300)) {
        Ok(empty) => {
            assert!(!empty);
            true
        }
        Err(_) => false,
    };
    go_tx.send(()).unwrap();
    assert!(a.join().unwrap());
    b.join().unwrap();
    let mut st = fc4_occmgr_state::default();
    // SAFETY: live manager.
    assert!(unsafe { fc4_occmgr_get_state(m.0, &mut st) });
    destroy(&world, m.0);
    (during, st.lock_waits)
}

#[test]
fn slot42_callbacks_run_unlocked_across_threads() {
    let (during, waits) = other_thread_runs_during(42);
    assert!(during, "slot 42 must release the lock around its callbacks");
    assert_eq!(waits, 0);
}

#[test]
fn slot44_holds_the_lock_across_threads() {
    let (during, waits) = other_thread_runs_during(44);
    assert!(!during, "slot 44 holds the lock for the whole walk");
    assert_eq!(waits, 1);
}
