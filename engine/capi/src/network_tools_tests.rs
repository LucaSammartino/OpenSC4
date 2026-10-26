// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The C ABI tests of the network tools (`fc4_nettool_*`), driving the §18
//! tests through the host table.

use super::network_tools::{
    fc4_nettool_cell, fc4_nettool_create, fc4_nettool_destroy, fc4_nettool_destroy_occupants_and_resolve,
    fc4_nettool_get_failure_state, fc4_nettool_get_state, fc4_nettool_host, fc4_nettool_init, fc4_nettool_listed,
    fc4_nettool_neighbour, fc4_nettool_pass, fc4_nettool_place_network_occupant_by_id,
    fc4_nettool_place_network_occupants, fc4_nettool_reset, fc4_nettool_run, fc4_nettool_solved, fc4_nettool_state,
    fc4_nettool_underground, fc4_nettool_update_underground_heights, FC4_NETTOOL_MAX_CELLS, FC4_NETTOOL_U_KEPT,
    FC4_NETTOOL_U_UPDATED,
};

type U = *mut std::ffi::c_void;

/// The fake world the host table drives.
#[derive(Default)]
struct World {
    pieces: std::collections::BTreeMap<(i32, i32), u32>,
    inserted: Vec<(i32, i32, u32, u32)>,
    posts: Vec<(u32, u64)>,
    drawn: u32,
    pass_valid: u32,
    pass_failure: u32,
    draw_calls: u32,
    specific_calls: u32,
    callbacks: Vec<(u32, i32, i32, i32, i32)>,
    neighbours: Vec<fc4_nettool_neighbour>,
    /// The occupants that are themselves intersection pieces, with the
    /// structure each pulls in (§10 step 3).
    structures: std::collections::BTreeMap<u64, Vec<u64>>,
    portals: std::collections::BTreeMap<u64, u64>,
    /// The cell of each occupant the tests name (slot 59).
    cells: std::collections::BTreeMap<u64, (i32, i32)>,
    pre_demolish: Vec<u64>,
    removal_kinds: Vec<(u64, bool)>,
    resolve_cells: Vec<(i32, i32)>,
    terrain: std::collections::BTreeMap<(i32, i32), [f32; 4]>,
    solved: std::collections::BTreeMap<(i32, i32), (u64, [f32; 4])>,
    smooth_ok: bool,
    underground: Vec<(u64, u32, [f32; 4], [f32; 4])>,
    destroyed: Vec<u64>,
}

impl World {
    fn occ_is_intersection(&self, occ: u64) -> bool {
        self.structures.contains_key(&occ)
    }
}

struct Fixture {
    w: *mut World,
    host: Box<fc4_nettool_host>,
}

impl Fixture {
    /// SAFETY: the fixture owns the World; tests use one thread.
    fn w(&self) -> &World {
        unsafe { &*self.w }
    }
    /// SAFETY: as above.
    fn w_mut(&mut self) -> &mut World {
        unsafe { &mut *self.w }
    }
}

unsafe fn world<'a>(u: U) -> &'a mut World {
    &mut *(u as *mut World)
}

impl Fixture {
    fn new() -> Fixture {
        let w = Box::into_raw(Box::new(World::default()));
        let mut h: Box<fc4_nettool_host> = unsafe {
            let layout = std::alloc::Layout::new::<fc4_nettool_host>();
            let ptr = std::alloc::alloc_zeroed(layout) as *mut fc4_nettool_host;
            Box::from_raw(ptr)
        };
        h.struct_size = std::mem::size_of::<fc4_nettool_host>() as u32;
        h.user = w as U;
        h.draw_network_line = Some(trap_draw);
        h.validate_and_solve = Some(trap_pass);
        h.insert_occupants_for_solved_cells = Some(trap_insert);
        h.mark_occupants_usable = Some(trap_usable);
        h.insert_network_occupant_by_id = Some(trap_by_id);
        h.get_network_occupant = Some(trap_get);
        h.callback_place = Some(trap_cb);
        h.neighbours_by_network_type = Some(trap_neighbours);
        h.is_intersection_piece = Some(trap_is_int);
        h.intersection_structure = Some(trap_structure);
        h.is_tunnel_portal = Some(trap_is_portal);
        h.paired_portal = Some(trap_portal);
        h.send_pre_demolish_sync = Some(trap_pre);
        h.remove_occupant = Some(trap_remove);
        h.remove_or_replace_occupant = Some(trap_ror);
        h.solve_network_resolve = Some(trap_solve);
        h.corner_altitudes = Some(trap_corners);
        h.smoothen_underground = Some(trap_smooth);
        h.find_underground_solved = Some(trap_solved);
        h.destroy_network_occupants = Some(trap_destroy);
        h.underground_rebuild = Some(trap_rebuild);
        h.occupied_cell = Some(trap_cell);
        Fixture { w, host: h }
    }

