// SPDX-License-Identifier: GPL-3.0-or-later
//! The MySim dispatch front's C ABI (mysims.md §4.6.1, §4.7.1, §6.2.1, §7.2.1, §7.2.3).

use std::cell::RefCell;
use std::ffi::c_void;

use crate::advisors::{fc4_mysim_dispatch, fc4_mysim_job_destination, FC4_MYSIM_JOB_LOT};
use crate::mysim_dispatch::*;
use crate::test_streams::{istream, ostream, In, Out};

const TRIP_STATE_A: u32 = 0x4BB2_D020;
const TRIP_STATE_B: u32 = 0x4BB2_CFC0;
const JOB_FOUND: u32 = 0xEBA0_6BF4;
const ARRIVAL: u32 = 0xEBA0_6C12;

/// A 16 × 16 city: the home building 0x100 on lot (2,2)-(3,3), the job
/// building 0x200 on lot (8,8)-(9,9).
#[derive(Default)]
struct World {
    owner: bool,
    notes: RefCell<Vec<(u32, u32, u32)>>,
}

fn world(u: *mut c_void) -> &'static World {
    unsafe { &*(u as *const World) }
}

fn building_rect(b: u64) -> Option<[i32; 4]> {
    match b {
        0x100 => Some([2, 2, 3, 3]),
        0x200 => Some([8, 8, 9, 9]),
        _ => None,
    }
}

unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 16;
    *z = 16;
}
unsafe extern "C" fn occupant_lot_rect(_: *mut c_void, o: u64, r: *mut i32) -> bool {
    match building_rect(o) {
        Some(v) => {
            std::ptr::copy_nonoverlapping(v.as_ptr(), r, 4);
            true
        }
        None => false,
    }
}
unsafe extern "C" fn lot_rect_at(_: *mut c_void, x: i32, z: i32, r: *mut i32) -> bool {
    for b in [0x100, 0x200] {
        let v = building_rect(b).unwrap();
        if (v[0]..=v[2]).contains(&x) && (v[1]..=v[3]).contains(&z) {
            std::ptr::copy_nonoverlapping(v.as_ptr(), r, 4);
            return true;
        }
    }
    false
}
unsafe extern "C" fn lot_at_cell(_: *mut c_void, x: i32, z: i32, b: *mut u64) -> u32 {
    for t in [0x100, 0x200] {
        let v = building_rect(t).unwrap();
        if (v[0]..=v[2]).contains(&x) && (v[1]..=v[3]).contains(&z) {
            *b = t;
            return 2;
        }
    }
    0
}
unsafe extern "C" fn occupant_position(_: *mut c_void, o: u64, x: *mut f32, z: *mut f32) -> bool {
    let Some(v) = building_rect(o) else { return false };
    *x = (v[0] * 16 + 8) as f32;
    *z = (v[1] * 16 + 8) as f32;
    true
}
unsafe extern "C" fn wealth(_: *mut c_void) -> u32 {
    2
}
unsafe extern "C" fn has_owner(u: *mut c_void) -> bool {
    world(u).owner
}
unsafe extern "C" fn notify(u: *mut c_void, sub: u32, a: u32, b: u32) {
    world(u).notes.borrow_mut().push((sub, a, b));
}

fn host(w: &World) -> fc4_mysim_dispatch_host {
    fc4_mysim_dispatch_host {
        struct_size: std::mem::size_of::<fc4_mysim_dispatch_host>() as u32,
        user: w as *const World as *mut c_void,
        cell_counts: Some(cell_counts),
        occupant_lot_rect: Some(occupant_lot_rect),
        lot_rect_at: Some(lot_rect_at),
        lot_at_cell: Some(lot_at_cell),
        occupant_position: Some(occupant_position),
        wealth: Some(wealth),
        city_across_connection: None,
        region_city: None,
        connections_of_type_to_city: None,
        has_owner: Some(has_owner),
        notify: Some(notify),
    }
}

struct Fixture {
    w: Box<World>,
    p: *mut fc4_mysim_dispatch_obj,
}

