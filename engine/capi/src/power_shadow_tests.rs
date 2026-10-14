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

/// (element type, array flag, bytes, count).
type Written = (u16, bool, Vec<u8>, usize);

struct Game {
    grids: RefCell<HashMap<u64, Vec<u8>>>,
    next_grid: Cell<u64>,
    /// Occupant → (cells, capacity or requirement).
    occupants: RefCell<Vec<(u64, fc4_cell_rect, u32)>>,
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
    for (occ, c, _) in game(u).occupants.borrow().iter() {
        if c.x1 <= r.x2 && r.x1 <= c.x2 && c.z1 <= r.z2 && r.z1 <= c.z2 {
            cb(ctx, *occ);
        }
    }
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

impl World {
    fn new(g: Box<Game>, connections_ready: bool) -> World {
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
        let s = unsafe { fc4_power_shadow_create(&h, std::ptr::null()) };
        assert!(!s.is_null());
        World {
            g,
            p,
            s,
            lines: Vec::new(),
            event: false,
        }
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

    /// One of the original's messages (§5), outside the wrapped deliveries.
    fn notify(&mut self, msg_type: u32, occ: u64) {
        message(self.p, msg_type, occ, 0);
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