    fn tool(&self, type_id: u32) -> *mut super::network_tools::fc4_nettool {
        unsafe { fc4_nettool_create(&*self.host, type_id) }
    }
}

unsafe extern "C" fn trap_draw(u: U, run: *mut fc4_nettool_run, _snap: bool) -> bool {
    let w = world(u);
    w.draw_calls += 1;
    let n = w.drawn;
    let r = &mut *run;
    r.dragged_count = n;
    r.straight_count = n;
    r.valid_count = n;
    let m = (n as usize).min(FC4_NETTOOL_MAX_CELLS);
    r.cell_count = m as u32;
    for (i, c) in r.cells.iter_mut().take(m).enumerate() {
        *c = fc4_nettool_cell {
            x: r.start_x + i as i32,
            z: r.start_z,
        };
    }
    true
}

unsafe extern "C" fn trap_pass(u: U, run: *mut fc4_nettool_run, _real: bool, out: *mut fc4_nettool_pass) -> bool {
    let w = world(u);
    (*run).valid_count = w.pass_valid;
    (*out).valid_count = w.pass_valid;
    (*out).failure = w.pass_failure;
    true
}

unsafe extern "C" fn trap_insert(u: U, run: *const fc4_nettool_run, _preview_only: bool) {
    let w = world(u);
    let r = &*run;
    let m = (r.cell_count as usize).min(FC4_NETTOOL_MAX_CELLS);
    for c in &r.cells[..m] {
        w.inserted.push((c.x, c.z, r.data4, 0x1000_0000));
        w.posts.push((0x99EF_1142, u64::from(r.data4)));
    }
}

unsafe extern "C" fn trap_usable(u: U, run: *const fc4_nettool_run, _release: bool) {
    let w = world(u);
    let r = &*run;
    let m = (r.cell_count as usize).min(FC4_NETTOOL_MAX_CELLS);
    for _ in &r.cells[..m] {
        w.posts.push((0x89D3_E9F3, 1));
    }
}

unsafe extern "C" fn trap_by_id(u: U, run: *const fc4_nettool_run, x: i32, z: i32, _piece: u32, usable: bool, _t: u32) {
    let w = world(u);
    let flags = if usable { 0x1000_0000 } else { 0x4000 };
    w.inserted.push((x, z, (*run).data4, flags));
    w.posts.push((0x99EF_1142, u64::from((*run).data4)));
}

unsafe extern "C" fn trap_get(u: U, x: i32, z: i32, _mask: u32) -> u32 {
    world(u).pieces.get(&(x, z)).copied().unwrap_or(0xFFFF_FFFF)
}

unsafe extern "C" fn trap_cb(u: U, cb: u32, ax: i32, az: i32, bx: i32, bz: i32, _real: bool) {
    world(u).callbacks.push((cb, ax, az, bx, bz));
}

unsafe extern "C" fn trap_neighbours(u: U, _occ: u64, out: *mut fc4_nettool_neighbour, cap: u32) -> u32 {
    let w = world(u);
    let n = w.neighbours.len().min(cap as usize);
    for (i, nb) in w.neighbours.iter().take(n).enumerate() {
        *out.add(i) = *nb;
    }
    w.neighbours.len() as u32
}

unsafe extern "C" fn trap_is_int(u: U, occ: u64) -> bool {
    world(u).occ_is_intersection(occ)
}

unsafe extern "C" fn trap_structure(u: U, occ: u64, out: *mut u64, cap: u32) -> u32 {
    let w = world(u);
    let v = w.structures.get(&occ).cloned().unwrap_or_default();
    let n = v.len().min(cap as usize);
    for (i, x) in v.iter().take(n).enumerate() {
        *out.add(i) = *x;
    }
    v.len() as u32
}

unsafe extern "C" fn trap_is_portal(u: U, occ: u64) -> bool {
    world(u).portals.contains_key(&occ)
}

