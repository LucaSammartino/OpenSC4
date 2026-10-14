// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests of the lot developer's C ABI (`lotdev.rs`): the interface table,
//! the save record through host streams, re-entrancy (queued messages,
//! refused calls, terrain slots answered mid-call) and the host table.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::hostgrid::fc4_cell_rect;
use crate::lotdev::*;
use crate::test_streams::{self, In, Out};

/// The stub world behind the host table.
#[derive(Default)]
struct World {
    handle: Cell<usize>,
    /// Lot token → (x, z, state).
    lots: RefCell<BTreeMap<u64, (i32, i32, u32)>>,
    /// Occupants that are buildings.
    buildings: RefCell<Vec<u64>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    next_occ: Cell<u64>,
    /// What `add_new_building_occupant` does re-entrantly.
    reenter: Cell<bool>,
    reentrant_slope: Cell<f32>,
    reentrant_abandon: Cell<Option<bool>>,
    construction_occupants: Cell<u32>,
}

fn w(u: *mut c_void) -> &'static World {
    // SAFETY: the tests pass a live World as `user`.
    unsafe { &*(u as *const World) }
}

unsafe extern "C" fn yes(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn terrain_counts(_: *mut c_void, out: *mut i32) {
    *out = 64;
    *out.add(1) = 64;
}
unsafe extern "C" fn cell_width(_: *mut c_void) -> f32 {
    16.0
}
/// Altitude = vertex index (a ramp), so slopes are non-zero.
unsafe extern "C" fn altitude(_: *mut c_void, index: i32) -> f32 {
    index as f32 * 0.5
}
unsafe extern "C" fn lot_rect(u: *mut c_void, lot: u64, out: *mut fc4_cell_rect) {
    if let Some(&(x, z, _)) = w(u).lots.borrow().get(&lot) {
        *out = fc4_cell_rect {
            x1: x,
            z1: z,
            x2: x + 1,
            z2: z + 1,
        };
    }
}
unsafe extern "C" fn lot_state_cb(u: *mut c_void, lot: u64) -> u32 {
    w(u).lots.borrow().get(&lot).map_or(0, |l| l.2)
}
unsafe extern "C" fn lot_set_state_cb(u: *mut c_void, lot: u64, state: u32, _w: u32, _r: u32) -> bool {
    if let Some(l) = w(u).lots.borrow_mut().get_mut(&lot) {
        l.2 = state;
    }
    true
}
unsafe extern "C" fn occupant_as_building(u: *mut c_void, occ: u64) -> u64 {
    if w(u).buildings.borrow().contains(&occ) {
        occ
    } else {
        0
    }
}
unsafe extern "C" fn add_ref(u: *mut c_void, b: u64) {
    *w(u).refs.borrow_mut().entry(b).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, b: u64) {
    *w(u).refs.borrow_mut().entry(b).or_insert(0) -= 1;
}
unsafe extern "C" fn add_building(u: *mut c_void, _t: u32, _pos: *const f32, _o: i32) -> u64 {
    let world = w(u);
    let occ = world.next_occ.get();
    world.next_occ.set(occ + 1);
    world.buildings.borrow_mut().push(occ);
    if world.reenter.get() {
        let p = world.handle.get() as *mut fc4_lotdev;
        // The occupant manager announces the insert synchronously: queued.
        let m = fc4_lotdev_message {
            msg_type: 0x99EF_1142,
            data3: 0x2781_28A0,
            occupant: occ,
            ..Default::default()
        };
        assert!(fc4_lotdev_do_message(p, &m));
        // Another subsystem asks for a slope: answered from the snapshot.
        world.reentrant_slope.set(fc4_lotdev_lot_slope(p, 1, false));
        // Any other slot is refused while the core is busy.
        world.reentrant_abandon.set(Some(fc4_lotdev_abandon_building(p, 1, 2)));
    }
    occ
}
unsafe extern "C" fn construction_occupant(u: *mut c_void, _lot: u64) {
    let world = w(u);
    world.construction_occupants.set(world.construction_occupants.get() + 1);
}
unsafe extern "C" fn no_tokens(_: *mut c_void, _: u64, _: u32, _: fc4_token_callback, _: *mut c_void) {}

fn table(world: &World) -> fc4_lotdev_host {
    // SAFETY: all-zero is a valid table (no callbacks).
    let mut h: fc4_lotdev_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_lotdev_host>() as u32;
    h.user = world as *const World as *mut c_void;
    h.acquire_services = Some(yes);
    h.has_simulator = Some(yes);
    h.acquire_message_server = Some(yes);
    h.terrain_cell_counts = Some(terrain_counts);
    h.city_cell_counts = Some(terrain_counts);
    h.terrain_cell_width = Some(cell_width);
    h.terrain_altitude = Some(altitude);
    h.lot_bounding_rect = Some(lot_rect);
    h.lot_state = Some(lot_state_cb);
    h.lot_set_state = Some(lot_set_state_cb);
    h.occupant_as_building = Some(occupant_as_building);
    h.building_add_ref = Some(add_ref);
    h.building_release = Some(release);
    h.add_new_building_occupant = Some(add_building);
    h.start_construction_occupant = Some(construction_occupant);
    h.lots_surrounding_lot = Some(no_tokens);
    h
}

struct Dev {
    p: *mut fc4_lotdev,
    s: *mut fc4_lotdev_statics,
}

impl Drop for Dev {
    fn drop(&mut self) {
        // SAFETY: both were created by the test.
        unsafe {
            fc4_lotdev_destroy(self.p);
            fc4_lotdev_statics_destroy(self.s);
        }
    }
}

fn create(world: &World) -> Dev {
    let s = fc4_lotdev_statics_create(1);
    let h = table(world);
    // SAFETY: valid table and statics.
    let p = unsafe { fc4_lotdev_create(&h, s) };
    assert!(!p.is_null());
    world.handle.set(p as usize);
    Dev { p, s }
}

fn state(p: *const fc4_lotdev) -> fc4_lotdev_state {
    let mut st = fc4_lotdev_state {
        struct_size: std::mem::size_of::<fc4_lotdev_state>() as u32,
        ..Default::default()
    };
    // SAFETY: live handle.
    assert!(unsafe { fc4_lotdev_get_state(p, &mut st) });
    st
}

#[test]
fn query_interface_table() {
    // §1.1.
    assert_eq!(fc4_lotdev_query_interface(0x0000_0001), 1);
    assert_eq!(fc4_lotdev_query_interface(0x4818_E5D2), 1);
    assert_eq!(fc4_lotdev_query_interface(0x4522_94AA), 2);
    assert_eq!(fc4_lotdev_query_interface(0xE4FD_A3D4), 3);
    assert_eq!(fc4_lotdev_query_interface(0x0000_0002), 0);
    assert_eq!(fc4_lotdev_query_interface(0xA990_BFE0), 0);
}

#[test]
fn create_rejects_bad_arguments() {
    let world = World::default();
    let s = fc4_lotdev_statics_create(1);
    let mut h = table(&world);
    // SAFETY: the pointers are valid or NULL as documented.
    unsafe {
        assert!(fc4_lotdev_create(&h, std::ptr::null()).is_null());
        assert!(fc4_lotdev_create(std::ptr::null(), s).is_null());
        h.struct_size -= 4;
        assert!(fc4_lotdev_create(&h, s).is_null());
        fc4_lotdev_statics_destroy(s);
        // NULL handles answer the fallbacks.
        assert!(!fc4_lotdev_init(std::ptr::null_mut()));
        assert!(fc4_lotdev_do_message(std::ptr::null_mut(), std::ptr::null()));
        assert_eq!(fc4_lotdev_lot_slope(std::ptr::null(), 1, false), 0.0);
    }
}

#[test]
fn save_record_matches_the_test_vector_and_round_trips() {
    // §13: a 64 × 64 city, cell width 16, no buildings → 24 bytes.
    let world = World::default();
    let d = create(&world);
    // SAFETY: live handle; streams outlive the calls.
    unsafe {
        assert!(fc4_lotdev_init(d.p));
        let mut out = Out::default();
        let os = test_streams::ostream(&mut out);
        assert!(fc4_lotdev_write(d.p, &os));
        assert_eq!(
            out.bytes,
            [
                0x01, 0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x44, 0x00, 0x00, 0x80, 0x44, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]
        );

        // Two buildings join the night deque (one reference each).
        world.buildings.borrow_mut().extend([0x100, 0x101]);
        for occ in [0x100u64, 0x101] {
            let m = fc4_lotdev_message {
                msg_type: 0x99EF_1142,
                data3: 0x2781_28A0,
                occupant: occ,
                ..Default::default()
            };
            assert!(fc4_lotdev_do_message(d.p, &m));
        }
        assert_eq!(state(d.p).night_len, 2);
        assert_eq!(world.refs.borrow().get(&0x100), Some(&1));
        let mut out = Out::default();
        let os = test_streams::ostream(&mut out);
        assert!(fc4_lotdev_write(d.p, &os));
        assert_eq!(out.refs, vec![(0x100, 0), (0x101, 0)]);

        // Read back into a fresh lot developer: the refs arrive with the
        // reference the deque keeps (the host stream's, not an AddRef).
        let world2 = World::default();
        let d2 = create(&world2);
        let mut inp = In::new(&out.bytes);
        let is = test_streams::istream(&mut inp);
        assert!(fc4_lotdev_read(d2.p, &is));
        let st = state(d2.p);
        assert_eq!(st.night_len, 2);
        assert_eq!(st.verts_x, 65);
        assert_eq!(st.extent_x, 1024.0);
        assert!(world2.refs.borrow().is_empty());
        // Destroying a loaded, never-initialised object releases them.
        drop(d2);
        assert_eq!(world2.refs.borrow().get(&0x100), Some(&-1));
        assert_eq!(world2.refs.borrow().get(&0x101), Some(&-1));
    }
}

#[test]
fn reentrant_calls_are_queued_refused_or_answered() {
    let world = World::default();
    world.next_occ.set(0x500);
    world.lots.borrow_mut().insert(1, (4, 6, 1));
    let d = create(&world);
    // SAFETY: live handle.
    unsafe {
        assert!(fc4_lotdev_init(d.p));
        world.reenter.set(true);
        // StartConstruction places a building; the occupant manager's insert
        // message arrives during the call.
        assert!(fc4_lotdev_start_construction(d.p, 1, 0, 0x1234, true));
    }
    let st = state(d.p);
    assert_eq!(st.queued_messages, 1);
    assert_eq!(st.night_len, 1, "the queued insert ran after the outermost call");
    assert_eq!(st.refused_calls, 1);
    assert_eq!(world.reentrant_abandon.get(), Some(false));
    assert!(world.reentrant_slope.get() > 0.0, "the slope came from the snapshot");
    assert_eq!(world.construction_occupants.get(), 1);
    assert_eq!(st.constructions_started, 1);
    // animate: the lot stays in state 2 (§6.1).
    assert_eq!(world.lots.borrow()[&1].2, 2);
}

#[test]
fn terrain_slots_match_the_core_formulas() {
    let world = World::default();
    world.lots.borrow_mut().insert(1, (2, 3, 1));
    let d = create(&world);
    // SAFETY: live handle; the rectangles outlive the calls.
    unsafe {
        assert!(fc4_lotdev_init(d.p));
        // Vertex (x, z) has index x + 65 z, altitude index / 2.
        assert_eq!(
            fc4_lotdev_terrain_altitude_by_vertex(d.p, 2, 3, false),
            (2 + 3 * 65) as f32 * 0.5
        );
        let r = fc4_cell_rect {
            x1: 2,
            z1: 3,
            x2: 3,
            z2: 4,
        };
        let lo = fc4_lotdev_area_altitude(d.p, &r as *const fc4_cell_rect as *const c_void, false, true, false);
        let hi = fc4_lotdev_area_altitude(d.p, &r as *const fc4_cell_rect as *const c_void, false, false, false);
        assert_eq!(lo, (2 + 3 * 65) as f32 * 0.5);
        assert_eq!(hi, (4 + 5 * 65) as f32 * 0.5);
        assert_eq!(fc4_lotdev_area_altitude_range(d.p, &r, false), hi - lo);
        assert_eq!(fc4_lotdev_lot_altitude_range(d.p, 1, false), hi - lo);
        // The float overload truncates its rectangle (LD-Q2).
        let f = [2.9f32, 3.2, 3.99, 4.5];
        let lo_f = fc4_lotdev_area_altitude(d.p, f.as_ptr() as *const c_void, true, true, false);
        assert_eq!(lo_f, lo);
        assert_eq!(
            fc4_lotdev_lot_slope(d.p, 1, false).to_bits(),
            fc4_lotdev_area_slope(d.p, &r, false).to_bits()
        );
        // Code-default tuning (no exemplar): an empty cost curve costs 0.
        assert_eq!(fc4_lotdev_lot_development_cost(d.p, &r), 0);
        assert!(!fc4_lotdev_is_area_flat(d.p, &r, false));
        assert!(!fc4_lotdev_is_lot_flat(d.p, 1, false));
    }
}
