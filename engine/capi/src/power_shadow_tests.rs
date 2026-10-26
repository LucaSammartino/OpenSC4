// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power shadow against a fake game whose "original" power simulator is
//! FreeCity4's own (`fc4_power_*`): its record is captured the way the
//! plugin captures the game's (references as `u32 objectID, u32 classID`
//! with the queried interfaces alongside), so a faithful shadow reports no
//! difference, and a cell, a scalar or a block order changed behind its back
//! is reported.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use fc4_core::power::{budget, msg, prop, shipped_tuning, state, GRID_CLSID};
use fc4_core::variant::mem;

use crate::budget::fc4_token_callback;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::power::*;
use crate::power_shadow::*;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::stream::fc4_ref;
use crate::test_streams::{ostream, Out};

const CELLS: i32 = 32;
const PLANT: u64 = 10;
const DEPT: u64 = 0xD0;
const OCCUPANT_BUILDING: u32 = 0x2781_28A0;
/// 2000-02-01.
const FEB_2000: i32 = 2_451_576;

#[test]
fn host_trace_replays_cycles_messages_and_cache_without_game() {
    let dir = crate::trace::test_record("power-roundtrip", || {
        let mut w = World::new(Game::new(1000, &[300, 400]), true);
        assert!(w.cycle().iter().all(|&n| n == 0));
        w.notify(msg::OCCUPANT_REMOVED, 12);
        assert!(w.cycle().iter().all(|&n| n == 0));
        assert_eq!(w.deliver(msg::SIM_NEW_MONTH, false, |_, _| {}), Some(0));
    });
    let manifest = dir.join("manifest.tsv");
    assert!(crate::trace::run(&manifest, true, 2).unwrap());
    assert!(crate::trace::run(&manifest, false, 2).unwrap());
    assert!(std::fs::read_to_string(dir.join("replay-verdicts.tsv"))
        .unwrap()
        .contains("\ttrue\t\n"));
}

/// (element type, array flag, bytes, count).
type Written = (u16, bool, Vec<u8>, usize);