impl Fixture {
    fn new(owner: bool) -> Fixture {
        let w = Box::new(World {
            owner,
            ..World::default()
        });
        let h = host(&w);
        let p = unsafe { fc4_mysim_dispatch_create(&h) };
        assert!(!p.is_null());
        Fixture { w, p }
    }
    fn state(&self) -> fc4_mysim_dispatch_state {
        let mut s = fc4_mysim_dispatch_state::default();
        assert!(unsafe { fc4_mysim_dispatch_get_state(self.p, &mut s) });
        s
    }
    fn take(&self) -> Vec<(u32, u32, u32)> {
        std::mem::take(&mut self.w.notes.borrow_mut())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe { fc4_mysim_dispatch_destroy(self.p) };
    }
}

#[test]
fn null_and_short_hosts_are_refused_and_null_handles_are_inert() {
    unsafe {
        assert!(fc4_mysim_dispatch_create(std::ptr::null()).is_null());
        let w = World::default();
        let mut h = host(&w);
        h.struct_size = 8;
        assert!(fc4_mysim_dispatch_create(&h).is_null());
        let null = std::ptr::null_mut();
        assert!(!fc4_mysim_dispatch_init(null));
        assert!(!fc4_mysim_dispatch_find_new_job(null));
        assert_eq!(fc4_mysim_dispatch_commute(null, true, 2, 8), FC4_MYSIM_TRIP_REJECTED);
        let mut d = fc4_mysim_dispatch::default();
        assert!(!fc4_mysim_dispatch_snapshot(null, &mut d));
        fc4_mysim_dispatch_minute(null);
        fc4_mysim_dispatch_destroy(null);
    }
}

#[test]
fn constructor_state_and_minute_clock_follow_the_pathfinder() {
    let f = Fixture::new(true);
    let s = f.state();
    assert_eq!((s.state, s.prev_state, s.flags, s.travel_type, s.minutes), (-1, -1, 0, 0xFF, 0));
    assert_eq!((s.change_counter, s.queue_len, s.has_pathfinder), (0, 0, 0));
    unsafe {
        // §4.6.1: no pathfinder, no clock.
        fc4_mysim_dispatch_minute(f.p);
        assert_eq!(f.state().minutes, 0);
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_minute(f.p);
        fc4_mysim_dispatch_minute(f.p);
    }
    assert_eq!((f.state().minutes, f.state().has_pathfinder), (10, 1));
    assert!(f.take().is_empty());
}

#[test]
fn home_then_animation_places_the_sim_at_home_with_ordered_notices() {
    let f = Fixture::new(true);
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        assert_eq!(f.take(), vec![(TRIP_STATE_A, 0, 0x80)]);
        assert!(fc4_mysim_dispatch_check_flags(f.p, 0x80));
        assert_eq!(fc4_mysim_dispatch_animation(f.p), FC4_MYSIM_TRIP_ACCEPTED);
    }
    // −1 with home: teleport home (arrival at home), then state 3 (flag 2 first).
    assert_eq!(
        f.take(),
        vec![
            (ARRIVAL, 1, 0),
            (TRIP_STATE_A, 0x80, 0x82),
            (TRIP_STATE_B, u32::MAX, 3)
        ]
    );
    let mut d = fc4_mysim_dispatch::default();
    assert!(unsafe { fc4_mysim_dispatch_snapshot(f.p, &mut d) });
    assert_eq!((d.state, d.travel_type, d.at_home, d.location), (3, 0xFE, 1, 0x100));
    assert_eq!((d.cell_x, d.cell_z), (2, 2));
    assert_eq!(f.state().home, 0x100);
}

#[test]
fn no_owner_receives_nothing_but_state_still_changes() {
    let f = Fixture::new(false);
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        fc4_mysim_dispatch_animation(f.p);
    }
    assert!(f.take().is_empty());
    assert_eq!(f.state().state, 3);
}