unsafe extern "C" fn trap_portal(u: U, occ: u64) -> u64 {
    world(u).portals.get(&occ).copied().unwrap_or(0)
}

unsafe extern "C" fn trap_pre(u: U, occ: u64) {
    let w = world(u);
    w.pre_demolish.push(occ);
    w.posts.push((0xAA7A_014D, occ));
}

unsafe extern "C" fn trap_remove(u: U, occ: u64, _shutdown: bool) {
    world(u).removal_kinds.push((occ, true));
}

unsafe extern "C" fn trap_ror(u: U, occ: u64) -> bool {
    world(u).removal_kinds.push((occ, false));
    true
}

unsafe extern "C" fn trap_solve(u: U, run: *mut fc4_nettool_run) {
    let w = world(u);
    let r = &mut *run;
    let m = (r.resolve_count as usize).min(FC4_NETTOOL_MAX_CELLS);
    w.resolve_cells = r.resolve_cells[..m].iter().map(|c| (c.x, c.z)).collect();
}

unsafe extern "C" fn trap_corners(u: U, x: i32, z: i32, out: *mut f32) {
    let a = world(u).terrain.get(&(x, z)).copied().unwrap_or([0.0; 4]);
    for (i, v) in a.iter().enumerate() {
        out.add(i).write(*v);
    }
}

unsafe extern "C" fn trap_smooth(u: U, _run: *mut fc4_nettool_run, _level: i32) -> bool {
    world(u).smooth_ok
}

unsafe extern "C" fn trap_solved(u: U, x: i32, z: i32, out: *mut fc4_nettool_solved) -> bool {
    let w = world(u);
    match w.solved.get(&(x, z)) {
        Some((occ, c)) => {
            (*out).occ = *occ;
            (*out).corners = *c;
            true
        }
        None => false,
    }
}

unsafe extern "C" fn trap_destroy(u: U, occ: *const u64, count: usize) {
    let w = world(u);
    for i in 0..count {
        w.destroyed.push(*occ.add(i));
    }
}

unsafe extern "C" fn trap_cell(u: U, occ: u64, out: *mut fc4_nettool_cell) -> bool {
    // The fixture's stand-in for network occupant slot 59: the token's
    // recorded cell from the §18 runs (the destroy tests list them in
    // `cells`).
    let w = world(u);
    match w.cells.get(&occ) {
        Some(&(x, z)) => {
            (*out) = fc4_nettool_cell { x, z };
            true
        }
        None => false,
    }
}

unsafe extern "C" fn trap_rebuild(u: U, occ: u64, outcome: u32, vh: *const f32, st: *const f32) {
    let w = world(u);
    let zero = [0.0f32; 4];
    let vh = if vh.is_null() { zero } else { *(vh as *const [f32; 4]) };
    let st = if st.is_null() { zero } else { *(st as *const [f32; 4]) };
    w.underground.push((occ, outcome, vh, st));
}

#[test]
fn create_null_and_bad_size() {
    assert_eq!(unsafe { fc4_nettool_create(std::ptr::null(), 0) }, std::ptr::null_mut());
    let small: u32 = 4;
    assert_eq!(
        unsafe { fc4_nettool_create(&small as *const u32 as *const fc4_nettool_host, 0) },
        std::ptr::null_mut()
    );
}