struct Game {
    grids: RefCell<HashMap<u64, Vec<u8>>>,
    next_grid: Cell<u64>,
    /// Occupant → (cells, capacity or requirement).
    occupants: RefCell<Vec<(u64, fc4_cell_rect, u32)>>,
    /// Occupants removed from the occupant manager (still readable, as
    /// while their removal message is delivered).
    gone: RefCell<Vec<u64>>,
    /// Zoned cells (zone type 1).
    zones: RefCell<Vec<(i32, i32)>>,
    written: RefCell<HashMap<(u64, u32), Written>>,
    dept: Cell<u64>,
    posts: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

fn rect(x1: i32, z1: i32, x2: i32, z2: i32) -> fc4_cell_rect {
    fc4_cell_rect { x1, z1, x2, z2 }
}

impl Game {
    /// A plant of `capacity` at (8,0)-(9,1) and consumers of `need` at
    /// x = 12, 18, 24 … (rows 0-1, two cells wide, ids 11, 12 …).
    fn new(capacity: u32, consumers: &[u32]) -> Box<Game> {
        let mut occ = vec![(PLANT, rect(8, 0, 9, 1), capacity)];
        for (i, &need) in consumers.iter().enumerate() {
            let x = 12 + 6 * i as i32;
            occ.push((11 + i as u64, rect(x, 0, x + 1, 1), need));
        }
        Box::new(Game {
            grids: RefCell::new(HashMap::new()),
            next_grid: Cell::new(0),
            occupants: RefCell::new(occ),
            gone: RefCell::new(Vec::new()),
            zones: RefCell::new(Vec::new()),
            written: RefCell::new(HashMap::new()),
            dept: Cell::new(0),
            posts: Cell::new(0),
        })
    }
    fn cells_of(&self, occ: u64) -> fc4_cell_rect {
        self.occupants
            .borrow()
            .iter()
            .find(|o| o.0 == occ)
            .map_or(rect(0, 0, 0, 0), |o| o.1)
    }
    fn amount(&self, occ: u64) -> Option<u32> {
        self.occupants.borrow().iter().find(|o| o.0 == occ).map(|o| o.2)
    }
    fn grid(&self) -> u64 {
        *self
            .grids
            .borrow()
            .keys()
            .next()
            .expect("the original created its grid")
    }
    fn set_cell(&self, x: i32, z: i32, v: u8) {
        let t = self.grid();
        self.grids.borrow_mut().get_mut(&t).unwrap()[(x * CELLS + z) as usize] = v;
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, _instance: u32, _ts: i32) -> u64 {
    assert_eq!(clsid, GRID_CLSID);
    let g = game(u);
    let t = 0x7000 + g.next_grid.get();
    g.next_grid.set(g.next_grid.get() + 0x10);
    g.grids.borrow_mut().insert(t, vec![0; (CELLS * CELLS) as usize]);
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    if game(u).grids.borrow().contains_key(&t) {
        GRID_CLSID
    } else {
        0
    }
}
unsafe extern "C" fn g_geometry(_: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
    *out = fc4_grid_geometry {
        tract_size: 1,
        shift: 0,
        count_x: CELLS,
        count_z: CELLS,
        width_x: 16.0,
        width_z: 16.0,
        inv_width_x: 1.0 / 16.0,
        inv_width_z: 1.0 / 16.0,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32) -> f64 {
    f64::from(game(u).grids.borrow()[&t][(x * CELLS + z) as usize])
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap()[(x * CELLS + z) as usize] = v as u8;
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().fill(v as u8);
}
unsafe extern "C" fn g_noop(_: *mut c_void, _t: u64) {}

// ---- city, occupants, budget ---------------------------------------------------------------------------

unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = CELLS;
    *z = CELLS;
}
unsafe extern "C" fn cell_widths(_: *mut c_void, x: *mut f32, z: *mut f32) {
    *x = 16.0;
    *z = 16.0;
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn post(u: *mut c_void, _p: *const fc4_power_post) {
    game(u).posts.set(game(u).posts.get() + 1);
}
unsafe extern "C" fn resolve_ref(_: *mut c_void, t: u64, _iid: u32) -> u64 {
    t
}
unsafe extern "C" fn occupant_type(u: *mut c_void, occ: u64) -> u32 {
    if game(u).amount(occ).is_some() {
        OCCUPANT_BUILDING
    } else {
        0
    }
}
unsafe fn scalar<T>(p: *mut fc4_props, id: u32, mem_type: u16, v: T) -> bool {
    fc4_props_set_typed(p, id, mem_type, false, &v as *const T as *const c_void, 1)
}
unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    let g = game(u);
    if let Some((t, a, b, n)) = g.written.borrow().get(&(occ, id)) {
        return fc4_props_set_typed(p, id, *t, *a, b.as_ptr() as *const c_void, *n);
    }
    let Some(amount) = g.amount(occ) else { return false };
    match (occ, id) {
        (PLANT, prop::CAPACITY) => scalar(p, id, mem::UINT32, amount),
        (PLANT, prop::PLANT_TYPE) => scalar(p, id, mem::SINT32, 1i32),
        (PLANT, prop::CONDITION_CURVE) => {
            let c = [0.0f32, 1.0, 1.0, 1.0];
            fc4_props_set_typed(p, id, mem::FLOAT32, true, c.as_ptr() as *const c_void, c.len())
        }
        (PLANT, _) => false,
        (_, prop::REQUIREMENT) => scalar(p, id, mem::UINT32, amount),
        _ => false,
    }
}
unsafe extern "C" fn set_occupant_property(
    u: *mut c_void,
    occ: u64,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const c_void,
    count: usize,
) {
    let size = match mem_type {
        mem::BOOL | mem::UINT8 | mem::SINT8 | mem::CHAR | mem::RZ_CHAR => 1,
        mem::UINT16 | mem::SINT16 | mem::UNICODE_CHAR => 2,
        mem::UINT64 | mem::SINT64 | mem::FLOAT64 => 8,
        _ => 4,
    };
    let bytes = std::slice::from_raw_parts(data as *const u8, size * count).to_vec();
    game(u)
        .written
        .borrow_mut()
        .insert((occ, id), (mem_type, is_array, bytes, count));
}
unsafe extern "C" fn bounding_box(u: *mut c_void, occ: u64, a: *mut f32, b: *mut f32) {
    let r = game(u).cells_of(occ);
    let (a, b) = (
        std::slice::from_raw_parts_mut(a, 3),
        std::slice::from_raw_parts_mut(b, 3),
    );
    a.copy_from_slice(&[r.x1 as f32 * 16.0, 0.0, r.z1 as f32 * 16.0]);
    b.copy_from_slice(&[(r.x2 + 1) as f32 * 16.0, 10.0, (r.z2 + 1) as f32 * 16.0]);
}
unsafe extern "C" fn position(u: *mut c_void, occ: u64, out: *mut f32) {
    let r = game(u).cells_of(occ);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o.copy_from_slice(&[(r.x1 + r.x2 + 1) as f32 * 8.0, 0.0, (r.z1 + r.z2 + 1) as f32 * 8.0]);
}
unsafe extern "C" fn bounding_cells(u: *mut c_void, occ: u64, out: *mut fc4_cell_rect) {
    *out = game(u).cells_of(occ);
}
unsafe extern "C" fn in_manager(_: *mut c_void, _occ: u64) -> bool {
    true
}
unsafe extern "C" fn occupants_in(u: *mut c_void, r: *const fc4_cell_rect, cb: fc4_token_callback, ctx: *mut c_void) {
    let r = &*r;
    let Some(cb) = cb else { return };
    let g = game(u);
    for (occ, c, _) in g.occupants.borrow().iter() {
        if c.x1 <= r.x2 && r.x1 <= c.x2 && c.z1 <= r.z2 && r.z1 <= c.z2 && !g.gone.borrow().contains(occ) {
            cb(ctx, *occ);
        }
    }
}
unsafe extern "C" fn zone_type(u: *mut c_void, x: i32, z: i32) -> u32 {
    u32::from(game(u).zones.borrow().contains(&(x, z)))
}
unsafe extern "C" fn department(u: *mut c_void, id: u32) -> u64 {
    if id == budget::DEPARTMENT {
        game(u).dept.get()
    } else {
        0
    }
}
unsafe extern "C" fn create_department(u: *mut c_void, _id: u32, _group: u32) -> u64 {
    game(u).dept.set(DEPT);
    DEPT
}
unsafe extern "C" fn department_funding(_: *mut c_void, _d: u64) -> f32 {
    1.0
}
unsafe extern "C" fn create_line_item(_: *mut c_void, _d: u64, line: u32, _f: bool) -> u64 {
    0x1000 + u64::from(line)
}
unsafe extern "C" fn budget_item(_: *mut c_void, occ: u64, purpose: u32, out: *mut fc4_power_budget_item) -> bool {
    if occ != PLANT || purpose != budget::PURPOSE {
        return false;
    }
    *out = fc4_power_budget_item {
        department: budget::DEPARTMENT,
        line: 7,
        cost: 250,
        index: 0,
        local_funding: 1.0,
    };
    true
}

fn host(g: &Game) -> fc4_power_host {
    let mut h: fc4_power_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_power_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.grids = fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user: h.user,
        create: Some(g_create),
        class_of: Some(g_class),
        geometry: Some(g_geometry),
        get: Some(g_get),
        set: Some(g_set),
        fill: Some(g_fill),
        shutdown: Some(g_noop),
        release: Some(g_noop),
    };
    h.cell_counts = Some(cell_counts);
    h.cell_widths = Some(cell_widths);
    h.tuning = Some(tuning);
    h.post = Some(post);
    h.resolve_ref = Some(resolve_ref);
    h.occupant_type = Some(occupant_type);
    h.occupant_property = Some(occupant_property);
    h.set_occupant_property = Some(set_occupant_property);
    h.occupant_position = Some(position);
    h.occupant_bounding_box = Some(bounding_box);
    h.occupant_bounding_cells = Some(bounding_cells);
    h.occupant_in_manager = Some(in_manager);
    h.occupants_in_cells = Some(occupants_in);
    h.zone_type = Some(zone_type);
    h.department = Some(department);
    h.create_department = Some(create_department);
    h.department_funding = Some(department_funding);
    h.create_line_item = Some(create_line_item);
    h.create_line_item_for_building_type = Some(create_line_item);
    h.budget_item_for_purpose = Some(budget_item);
    h
}

// ---- the capture ----------------------------------------------------------------------------------------

/// The plugin's capture stream: a reference is `u32 objectID` (index + 1,
/// 0 = null) then `u32 classID`.
unsafe extern "C" fn capture_ref(u: *mut c_void, r: *const fc4_ref) -> bool {
    let o = &mut *(u as *mut Out);
    let r = &*r;
    if r.token == 0 {
        o.bytes.extend_from_slice(&0u32.to_le_bytes());
        return true;
    }
    o.refs.push((r.token, r.class_id));
    o.bytes.extend_from_slice(&(o.refs.len() as u32).to_le_bytes());
    o.bytes.extend_from_slice(&r.class_id.to_le_bytes());
    true
}

struct Captured {
    bytes: Vec<u8>,
    refs: Vec<fc4_power_capture_ref>,
}

impl Captured {
    fn of(p: *const fc4_power) -> Captured {
        let mut out = Out::default();
        let mut os = ostream(&mut out);
        os.put_ref = Some(capture_ref);
        assert!(unsafe { fc4_power_write(p, &os) });
        let refs = out
            .refs
            .iter()
            .map(|&(t, class_id)| fc4_power_capture_ref {
                occupant: if t == DEPT || class_id == GRID_CLSID { 0 } else { t },
                grid: if class_id == GRID_CLSID { t } else { 0 },
                department: if t == DEPT { t } else { 0 },
                class_id,
                flags: 0,
            })
            .collect();
        Captured { bytes: out.bytes, refs }
    }
    fn c(&self) -> fc4_power_capture {
        fc4_power_capture {
            struct_size: std::mem::size_of::<fc4_power_capture>() as u32,
            ok: 1,
            bytes: self.bytes.as_ptr(),
            len: self.bytes.len(),
            refs: self.refs.as_ptr(),
            ref_count: self.refs.len(),
        }
    }
    /// The byte offset of ringIndex: after the version and the department
    /// reference.
    fn ring_index_at(&self) -> usize {
        let id = u32::from_le_bytes(self.bytes[2..6].try_into().unwrap());
        if id == 0 {
            6
        } else {
            10
        }
    }
    /// lastImport: ringIndex, production, consumption, deficit, consumers,
    /// powered, unpowered, requirement (4 bytes each), costBase (8),
    /// lastProduction, lastConsumption, then lastImport (§9.1).
    fn last_import_at(&self) -> usize {
        self.ring_index_at() + 48
    }
    fn add_i32(&mut self, at: usize, d: i32) {
        let v = i32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap());
        self.bytes[at..at + 4].copy_from_slice(&(v + d).to_le_bytes());
    }
}