#[test]
fn change_job_stores_the_lot_and_reports_it() {
    let f = Fixture::new(true);
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        f.take();
        assert!(fc4_mysim_dispatch_change_job(f.p, 0x200));
        assert_eq!(f.take(), vec![(JOB_FOUND, 0, 0)]);
        let mut j = fc4_mysim_job_destination::default();
        assert!(fc4_mysim_dispatch_job_destination(f.p, &mut j));
        assert_eq!((j.kind, j.building, j.cell_x, j.cell_z), (FC4_MYSIM_JOB_LOT, 0x200, 8, 8));
    }
    let s = f.state();
    assert_eq!((s.commute, s.commute_by_commute), (0x200, 1));
}

#[test]
fn job_search_on_the_empty_network_fails_without_a_job() {
    let f = Fixture::new(true);
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        f.take();
        assert!(!fc4_mysim_dispatch_find_new_job(f.p));
        let mut j = fc4_mysim_job_destination::default();
        assert!(!fc4_mysim_dispatch_job_destination(f.p, &mut j));
    }
    assert!(f.take().iter().all(|n| n.0 != JOB_FOUND));
}

#[test]
fn commute_requests_queue_without_arrival() {
    let f = Fixture::new(true);
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        assert!(fc4_mysim_dispatch_change_job(f.p, 0x200));
        f.take();
        assert_eq!(fc4_mysim_dispatch_commute(f.p, false, 2, 8), FC4_MYSIM_TRIP_REJECTED);
        assert_eq!(f.state().queue_len, 0);
        // State 2 (at home, employed): the commute destination is queued.
        assert_eq!(fc4_mysim_dispatch_commute(f.p, true, 2, 8), FC4_MYSIM_TRIP_ACCEPTED);
        assert_eq!(f.state().queue_len, 1);
        // State 3 (at work): replace the queue with home.
        assert_eq!(fc4_mysim_dispatch_commute(f.p, true, 3, 17), FC4_MYSIM_TRIP_ACCEPTED);
        assert_eq!(f.state().queue_len, 1);
    }
    // A request is not an arrival.
    assert!(f.take().iter().all(|n| n.0 != ARRIVAL));
    assert_eq!(f.state().state, -1);
}

#[test]
fn u_drive_is_refused_while_unplaced() {
    let f = Fixture::new(true);
    unsafe {
        assert!(!fc4_mysim_dispatch_u_drive(f.p, 0));
        assert!(!fc4_mysim_dispatch_u_drive(f.p, 1));
        assert!(!fc4_mysim_dispatch_u_drive(f.p, 9));
    }
}

#[test]
fn record_roundtrips_through_the_c_streams_and_replays_on_setup() {
    let f = Fixture::new(true);
    let mut bytes = Out::default();
    unsafe {
        assert!(fc4_mysim_dispatch_init(f.p));
        fc4_mysim_dispatch_set_home(f.p, 0x100);
        fc4_mysim_dispatch_animation(f.p);
        let o = ostream(&mut bytes);
        assert!(fc4_mysim_dispatch_write(f.p, &o));
    }
    assert!(bytes.refs.iter().any(|r| r.0 == 0x100));
    let g = Fixture::new(true);
    unsafe {
        let mut i = In::new(&bytes.bytes);
        let s = istream(&mut i);
        assert!(fc4_mysim_dispatch_read(g.p, &s));
        assert_eq!(i.pos, bytes.bytes.len());
        assert!(fc4_mysim_dispatch_deferred_setup(g.p));
    }
    let (a, b) = (f.state(), g.state());
    assert_eq!((b.state, b.flags, b.travel_type, b.home), (a.state, a.flags, a.travel_type, a.home));
    assert_eq!(g.take(), vec![(TRIP_STATE_A, 0, a.flags), (TRIP_STATE_B, (-2i32) as u32, 3)]);
    let mut again = Out::default();
    unsafe {
        let o = ostream(&mut again);
        assert!(fc4_mysim_dispatch_write(g.p, &o));
    }
    assert_eq!(again.bytes, bytes.bytes);
}

#[test]
fn layouts_match_fc4_h() {
    let ptr = std::mem::size_of::<*const u8>();
    assert_eq!(std::mem::size_of::<fc4_mysim_dispatch_host>(), if ptr == 4 { 52 } else { 104 });
    assert_eq!(std::mem::size_of::<fc4_mysim_dispatch_state>(), 56);
}