#[test]
fn t18_01_free_code_placement_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.drawn = 11;
        w.pass_valid = 11;
    }
    let t = f.tool(0);
    assert!(!t.is_null());
    let ok = unsafe { fc4_nettool_place_network_occupants(t, 10, 10, 20, 10, 0, 0, true, -1) };
    assert!(ok);
    let w = f.w();
    assert_eq!(w.inserted.len(), 11);
    assert!(w.inserted.iter().all(|i| i.2 == 3 && i.3 & 0x1000_0000 != 0));
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_04_diagonal_quirk_over_the_abi() {
    let f = Fixture::new();
    let t = f.tool(3);
    let ok = unsafe { fc4_nettool_place_network_occupants(t, 5, 5, 8, 9, 0xC49, 0, true, 3) };
    assert!(ok);
    let w = f.w();
    assert!(w.inserted.is_empty() && w.posts.is_empty() && w.draw_calls == 0);
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_07_join_callback_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.pieces.insert((10, 10), 0);
        w.drawn = 4;
        w.pass_valid = 4;
    }
    let t = f.tool(3);
    let ok = unsafe { fc4_nettool_place_network_occupants(t, 11, 10, 14, 10, 0xC49, 0xDEAD, true, 3) };
    assert!(ok);
    let w = f.w();
    assert_eq!(w.callbacks.len(), 1);
    assert_eq!(w.callbacks[0], (0xDEAD, 11, 10, 10, 10));
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_08_failure_leaves_nothing_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.drawn = 11;
        w.pass_valid = 0;
        w.pass_failure = 0xF000_0002;
    }
    let t = f.tool(0);
    let ok = unsafe { fc4_nettool_place_network_occupants(t, 10, 10, 20, 10, 0, 0, true, -1) };
    assert!(!ok);
    assert_eq!(unsafe { fc4_nettool_get_failure_state(t) }, 1);
    assert!(f.w().inserted.is_empty());
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_19_place_by_id_over_the_abi() {
    let f = Fixture::new();
    let t = f.tool(0);
    unsafe { fc4_nettool_place_network_occupant_by_id(t, 7, 9, 0x100, false, 0) };
    {
        let w = f.w();
        assert_eq!(w.inserted.len(), 1);
        assert_eq!((w.inserted[0].0, w.inserted[0].1), (7, 9));
        assert_eq!(w.inserted[0].2, 1);
        assert_eq!(w.inserted[0].3 & 0x4000, 0x4000);
    }
    unsafe { fc4_nettool_place_network_occupant_by_id(t, 7, 9, 0x100, true, 0) };
    let w = f.w();
    assert_eq!(w.inserted[1].3 & 0x4000, 0);
    assert_ne!(w.inserted[1].3 & 0x1000_0000, 0);
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_13_destroy_and_resolve_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.neighbours = vec![
            fc4_nettool_neighbour {
                occ: 11,
                x: 10,
                z: 10,
                ..Default::default()
            },
            fc4_nettool_neighbour {
                occ: 12,
                x: 12,
                z: 10,
                ..Default::default()
            },
        ];
        w.cells.insert(2, (11, 10)); // the destroyed piece
    }
    let t = f.tool(0);
    let list = [fc4_nettool_listed { occ: 2, x: 11, z: 10 }];
    assert!(unsafe { fc4_nettool_destroy_occupants_and_resolve(t, list.as_ptr(), 1) });
    let w = f.w();
    // The destroyed piece got the synchronous message and the plain removal
    // (its cell is a destroyed cell, §10 step 5.3).
    assert_eq!(w.pre_demolish, vec![2]);
    assert!(w.removal_kinds.iter().any(|&(o, d)| o == 2 && d));
    // The surviving neighbours' cells went into the re-solve set (§13).
    assert!(w.resolve_cells.contains(&(10, 10)));
    assert!(w.resolve_cells.contains(&(12, 10)));
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn destroy_closure_pulls_structures_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        // Token 1 is an intersection piece whose structure holds token 4;
        // token 2 is an immovable non-intersection neighbour (joins the list).
        w.structures.insert(1, vec![4]);
        w.neighbours = vec![fc4_nettool_neighbour {
            occ: 2,
            x: 11,
            z: 10,
            immovable: 1,
            intersection: 0,
            ..Default::default()
        }];
        w.cells.insert(1, (10, 10));
        w.cells.insert(2, (11, 10));
        w.cells.insert(4, (14, 10));
    }
    let t = f.tool(0);
    let list = [fc4_nettool_listed { occ: 1, x: 10, z: 10 }];
    assert!(unsafe { fc4_nettool_destroy_occupants_and_resolve(t, list.as_ptr(), 1) });
    let w = f.w();
    assert!(w.pre_demolish.contains(&2));
    assert!(w.pre_demolish.contains(&4));
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn destroy_tunnel_portal_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.portals.insert(1, 2);
        w.cells.insert(1, (5, 5));
        w.cells.insert(2, (9, 5));
    }
    let t = f.tool(0);
    let list = [fc4_nettool_listed { occ: 1, x: 5, z: 5 }];
    assert!(unsafe { fc4_nettool_destroy_occupants_and_resolve(t, list.as_ptr(), 1) });
    assert!(f.w().pre_demolish.contains(&2));
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_14_underground_refit_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.smooth_ok = true;
        w.terrain.insert((5, 5), [110.0; 4]);
        w.solved.insert((5, 5), (7, [100.0; 4]));
    }
    let t = f.tool(4);
    unsafe { fc4_nettool_init(t, 64, 64, 10.0, 5.0, 0.0, 0.0, false) };
    let piece = fc4_nettool_underground {
        occ: 7,
        x: 5,
        z: 5,
        stored_terrain: [100.0; 4],
        vertices: [90.0; 4],
        edges: 0,
    };
    assert!(unsafe { fc4_nettool_update_underground_heights(t, &piece, 1, 4) });
    let w = f.w();
    assert_eq!(w.underground.len(), 1);
    assert_eq!(w.underground[0].0, 7);
    assert_eq!(w.underground[0].1, FC4_NETTOOL_U_UPDATED);
    assert_eq!(w.underground[0].2, [100.0; 4]);
    assert_eq!(w.underground[0].3, [110.0; 4]);
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn t18_16_underground_fallback_over_the_abi() {
    let mut f = Fixture::new();
    {
        let w = f.w_mut();
        w.smooth_ok = false;
        w.terrain.insert((5, 5), [-15.0; 4]);
    }
    let t = f.tool(4);
    unsafe { fc4_nettool_init(t, 64, 64, 10.0, 5.0, 0.0, 0.0, false) };
    let piece = fc4_nettool_underground {
        occ: 9,
        x: 5,
        z: 5,
        stored_terrain: [-30.0; 4],
        vertices: [-20.0; 4],
        edges: 0,
    };
    assert!(unsafe { fc4_nettool_update_underground_heights(t, &piece, 1, 4) });
    {
        let w = f.w();
        let last = w.underground.last().unwrap();
        assert_eq!(last.0, 9);
        assert_eq!(last.1, FC4_NETTOOL_U_KEPT);
        assert_eq!(last.3, [-15.0; 4]);
    }
    // −16 at one corner destroys (−16 < −20 + 5 for every corner).
    f.w_mut().underground.clear();
    f.w_mut().terrain.insert((5, 5), [-16.0; 4]);
    assert!(unsafe { fc4_nettool_update_underground_heights(t, &piece, 1, 4) });
    let w = f.w();
    assert!(w.destroyed.contains(&9));
    assert!(w.underground.is_empty());
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn state_round_trip_and_reset() {
    let f = Fixture::new();
    let t = f.tool(0);
    let mut s = fc4_nettool_state::default();
    assert!(unsafe { fc4_nettool_get_state(t, &mut s) });
    assert_eq!(s.type_id, 0);
    assert_eq!(s.layer_mask, 0x1F4F);
    assert_eq!(s.data4, 1);
    assert!(s.stop_on_failure != 0 && s.preview_update != 0);
    unsafe { fc4_nettool_reset(t) };
    assert!(unsafe { fc4_nettool_get_state(t, &mut s) } && s.cache_count == 0);
    unsafe { fc4_nettool_destroy(t) };
}

#[test]
fn powerline_draft_override_bypasses_base_solve_over_the_abi() {
    unsafe extern "C" fn specific(u: U, run: *mut fc4_nettool_run, _: u32, mode: u32, real: bool) -> bool {
        let run = &mut *run;
        assert_eq!(run.type_id, 5);
        assert_eq!(mode, 0);
        assert!(!real);
        world(u).specific_calls += 1;
        run.valid_count = if run.dragged_count < 2 { 0 } else { run.dragged_count };
        true
    }

    // network-tools sections 5.1/9: the subclass handles even the one-step
    // draft. Falling through to the base solve would return this refusal.
    for drawn in [1, 5, 13] {
        let mut f = Fixture::new();
        f.w_mut().drawn = drawn;
        f.w_mut().pass_failure = 0xF000_0012;
        f.host.do_network_specific_placement = Some(specific);
        let t = f.tool(5);
        assert!(unsafe { fc4_nettool_init(t, 64, 64, 0.0, 0.0, 0.0, 0.0, false) });
        assert!(unsafe { fc4_nettool_place_network_occupants(t, 2, 2, 2 + drawn as i32 - 1, 2, 0, 0, false, -1) });
        assert_eq!(unsafe { fc4_nettool_get_failure_state(t) }, 0);
        assert_eq!(f.w().specific_calls, 1);
        assert!(f.w().inserted.is_empty());
        unsafe { fc4_nettool_destroy(t) };
    }
}