fn counts_of(p: *const fc4_power) -> fc4_power_shadow_counts {
    unsafe {
        fc4_power_shadow_counts {
            struct_size: std::mem::size_of::<fc4_power_shadow_counts>() as u32,
            ok: 1,
            overworked: fc4_power_slot_value(p, 27) as u32,
            aged: fc4_power_slot_value(p, 28) as u32,
            distressed: fc4_power_slot_value(p, 29) as u32,
            reserved: 0,
        }
    }
}

unsafe extern "C" fn line_sink(ctx: *mut c_void, data: *const u8, len: usize) {
    let l = String::from_utf8_lossy(std::slice::from_raw_parts(data, len)).into_owned();
    (*(ctx as *mut Vec<String>)).push(l);
}

/// A game, its "original" simulator and a shadow over the same game.
struct World {
    g: Box<Game>,
    p: *mut fc4_power,
    s: *mut fc4_power_shadow,
    lines: Vec<String>,
    /// One of the original's messages arrived since the last wrapped
    /// delivery (the plugin's message-target flag).
    event: bool,
}

fn shadow_of(g: &Game, flags: u32) -> *mut fc4_power_shadow {
    let h = host(g);
    let o = fc4_power_shadow_options {
        struct_size: std::mem::size_of::<fc4_power_shadow_options>() as u32,
        max_cells: 0,
        max_lines: 0,
        flags,
        connection_visited: None,
    };
    let s = unsafe { fc4_power_shadow_create(&h, &o) };
    assert!(!s.is_null());
    s
}

impl World {
    /// The original just initialised, the shadow created as at the city's
    /// PostCityInit (the plugin's).
    fn new(g: Box<Game>, connections_ready: bool) -> World {
        World::with_flags(g, connections_ready, FC4_POWER_SHADOW_CITY_START)
    }

    /// A shadow switched on mid-city: nothing unsaved is known.
    fn mid_city(g: Box<Game>, connections_ready: bool) -> World {
        World::with_flags(g, connections_ready, 0)
    }

    fn with_flags(g: Box<Game>, connections_ready: bool, flags: u32) -> World {
        let h = host(&g);
        let p = unsafe { fc4_power_create(&h, 0x51) };
        assert!(!p.is_null());
        assert!(unsafe { fc4_power_init(p) });
        if connections_ready {
            message(p, msg::CONNECTIONS_READY, 0, 0);
        }
        let n = g.occupants.borrow().len();
        for i in 0..n {
            let occ = g.occupants.borrow()[i].0;
            message(p, msg::OCCUPANT_INSERTED, occ, 0);
        }
        let s = shadow_of(&g, flags);
        World {
            g,
            p,
            s,
            lines: Vec::new(),
            event: false,
        }
    }

    /// A new shadow over the same original.
    fn restart(&mut self, flags: u32) {
        unsafe { fc4_power_shadow_destroy(self.s) };
        self.s = shadow_of(&self.g, flags);
        self.lines.clear();
    }

    /// One wrapped delivery; `tamper` changes the game or the record after
    /// the original's update. None: passed through.
    fn deliver(&mut self, msg_type: u32, event: bool, tamper: impl FnOnce(&Game, &mut Captured)) -> Option<i32> {
        let julian = FEB_2000;
        let event = std::mem::take(&mut self.event) || event;
        if !unsafe { fc4_power_shadow_wants(self.s, msg_type, event) } {
            message(self.p, msg_type, 0, julian as u32);
            return None;
        }
        let before = Captured::of(self.p);
        let cb = counts_of(self.p);
        let r = unsafe { fc4_power_shadow_begin(self.s, msg_type, julian, &before.c(), &cb) };
        assert_eq!(r, 0, "our run on the original's record");
        message(self.p, msg_type, 0, julian as u32);
        let mut after = Captured::of(self.p);
        tamper(&self.g, &mut after);
        let ca = counts_of(self.p);
        let lines = &mut self.lines as *mut Vec<String> as *mut c_void;
        Some(unsafe { fc4_power_shadow_finish(self.s, &after.c(), &ca, Some(line_sink), lines) })
    }

    fn step(&mut self) -> Option<i32> {
        self.deliver(msg::SIM_IDLE, false, |_, _| {})
    }

    /// Wrapped idle deliveries until the original is back at state 4.
    fn cycle(&mut self) -> Vec<i32> {
        let mut out = Vec::new();
        for _ in 0..8 {
            if let Some(n) = self.step() {
                out.push(n);
            }
            if state_of(self.p).state == state::IDLE && !out.is_empty() {
                return out;
            }
        }
        panic!("the cycle did not finish");
    }

    /// One of the original's messages (§5), outside the wrapped deliveries:
    /// delivered to the original and forwarded to the shadow (the plugin's
    /// message target).
    fn notify(&mut self, msg_type: u32, occ: u64) {
        message(self.p, msg_type, occ, 0);
        let m = fc4_power_message {
            msg_type,
            void1: occ,
            ..Default::default()
        };
        unsafe { fc4_power_shadow_message(self.s, &m) };
        self.event = true;
    }

    fn stats(&self) -> fc4_power_shadow_stats {
        let mut st = fc4_power_shadow_stats::default();
        assert!(unsafe { fc4_power_shadow_get_stats(self.s, &mut st) });
        st
    }

    fn last(&self, kind: &str) -> &str {
        self.lines
            .iter()
            .rev()
            .find(|l| l.contains(&format!(" {kind}")))
            .unwrap_or_else(|| panic!("no {kind} line in {:#?}", self.lines))
    }
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe {
            fc4_power_shadow_destroy(self.s);
            fc4_power_shutdown(self.p);
            fc4_power_destroy(self.p);
        }
    }
}

fn message(p: *mut fc4_power, msg_type: u32, void1: u64, data1: u32) {
    let m = fc4_power_message {
        msg_type,
        void1,
        data1,
        ..Default::default()
    };
    assert!(unsafe { fc4_power_do_message(p, &m) });
}

fn state_of(p: *const fc4_power) -> fc4_power_state {
    let mut s = fc4_power_state::default();
    assert!(unsafe { fc4_power_get_state(p, &mut s) });
    s
}

fn mismatches(line: &str) -> u64 {
    let at = line.rfind("mismatches=").expect("mismatches= on every line") + "mismatches=".len();
    line[at..].trim().parse().unwrap()
}

#[test]
fn create_rejects_bad_tables_and_null_handles_answer_fallbacks() {
    let g = Game::new(1000, &[500]);
    let mut h = host(&g);
    h.struct_size = 8;
    assert!(unsafe { fc4_power_shadow_create(&h, std::ptr::null()) }.is_null());
    let mut h = host(&g);
    h.grids.get = None;
    assert!(unsafe { fc4_power_shadow_create(&h, std::ptr::null()) }.is_null());
    assert!(unsafe { fc4_power_shadow_create(std::ptr::null(), std::ptr::null()) }.is_null());
    unsafe {
        let null = std::ptr::null_mut();
        assert!(!fc4_power_shadow_wants(null, msg::SIM_NEW_MONTH, false));
        assert_eq!(
            fc4_power_shadow_begin(null, msg::SIM_IDLE, 0, std::ptr::null(), std::ptr::null()),
            -1
        );
        assert_eq!(
            fc4_power_shadow_finish(null, std::ptr::null(), std::ptr::null(), None, null.cast()),
            -1
        );
        let mut st = fc4_power_shadow_stats::default();
        assert!(!fc4_power_shadow_get_stats(null, &mut st));
        fc4_power_shadow_destroy(null);
    }
    // Another message type is not wrapped.
    let h = host(&g);
    let s = unsafe { fc4_power_shadow_create(&h, std::ptr::null()) };
    unsafe {
        assert!(!fc4_power_shadow_wants(s, msg::OCCUPANT_INSERTED, true));
        assert_eq!(
            fc4_power_shadow_begin(s, msg::OCCUPANT_INSERTED, 0, std::ptr::null(), std::ptr::null()),
            -2
        );
        // A failed capture: not compared, an INCOMPLETE line.
        assert_eq!(
            fc4_power_shadow_begin(s, msg::SIM_IDLE, FEB_2000, std::ptr::null(), std::ptr::null()),
            -1
        );
        let mut lines: Vec<String> = Vec::new();
        let ctx = &mut lines as *mut Vec<String> as *mut c_void;
        assert_eq!(
            fc4_power_shadow_finish(s, std::ptr::null(), std::ptr::null(), Some(line_sink), ctx),
            -1
        );
        assert!(
            lines[0].starts_with("power shadow 2000-02-01 step: INCOMPLETE (before: no capture)"),
            "{lines:?}"
        );
        let mut st = fc4_power_shadow_stats::default();
        assert!(fc4_power_shadow_get_stats(s, &mut st));
        assert_eq!((st.deliveries, st.incomplete, st.last_mismatches), (1, 1, -1));
        fc4_power_shadow_destroy(s);
    }
}

#[test]
fn a_clean_cycle_and_month_have_no_mismatch() {
    let mut w = World::new(Game::new(1000, &[500]), true);
    let n = w.cycle();
    assert_eq!(n, vec![0, 0, 0, 0, 0], "4>0, 0, 1, 2, 3: {:#?}", w.lines);
    let st = state_of(w.p);
    assert_eq!((st.powered, st.last_production, st.last_consumption), (1, 1000, 500));
    let line = w.last("cycle:").to_string();
    assert!(
        line.starts_with("power shadow 2000-02-01 cycle: steps 4>0 0 1 2 3 on 32x32 cells"),
        "{line}"
    );
    assert!(line.contains("production=1000/1000 consumption=500/500"), "{line}");
    assert!(line.contains("powered=1/1 unpowered=0/0 plants=1/1"), "{line}");
    assert!(line.contains("max|d|=0"), "{line}");
    assert_eq!(mismatches(&line), 0);
    assert!(line.contains("props 2/0"), "the plant's used and wear compared: {line}");

    // Idle afterwards: passed through until a message arrives.
    assert_eq!(w.step(), None);
    assert_eq!(w.step(), None);
    assert_eq!(w.stats().idle_skipped, 2);
    assert_eq!(
        w.deliver(msg::SIM_IDLE, true, |_, _| {}),
        Some(0),
        "an event: wrapped again"
    );

    // The month.
    assert_eq!(
        w.deliver(msg::SIM_NEW_MONTH, false, |_, _| {}),
        Some(0),
        "{:#?}",
        w.lines
    );
    let month = w.last("month:").to_string();
    assert!(
        month.starts_with("power shadow 2000-02-01 month: 1024 cells compared;"),
        "{month}"
    );
    assert_eq!(mismatches(&month), 0, "{month}");
    let book = w.last("bookkeeping:").to_string();
    assert!(book.contains("of the 2 live occupants"), "{book}");
    assert!(
        book.contains("consumers=1/1") && book.contains("plantCount=1/1"),
        "{book}"
    );
    assert_eq!(mismatches(&book), 0, "{book}");
    let summary = w.last("month to").to_string();
    assert!(
        summary.starts_with("power shadow month to 2000-02-01: deliveries="),
        "{summary}"
    );
    assert_eq!(mismatches(&summary), 0, "{summary}");

    // The month asked for a recompute: the next cycle is clean too.
    let n = w.cycle();
    assert!(n.iter().all(|&m| m == 0), "{:#?}", w.lines);
    let st = w.stats();
    assert_eq!((st.mismatches, st.not_exact, st.incomplete), (0, 0, 0));
    assert_eq!(st.months, 1);
    assert_eq!(st.cycles, 2);
    assert_eq!(st.last_mismatches, 0);
    assert!(w.g.posts.get() > 0, "the original posted; the shadow did not");
}

#[test]
fn one_wrong_grid_cell_is_one_mismatch() {
    let mut w = World::new(Game::new(1000, &[500]), true);
    // 4>0, 0, 1: then the neighbours step with the grid write.
    for _ in 0..3 {
        assert_eq!(w.step(), Some(0), "{:#?}", w.lines);
    }
    assert_eq!(state_of(w.p).state, state::NEIGHBOURS);
    let n = w.deliver(msg::SIM_IDLE, false, |g, _| g.set_cell(13, 1, 1));
    assert_eq!(n, Some(1), "{:#?}", w.lines);
    assert_eq!(w.step(), Some(0));
    let line = w.last("cycle:").to_string();
    assert_eq!(mismatches(&line), 1, "{line}");
    assert!(
        line.contains("grid(13,1) 1/2 [2]"),
        "the first differing cell, orig/ours: {line}"
    );
    assert!(line.contains("max|d|=1"), "{line}");
    assert!(line.contains("steps 4>0 0 1 2! 3"), "{line}");
}

#[test]
fn one_wrong_scalar_is_one_mismatch() {
    let mut w = World::new(Game::new(1000, &[500]), true);
    w.cycle();
    let n = w.deliver(msg::SIM_NEW_MONTH, false, |_, a| {
        let at = a.ring_index_at();
        let v = i32::from_le_bytes(a.bytes[at..at + 4].try_into().unwrap());
        a.bytes[at..at + 4].copy_from_slice(&(v + 1).to_le_bytes());
    });
    assert_eq!(n, Some(1), "{:#?}", w.lines);
    let month = w.last("month:").to_string();
    assert!(month.contains("fields orig/ours: ringIndex="), "{month}");
    assert_eq!(mismatches(&month), 1, "{month}");
    assert_eq!(mismatches(w.last("month to")), 1);
    assert_eq!(w.stats().mismatches, 1);
}

#[test]
fn steps_before_connections_ready_are_not_compared() {
    // ConnectionsReady never came: the original's DoIdle does nothing.
    let mut w = World::new(Game::new(1000, &[500]), false);
    assert_eq!(w.step(), Some(-1));
    assert_eq!(w.step(), Some(-1));
    let st = w.stats();
    assert_eq!((st.not_enabled, st.steps, st.mismatches), (2, 0, 0));
    assert_eq!(state_of(w.p).state, state::IDLE);
}

#[test]
fn a_shortfall_takes_the_original_block_order() {
    // 1000 of supply for three consumers of 600: one block gets power, which
    // one depends on the original's rand() draw. Cycles move the original's
    // CRT state on; the shadow's simulator restarts from the default seed
    // each run, so its own draws differ.
    let mut w = World::new(Game::new(1000, &[600, 600, 600]), true);
    let mut powered = std::collections::BTreeSet::new();
    for _ in 0..6 {
        let n = w.cycle();
        assert!(n.iter().all(|&m| m == 0), "{:#?}", w.lines);
        let st = state_of(w.p);
        assert_eq!((st.powered, st.unpowered), (1, 2));
        let which: Vec<i32> = [12, 18, 24]
            .into_iter()
            .filter(|&x| unsafe { fc4_power_is_powered(w.p, x, 0) })
            .collect();
        powered.insert(which);
        // A removal and reinsertion asks for the next recompute.
        w.notify(msg::OCCUPANT_REMOVED, 13);
        w.notify(msg::OCCUPANT_INSERTED, 13);
    }
    assert!(powered.len() > 1, "the draws powered different blocks: {powered:?}");
    let line = w.last("cycle:").to_string();
    assert!(line.contains("order-dependent=1 matched=1 unmatched=0"), "{line}");
    let st = w.stats();
    assert_eq!((st.mismatches, st.not_exact), (0, 0));
    assert!(st.reruns >= 6, "each shortfall step ran again with the target");
}

#[test]
fn the_dirty_rectangle_comes_from_the_forwarded_messages() {
    // PW5, the example of power.md §5.3 (R = 2): a wire source along x = 10
    // (a building one cell wide wires x 8..12), the zoned cell (11, 15) and a
    // building at x 12..18 × z 12..18 that is removed. The original rebuilds
    // its dirty rect B = x 10..20 × z 10..20 and keeps (13, 13..17) wired.
    let game = || {
        let g = Game::new(1000, &[]);
        g.occupants.borrow_mut().push((20, rect(10, 0, 10, 31), 0));
        g.occupants.borrow_mut().push((21, rect(12, 12, 18, 18), 0));
        g.zones.borrow_mut().push((11, 15));
        g
    };
    // From a city start: the forwarded removal gives the shadow B.
    let mut w = World::new(game(), true);
    assert!(w.cycle().iter().all(|&n| n == 0), "{:#?}", w.lines);
    w.g.gone.borrow_mut().push(21);
    w.notify(msg::OCCUPANT_REMOVED, 21);
    let n = w.cycle();
    assert!(n.iter().all(|&m| m == 0), "{n:?} {:#?}", w.lines);
    let line = w.last("cycle:").to_string();
    assert!(
        line.contains("wire rebuilt over the dirty rectangle (10,10)-(20,20) merged from the original's messages"),
        "{line}"
    );
    assert!(unsafe { fc4_power_is_wired(w.p, 13, 15) });
    assert_eq!(w.stats().reruns, 0);

    // Switched on mid-city after the removal: the rectangle is not known,
    // the changed bits' box A = x 13..20 × z 10..20 stands in, and its
    // rebuild loses (13, 13..17).
    let mut m = World::new(game(), true);
    m.cycle();
    m.g.gone.borrow_mut().push(21);
    message(m.p, msg::OCCUPANT_REMOVED, 21, 0);
    m.restart(0);
    m.event = true;
    let first = m.step().expect("wrapped");
    assert!(first > 0, "{:#?}", m.lines);
    m.cycle();
    let line = m.last("cycle:").to_string();
    assert!(
        line.contains("rerun with the changed bits' box (13,10)-(20,20)"),
        "{line}"
    );
    assert!(line.contains("wire(13,13) 1/0"), "orig/ours: {line}");
}

#[test]
fn a_city_start_knows_the_pass_values() {
    // PW3 (§9.1): the pass import / export / demand are 0 after a load, so a
    // shadow started at the city start compares state 3's totals exactly;
    // one started mid-city before a state 0 cannot.
    let mut w = World::mid_city(Game::new(1000, &[500]), true);
    for _ in 0..4 {
        message(w.p, msg::SIM_IDLE, 0, FEB_2000 as u32); // 4>0, 0, 1, 2: unobserved
    }
    assert_eq!(state_of(w.p).state, state::STATISTICS);
    let tamper = |_: &Game, a: &mut Captured| {
        let at = a.last_import_at();
        a.add_i32(at, 7);
    };
    let n = w.deliver(msg::SIM_IDLE, true, tamper);
    assert_eq!(n, Some(0), "{:#?}", w.lines);
    // lastImport and lastDemand: the original's demand comes from its own
    // unobserved states 1 and 2.
    assert_eq!(w.stats().not_exact, 2);
    let line = w.last("cycle:").to_string();
    assert!(line.contains("lastImport 7/0 not exact"), "{line}");
    assert!(line.contains("pass import/export/demand not observed"), "{line}");

    // The same step seen from a city start: a mismatch.
    let mut w = World::new(Game::new(1000, &[500]), true);
    for _ in 0..4 {
        assert_eq!(w.step(), Some(0), "{:#?}", w.lines); // 4>0, 0, 1, 2
    }
    assert_eq!(state_of(w.p).state, state::STATISTICS);
    let n = w.deliver(msg::SIM_IDLE, true, tamper);
    assert_eq!(n, Some(1), "{:#?}", w.lines);
    let line = w.last("cycle:").to_string();
    assert!(line.contains("lastImport=7/0"), "{line}");
    assert!(w.deliver(msg::SIM_NEW_MONTH, false, |_, _| {}).is_some());
    let summary = w.last("month to").to_string();
    assert!(
        summary.contains("factor=1 pass totals known dirty rectangle known"),
        "{summary}"
    );
}

#[test]
fn mid_city_the_factor_is_not_known_until_the_first_month() {
    // PW7 (§8): mid-city, the original's factor may be last month's; the
    // states that divide by it (1, 2) are not exact until a month start.
    let mut w = World::mid_city(Game::new(1000, &[500]), true);
    for _ in 0..3 {
        assert_eq!(w.step(), Some(0), "{:#?}", w.lines);
    }
    assert_eq!(state_of(w.p).state, state::NEIGHBOURS);
    let n = w.deliver(msg::SIM_IDLE, false, |g, _| g.set_cell(13, 1, 1));
    assert_eq!(n, Some(0), "{:#?}", w.lines);
    assert_eq!(w.stats().not_exact, 1);
    assert_eq!(w.step(), Some(0));
    let line = w.last("cycle:").to_string();
    assert!(
        line.contains("the ordinance factor is not known until the first month"),
        "{line}"
    );
    assert!(w.deliver(msg::SIM_NEW_MONTH, false, |_, _| {}).is_some());
    let month = w.last("month:").to_string();
    assert!(month.contains("brokenFraction not known"), "{month}");
    // After the month: the same difference is a mismatch.
    for _ in 0..3 {
        assert_eq!(w.step(), Some(0), "{:#?}", w.lines);
    }
    let n = w.deliver(msg::SIM_IDLE, false, |g, _| g.set_cell(12, 0, 1));
    assert_eq!(n, Some(1), "{:#?}", w.lines);
}

/// One wrapped SimNewMonth; its bookkeeping line.
fn book_month(w: &mut World) -> String {
    assert!(w.deliver(msg::SIM_NEW_MONTH, false, |_, _| {}).is_some());
    w.last("bookkeeping:").to_string()
}

const INHERITED: &str = "inherited (as at the baseline line 2000-02-01, power.md §5.2 case 1)";
const ABSENT: &str = "baseline entries absent (0 at this line: masked by a difference in flight; kept)";
const WAITING: &str = "unchanged while others changed (not inherited; counted at the next line)";
const PERSISTENT: &str =
    "persistent (not inherited; unchanged since the last line with nothing in flight, or at three lines)";
const WIRE_BITS: &str = "; wire bits, not counted (the original's running bitmap after its dirty-rectangle \
                         rebuilds vs one rebuild of the whole map, power.md §5.3: PW5, unclipped bits, no insertion \
                         filters; each rebuild is compared on the cycle lines)";

#[test]
fn the_bookkeeping_line_sets_apart_inherited_and_in_flight() {
    // PW8 (§5.2): the original's running counts keep what each delivery
    // read. Consumer 11 consumed 500 when inserted and 400 now: the +100 is
    // inherited; the baseline is the first line equal to the one before it.
    // An occupant the original has not been told of is in flight until it
    // is, and only a difference that stays without being inherited is a
    // mismatch.
    let mut w = World::new(Game::new(1000, &[500]), true);
    w.cycle();
    w.g.occupants.borrow_mut()[1].2 = 400;
    let month = book_month;
    let b = month(&mut w);
    assert!(
        b.contains(
            "baseline not set yet (the first line since the city start; the baseline is the first line equal to \
             the one before it, nothing counted until then): 2 differences [requirement +100, cell (13,1) +100]"
        ),
        "{b}"
    );
    assert_eq!(mismatches(&b), 0);
    let b = month(&mut w);
    assert!(
        b.contains(
            "baseline (the first line equal to the one before it since the city start, nothing in flight): 2 \
             differences inherited from the save (power.md §5.2 case 1) [requirement +100, cell (13,1) +100]"
        ),
        "{b}"
    );
    assert_eq!(mismatches(&b), 0);
    let b = month(&mut w);
    assert!(b.contains(&format!("{INHERITED} 2")), "{b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    // Built, not yet delivered to the original.
    w.g.occupants.borrow_mut().push((30, rect(30, 10, 31, 11), 50));
    let b = month(&mut w);
    assert!(b.contains("in flight (changed since the last line, case 2)"), "{b}");
    assert!(b.contains("consumers -1"), "{b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    w.notify(msg::OCCUPANT_INSERTED, 30);
    let b = month(&mut w);
    assert!(b.contains("in flight (changed since the last line, case 2) 0;"), "{b}");
    assert!(!b.contains("gone since the last line 0;"), "{b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    // Never delivered: in flight, then persistent.
    w.g.occupants.borrow_mut().push((31, rect(24, 20, 25, 21), 70));
    assert_eq!(mismatches(&month(&mut w)), 0);
    let b = month(&mut w);
    // The requirement's +100 plus 31's -70: not the baseline's either. The
    // wire bits of 31's neighbourhood are listed, not counted.
    assert!(
        b.contains(&format!(
            "{PERSISTENT} 3 [consumers -1, requirement +30, cell (25,21) -70]{WIRE_BITS} 36 [wire (22,18) 0/1,"
        )),
        "{b}"
    );
    assert_eq!(mismatches(&b), 3, "{b}");
    assert_eq!(w.stats().bookkeeping_mismatches, 3);
}

#[test]
fn a_baseline_entry_masked_by_a_difference_in_flight_stays_inherited() {
    // Run gsT1, 2027-12-01: the save's consumers +2 and two new buildings
    // not yet delivered to the original gave consumers 660/660, and the +2
    // was back the month after. It left the baseline at the 0 and was
    // counted as persistent at every later line.
    let mut w = World::new(Game::new(1000, &[500]), true);
    w.cycle();
    // Consumer 11 consumed 500 when inserted and nothing now: the original
    // still counts it (consumers +1, requirement +500, cell (13,1) +500).
    w.g.occupants.borrow_mut()[1].2 = 0;
    book_month(&mut w);
    let b = book_month(&mut w);
    assert!(
        b.contains("3 differences inherited from the save (power.md §5.2 case 1) [consumers +1, requirement +500, cell (13,1) +500]"),
        "{b}"
    );
    // A consumer of 500 built, not yet delivered: consumers and requirement
    // read 0, the cell is still inherited.
    w.g.occupants.borrow_mut().push((30, rect(30, 10, 31, 11), 500));
    let b = book_month(&mut w);
    assert!(b.contains("consumers=1/1; requirement=500/500;"), "{b}");
    assert!(b.contains(&format!("{INHERITED} 1 [cell (13,1) +500]")), "{b}");
    assert!(
        b.contains(&format!("{ABSENT} 2 [consumers +1, requirement +500]")),
        "{b}"
    );
    assert!(b.contains("cell (31,11) -500"), "in flight: {b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    // Delivered: the baseline's entries are back, inherited.
    w.notify(msg::OCCUPANT_INSERTED, 30);
    for _ in 0..3 {
        let b = book_month(&mut w);
        assert!(
            b.contains(&format!(
                "{INHERITED} 3 [consumers +1, requirement +500, cell (13,1) +500]"
            )),
            "{b}"
        );
        assert!(b.contains(&format!("{ABSENT} 0;")), "{b}");
        assert!(b.contains(&format!("{PERSISTENT} 0;")), "{b}");
        assert_eq!(mismatches(&b), 0, "{b}");
    }
    assert_eq!(w.stats().bookkeeping_mismatches, 0);
}

#[test]
fn a_first_line_with_a_removal_in_flight_is_not_the_baseline() {
    // Runs pt-base-turbo-1 and pt-base-normal-1, 2026-02-01: the first line
    // had consumers +3, the save's +2 and +1 in flight; from the next line on
    // +2, which was counted as persistent against the first line at every
    // later line. Here the +1 in flight is a removal not yet delivered.
    let mut w = World::new(Game::new(1000, &[500, 70]), true);
    w.cycle();
    // Inherited, as above.
    w.g.occupants.borrow_mut()[1].2 = 0;
    // Consumer 12 (70 at x 18..19) gone from the city, its removal message
    // not yet delivered: the original still counts it.
    w.g.gone.borrow_mut().push(12);
    let b = book_month(&mut w);
    assert!(
        b.contains("baseline not set yet (the first line since the city start;"),
        "{b}"
    );
    assert!(
        b.contains(&format!(
            "[consumers +2, requirement +570, cell (13,1) +500, cell (19,1) +70]{WIRE_BITS} "
        )),
        "{b}"
    );
    assert!(b.contains("1/0"), "12's wire bits, the original's only: {b}");
    w.notify(msg::OCCUPANT_REMOVED, 12);
    w.cycle(); // the original rebuilds the wires around the removal
    let b = book_month(&mut w);
    assert!(
        b.contains("differences changed since the last line: in flight, power.md §5.2 case 2;"),
        "{b}"
    );
    assert_eq!(mismatches(&b), 0, "{b}");
    let b = book_month(&mut w);
    assert!(
        b.contains(
            "baseline (the first line equal to the one before it since the city start, nothing in flight): 3 \
             differences inherited from the save (power.md §5.2 case 1) [consumers +1, requirement +500, cell \
             (13,1) +500]"
        ),
        "{b}"
    );
    for _ in 0..2 {
        let b = book_month(&mut w);
        assert!(b.contains(&format!("{INHERITED} 3 [")), "{b}");
        assert_eq!(mismatches(&b), 0, "{b}");
    }
    assert_eq!(w.stats().bookkeeping_mismatches, 0);
}

#[test]
fn two_buildings_in_flight_at_two_lines_are_not_a_mismatch() {
    // Under the turbo clock a month line often has a building in flight;
    // two lines in a row can show the same counts (consumers -1) but not the
    // same cells. A difference that really stays is still counted, at the
    // third line when the lines around it are not quiet.
    let mut w = World::new(Game::new(1000, &[500]), true);
    w.cycle();
    book_month(&mut w);
    let b = book_month(&mut w);
    assert!(b.contains("nothing in flight): 0 differences"), "{b}");
    w.g.occupants.borrow_mut().push((30, rect(30, 10, 31, 11), 50));
    let b = book_month(&mut w);
    assert!(b.contains("in flight (changed since the last line, case 2) "), "{b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    // 30 delivered, 31 (the same 50) in flight.
    w.notify(msg::OCCUPANT_INSERTED, 30);
    w.g.occupants.borrow_mut().push((31, rect(24, 20, 25, 21), 50));
    let b = book_month(&mut w);
    assert!(
        b.contains(&format!("{WAITING} 2 [consumers -1, requirement -50]")),
        "{b}"
    );
    assert!(
        b.contains("gone since the last line ") && b.contains("cell (31,11) -50"),
        "{b}"
    );
    assert_eq!(mismatches(&b), 0, "{b}");
    // 31 never delivered; 32 in flight: 31's cell unchanged at two lines,
    // the counts changed.
    w.g.occupants.borrow_mut().push((32, rect(24, 26, 25, 27), 20));
    let b = book_month(&mut w);
    assert!(b.contains("cell (25,21) -50"), "{b}");
    assert!(b.contains(&format!("{PERSISTENT} 0;")), "{b}");
    assert_eq!(mismatches(&b), 0, "{b}");
    // 32 delivered: 31's cell unchanged at three lines, a mismatch.
    w.notify(msg::OCCUPANT_INSERTED, 32);
    let b = book_month(&mut w);
    assert!(
        b.contains(&format!("{PERSISTENT} ")) && b.contains("cell (25,21) -50"),
        "{b}"
    );
    assert!(mismatches(&b) > 0, "{b}");
}

#[test]
fn wire_bits_a_rectangle_rebuild_lost_are_listed_not_counted() {
    // PW5 (§5.3), as in runs pt-fix1-turbo-1 / -2: a building along x = 10
    // wires x 8..12 and the zoned cell (11,15) wires x 9..13 × z 13..17.
    // Building 21 at x 14..18 × z 14..16 is removed; the original rebuilds
    // its rectangle x 12..20 × z 12..18, which does not hold (11,15), so
    // (13,13..17) lose their bits for good. One rebuild of the whole map
    // keeps them. The cycle line compares the original's rebuild exactly.
    let g = Game::new(1000, &[]);
    g.occupants.borrow_mut().push((20, rect(10, 0, 10, 31), 0));
    g.occupants.borrow_mut().push((21, rect(14, 14, 18, 16), 0));
    g.zones.borrow_mut().push((11, 15));
    let mut w = World::new(g, true);
    w.cycle();
    book_month(&mut w);
    let b = book_month(&mut w);
    assert!(
        b.contains(&format!(
            "nothing in flight): 0 differences inherited from the save (power.md §5.2 case 1){WIRE_BITS} 0;"
        )),
        "{b}"
    );
    w.g.gone.borrow_mut().push(21);
    w.notify(msg::OCCUPANT_REMOVED, 21);
    assert!(w.cycle().iter().all(|&n| n == 0), "{:#?}", w.lines);
    assert!(!unsafe { fc4_power_is_wired(w.p, 13, 15) });
    for _ in 0..3 {
        let b = book_month(&mut w);
        assert!(b.contains("wire 5 cells differ;"), "{b}");
        assert!(
            b.contains(&format!(
                "{PERSISTENT} 0{WIRE_BITS} 5 [wire (13,13) 0/1, wire (13,14) 0/1, wire (13,15) 0/1, wire (13,16) \
                 0/1, wire (13,17) 0/1]; mismatches=0"
            )),
            "{b}"
        );
    }
    assert_eq!(w.stats().bookkeeping_mismatches, 0);
}
