// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flammability shadow against a fake game whose "original" is our own
//! simulator, run day by day with a random iterator start: clean cycles
//! (with occupant, season and ordinance changes inside a cycle), a watered
//! state that changed and changed back inside a cycle (FL5/FL6), one wrong
//! tract, one wrong scalar, and the INCOMPLETE paths.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::flammability::{
    msg, occupant_type, prop, CellRect, CrtRand, FlammabilityHost, FlammabilityRandom, FlammabilitySim, Message,
};
use fc4_core::grid::{GridProvider, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::simgrid::CityCells;
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::flammability::{fc4_flammability_host, fc4_flammability_message, CHost};
use crate::flammability_shadow::*;
use crate::hostgrid::fc4_cell_rect;
use crate::landvalue::fc4_grid_geometry;
use crate::props::{fc4_props, fc4_props_set_i32s};

const N: i32 = 8;

struct Occ {
    kind: u32,
    rect: (i32, i32, i32, i32),
    value: i32,
}

#[derive(Default)]
struct Game {
    occs: RefCell<BTreeMap<u64, Occ>>,
    season: Cell<i32>,
    factor: Cell<f32>,
    /// Tracts a plumbing pass left without water (besides x >= 5).
    dry: RefCell<BTreeSet<(i32, i32)>>,
    /// The plumbing simulator's published grid (slot 7): its object
    /// alternates at every pass.
    published: Cell<u64>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn city_cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = N;
    *z = N;
}
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0) as i32;
    *cz = (z / 16.0) as i32;
}
unsafe extern "C" fn occupant_type_cb(u: *mut c_void, occ: u64) -> u32 {
    game(u).occs.borrow().get(&occ).map_or(0, |o| o.kind)
}
unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    match game(u).occs.borrow().get(&occ) {
        Some(o) if id == prop::FLAMMABILITY => fc4_props_set_i32s(p, id, &o.value, 1),
        _ => false,
    }
}
unsafe extern "C" fn occupant_bounds(u: *mut c_void, occ: u64, out: *mut fc4_cell_rect) {
    if let Some(o) = game(u).occs.borrow().get(&occ) {
        *out = fc4_cell_rect {
            x1: o.rect.0,
            z1: o.rect.1,
            x2: o.rect.2,
            z2: o.rect.3,
        };
    }
}
unsafe extern "C" fn occupants_in_cells(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let r = &*r;
    let Some(cb) = cb else { return };
    for (&t, o) in game(u).occs.borrow().iter() {
        if o.rect.0 <= r.x2 && r.x1 <= o.rect.2 && o.rect.1 <= r.z2 && r.z1 <= o.rect.3 {
            cb(ctx, t);
        }
    }
}
unsafe extern "C" fn cumulative_factor(u: *mut c_void, _p: u32, _i: u32, out: *mut f32) -> bool {
    *out = game(u).factor.get();
    true
}
unsafe extern "C" fn current_season(u: *mut c_void, out: *mut i32) -> bool {
    *out = game(u).season.get();
    true
}
unsafe extern "C" fn is_cell_watered(u: *mut c_void, x: i32, z: i32) -> i32 {
    i32::from(x < 5 && !game(u).dry.borrow().contains(&(x, z)))
}
unsafe extern "C" fn watered_grid(u: *mut c_void) -> u64 {
    game(u).published.get()
}
unsafe extern "C" fn zone_type(_: *mut c_void, _x: i32, z: i32, out: *mut u32) -> bool {
    *out = u32::from(z < 6);
    true
}

/// The read-only table the plugin hands the shadow: no grids, history,
/// fire, random lot or post.
fn read_only(g: &Game) -> fc4_flammability_host {
    // SAFETY: an all-zero table is valid (NULL callbacks).
    let mut h: fc4_flammability_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_flammability_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.city_cell_counts = Some(city_cell_counts);
    h.position_to_cell = Some(position_to_cell);
    h.occupant_type = Some(occupant_type_cb);
    h.occupant_property = Some(occupant_property);
    h.occupant_bounds = Some(occupant_bounds);
    h.occupants_in_cells = Some(occupants_in_cells);
    h.cumulative_factor = Some(cumulative_factor);
    h.current_season = Some(current_season);
    h.is_cell_watered = Some(is_cell_watered);
    h.zone_type = Some(zone_type);
    h.watered_grid = Some(watered_grid);
    h
}

/// The plumbing simulator's pass end message (plumbing §6.7 step 4).
const WATER_MAP_CHANGED: u32 = 0x6946_DC34;

/// The "original": our simulator on core grids, reading the same game.
struct OrigHost {
    c: CHost,
    cycles: Cell<u32>,
}

impl GridProvider<u8> for OrigHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        Some(SimGrid::<u8>::create(&CityCells::square(N), instance, tract_size))
    }
}

impl FlammabilityHost for OrigHost {
    fn city_cell_counts(&self) -> (i32, i32) {
        self.c.city_cell_counts()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.c.position_to_cell(x, z)
    }
    fn is_tutorial(&self) -> bool {
        true
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        None
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.c.occupant_type(occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.c.occupant_property(occ, id)
    }
    fn occupant_bounds(&self, occ: u64) -> CellRect {
        self.c.occupant_bounds(occ)
    }
    fn occupant_position(&self, _occ: u64) -> [f32; 3] {
        [0.0; 3]
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.c.occupants_in_cells(rect)
    }
    fn occupant_lot(&self, _occ: u64) -> Option<u64> {
        None
    }
    fn lot_condition(&self, _lot: u64) -> u8 {
        0
    }
    fn lot_building(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn random_lot(&self) -> Option<u64> {
        None
    }
    fn cumulative_factor(&self, p: u32, i: u32) -> Option<f32> {
        self.c.cumulative_factor(p, i)
    }
    fn current_season(&self) -> Option<i32> {
        self.c.current_season()
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> Option<bool> {
        self.c.is_cell_watered(x, z)
    }
    fn zone_type(&self, x: i32, z: i32) -> Option<u32> {
        self.c.zone_type(x, z)
    }
    fn record_history(&self, _t: u32, _v: i32) {}
    fn ignite_cell(&self, _x: u32, _z: u32, _p: bool, _t: bool, _f: u32) -> bool {
        false
    }
    fn set_fire_cause(&self, _cause: u32) {}
    fn post(&self, m: u32) {
        if m == msg::SIM_CYCLE_COMPLETE {
            self.cycles.set(self.cycles.get() + 1);
        }
    }
    fn grid_shutdown(&self, _grid: &Rc<dyn TractGrid<u8>>) {}
}

struct NoDraw;
impl FlammabilityRandom for NoDraw {
    fn range_f64(&mut self, a: f64, _b: f64) -> f64 {
        a
    }
}

/// What the plugin reads from the original at the cycle end.
struct Read {
    adjusted: Vec<u8>,
    raw: Vec<u8>,
    hotspots: Vec<fc4_flammability_shadow_hotspot>,
    flammable: Vec<u8>,
    o: fc4_flammability_original,
}

impl Read {
    fn original(&self) -> fc4_flammability_original {
        fc4_flammability_original {
            adjusted: self.adjusted.as_ptr(),
            raw: self.raw.as_ptr(),
            hotspots: self.hotspots.as_ptr(),
            hotspot_count: self.hotspots.len() as u32,
            flammable: self.flammable.as_ptr(),
            ..self.o
        }
    }
}

struct World {
    game: Box<Game>,
    host: Rc<OrigHost>,
    orig: Rc<FlammabilitySim>,
    shadow: *mut fc4_flammability_shadow,
    lines: Vec<String>,
    next_occ: u64,
    day: i32,
}

unsafe extern "C" fn sink(ctx: *mut c_void, p: *const u8, n: usize) {
    let lines = &mut *(ctx as *mut Vec<String>);
    lines.push(String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned());
}

impl World {
    fn new() -> World {
        let game = Box::<Game>::default();
        game.factor.set(1.0);
        let table = read_only(&game);
        let host = Rc::new(OrigHost {
            c: CHost(table),
            cycles: Cell::new(0),
        });
        let orig = FlammabilitySim::new(host.clone(), 0x5157, Box::new(NoDraw));
        orig.set_crt(CrtRand::new(12345));
        assert!(orig.init());
        let shadow = unsafe { fc4_flammability_shadow_create(&table, 3) };
        assert!(!shadow.is_null());
        let mut w = World {
            game,
            host,
            orig,
            shadow,
            lines: Vec::new(),
            next_occ: 0x1000,
            day: 2_451_545,
        };
        // A building on every cell: 150 ..= 249, so many tracts are hot
        // (code defaults: threshold 196, radius 8, minimum weight 16).
        for x in 0..N {
            for z in 0..N {
                w.insert(occupant_type::BUILDING, (x, z, x, z), 150 + (x * 37 + z * 11) % 100);
            }
        }
        // The multiplier is 0 until the first season or ordinance message (§3).
        w.send(msg::SEASON_CHANGE, 0);
        w
    }

    /// A message, to the shadow first (the plugin's subscription comes first).
    fn send(&mut self, msg_type: u32, occupant: u64) {
        let m = fc4_flammability_message {
            msg_type,
            reserved: 0,
            occupant,
            lot: 0,
        };
        unsafe { fc4_flammability_shadow_message(self.shadow, &m) };
        self.orig.do_message(&Message {
            msg_type,
            occupant: (occupant != 0).then_some(occupant),
            lot: None,
        });
    }

    fn insert(&mut self, kind: u32, rect: (i32, i32, i32, i32), value: i32) -> u64 {
        self.next_occ += 1;
        let t = self.next_occ;
        self.game.occs.borrow_mut().insert(t, Occ { kind, rect, value });
        self.send(msg::INSERT_OCCUPANT, t);
        t
    }

    /// A plumbing pass ends: `dry` is the new set of unwatered tracts, the
    /// published grid becomes the other object, and (`forward`) the plugin
    /// hands the shadow the pass end message.
    fn plumbing_pass(&mut self, dry: &[(i32, i32)], forward: bool) {
        *self.game.dry.borrow_mut() = dry.iter().copied().collect();
        let p = self.game.published.get();
        self.game.published.set(if p == 0xA0 { 0xB0 } else { 0xA0 });
        if forward {
            let m = fc4_flammability_message {
                msg_type: WATER_MAP_CHANGED,
                ..Default::default()
            };
            // Polled, not mirrored.
            assert!(!unsafe { fc4_flammability_shadow_message(self.shadow, &m) });
        }
    }

    fn remove(&mut self, t: u64) {
        // The game sends RemoveOccupant while the occupant is still listed.
        self.send(msg::REMOVE_OCCUPANT, t);
        self.game.occs.borrow_mut().remove(&t);
    }

    fn read(&self) -> Read {
        let a = self.orig.adjusted_grid().expect("adjusted");
        let r = self.orig.raw_grid().expect("raw");
        let geom = a.geometry();
        let mut adjusted = Vec::new();
        let mut raw = Vec::new();
        let mut flammable = Vec::new();
        for x in 0..geom.count_x {
            for z in 0..geom.count_z {
                adjusted.push(a.get(x, z));
                raw.push(r.get(x, z));
                flammable.push(u8::from(self.orig.is_cell_flammable(x, z)));
            }
        }
        let d = self.orig.snapshot();
        let hotspots = d
            .hotspots
            .iter()
            .map(|h| fc4_flammability_shadow_hotspot {
                x: h.x,
                z: h.z,
                weight: h.weight,
            })
            .collect();
        let gg = fc4_grid_geometry {
            tract_size: geom.tract_size,
            shift: geom.shift,
            count_x: geom.count_x,
            count_z: geom.count_z,
            width_x: geom.width_x,
            width_z: geom.width_z,
            inv_width_x: geom.inv_width_x,
            inv_width_z: geom.inv_width_z,
        };
        let hot = self.orig.flammability_hotspot();
        Read {
            adjusted,
            raw,
            hotspots,
            flammable,
            o: fc4_flammability_original {
                struct_size: std::mem::size_of::<fc4_flammability_original>() as u32,
                has_record: 1,
                simulator: 0x5157,
                geometry: gg,
                raw_geometry: gg,
                adjusted: std::ptr::null(),
                raw: std::ptr::null(),
                hotspots: std::ptr::null(),
                flammable: std::ptr::null(),
                hotspot_count: 0,
                global_multiplier: d.global_multiplier,
                average: d.average,
                average_getter: self.orig.average_flammability(),
                hotspot_found: u8::from(hot.is_some()),
                hotspot_value: hot.map_or(0, |h| h.0),
                hotspot_x: hot.map_or(0, |h| h.1),
                hotspot_z: hot.map_or(0, |h| h.2),
                cells_x: N,
                cells_z: N,
            },
        }
    }

    fn cycle_with(&mut self, r: &Read) -> i32 {
        let o = r.original();
        let ctx = &mut self.lines as *mut Vec<String> as *mut c_void;
        unsafe { fc4_flammability_shadow_cycle(self.shadow, &o, self.day, Some(sink), ctx) }
    }

    /// Runs days until the original ends a cycle; the read (delivered after
    /// the day, as the posted message is) and the shadow's result.
    fn run_to_cycle_end(&mut self) -> Read {
        let before = self.host.cycles.get();
        for _ in 0..200 {
            self.day += 1;
            // The day agent: not a subscribed message, the shadow does not see it.
            self.orig.do_message(&Message::of_type(msg::SIM_NEW_DAY));
            if self.host.cycles.get() != before {
                return self.read();
            }
        }
        panic!("no cycle end");
    }

    fn cycle(&mut self) -> i32 {
        let r = self.run_to_cycle_end();
        self.cycle_with(&r)
    }

    fn last(&self) -> &str {
        self.lines.last().map_or("", |s| s.as_str())
    }

    fn stats(&self) -> fc4_flammability_shadow_stats {
        let mut st = fc4_flammability_shadow_stats::default();
        assert!(unsafe { fc4_flammability_shadow_get_stats(self.shadow, &mut st) });
        st
    }
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe { fc4_flammability_shadow_destroy(self.shadow) };
    }
}

#[test]
fn clean_cycles_with_changes_inside_a_cycle() {
    let mut w = World::new();
    // Day 1: the empty EndOfCycle (§6.5) is the baseline.
    assert_eq!(w.cycle(), -1);
    assert!(
        w.last().contains("cycle 1: baseline (first cycle seen)"),
        "{}",
        w.last()
    );
    assert!(!w.last().contains("mismatches="), "{}", w.last());
    // A full cycle with no change.
    assert_eq!(w.cycle(), 0, "{}", w.last());
    println!("{}", w.last());
    let r = w.read();
    assert!(!r.hotspots.is_empty(), "the fake city must have hotspots");
    assert!(w.last().contains("raw cells=64 mismatched=0"), "{}", w.last());
    assert!(w.last().contains("adjusted cells=64 mismatched=0"), "{}", w.last());
    assert!(w.last().contains(" same (rotation "), "{}", w.last());
    assert!(
        w.last().contains("flammable getter cells=64 mismatched=0"),
        "{}",
        w.last()
    );
    assert!(w.last().contains("mismatches=0; cycles=1 clean=1"), "{}", w.last());

    // Inside the next cycle: a hot prop, a removal, summer, an ordinance.
    for _ in 0..20 {
        w.day += 1;
        w.orig.do_message(&Message::of_type(msg::SIM_NEW_DAY));
    }
    let prop_occ = w.insert(occupant_type::PROP, (2, 2, 3, 3), 255);
    let first = *w.game.occs.borrow().keys().next().expect("an occupant");
    w.remove(first);
    w.game.season.set(2);
    w.send(msg::SEASON_CHANGE, 0);
    for _ in 0..10 {
        w.day += 1;
        w.orig.do_message(&Message::of_type(msg::SIM_NEW_DAY));
    }
    w.game.factor.set(0.75);
    w.send(msg::ORDINANCE_UPDATED, 0);
    assert_eq!(w.cycle(), 0, "{}", w.last());
    println!("{}", w.last());
    assert!(w.last().contains("events=4"), "{}", w.last());
    // The next cycle runs on the new inputs throughout.
    w.remove(prop_occ);
    assert_eq!(w.cycle(), 0, "{}", w.last());
    assert_eq!(w.cycle(), 0, "{}", w.last());
    let st = w.stats();
    assert_eq!((st.cycles, st.compared, st.clean, st.last_mismatches), (5, 4, 4, 0));
    assert_eq!(st.events, 5);
}

/// FL5/FL6 (§6.5): a pass takes the water from a 2 × 2 block of watered
/// tracts, the original visits them dry (× 1.25), a second pass waters them
/// again before the cycle ends: equal states at both cycle ends, but the
/// tracts hold their dry values. The two passes, seen through the pointer
/// of the published grid, mark them.
#[test]
fn watered_state_changed_inside_a_cycle() {
    let block = [(1, 1), (1, 2), (2, 1), (2, 2)];
    for forward in [true, false] {
        let mut w = World::new();
        w.game.published.set(0xA0);
        assert_eq!(w.cycle(), -1);
        assert_eq!(w.cycle(), 0, "{}", w.last());
        assert!(
            w.last()
                .contains("; plumbing passes=0 (tracts whose watered state changed: 0)"),
            "{}",
            w.last()
        );
        w.plumbing_pass(&block, forward);
        let r = w.run_to_cycle_end();
        // At least one block tract was visited dry in this cycle.
        let dry_seen = block.iter().any(|&(x, z)| {
            let i = (x * N + z) as usize;
            r.adjusted[i] != r.raw[i]
        });
        assert!(dry_seen, "{:?} {:?}", r.adjusted, r.raw);
        w.plumbing_pass(&[], forward);
        let n = w.cycle_with(&r);
        if forward {
            assert_eq!(n, 0, "{}", w.last());
            assert!(
                w.last()
                    .contains("; plumbing passes=2 (tracts whose watered state changed: 4)"),
                "{}",
                w.last()
            );
            // The next cycle: the block is watered throughout (tracts the
            // wrap day visited dry keep their value, counted unvisited).
            assert_eq!(w.cycle(), 0, "{}", w.last());
        } else {
            // Without the pass end, the pointer is back where it was at the
            // previous poll: the change is not seen, and the dry values are
            // reported (that is why the plugin forwards the pass end).
            assert!(n > 0, "{}", w.last());
            assert!(w.last().contains("; plumbing passes=0 "), "{}", w.last());
        }
    }
}

/// Without the host's `watered_grid` the two cycle ends alone decide.
#[test]
fn watered_grid_not_polled() {
    let mut w = World::new();
    let g: &Game = &w.game;
    let table = fc4_flammability_host {
        watered_grid: None,
        ..read_only(g)
    };
    unsafe { fc4_flammability_shadow_destroy(w.shadow) };
    w.shadow = unsafe { fc4_flammability_shadow_create(&table, 3) };
    assert_eq!(w.cycle(), -1);
    assert_eq!(w.cycle(), 0, "{}", w.last());
    assert!(w.last().contains("; plumbing passes not polled"), "{}", w.last());
}

#[test]
fn one_wrong_tract() {
    let mut w = World::new();
    w.cycle();
    let mut r = w.run_to_cycle_end();
    // Raw tract (5, 6) (x outer).
    r.raw[5 * 8 + 6] ^= 0x01;
    assert_eq!(w.cycle_with(&r), 1, "{}", w.last());
    assert!(w.last().contains("raw cells=64 mismatched=1 max|d|=1"), "{}", w.last());
    assert!(
        w.last().contains("first differing (x,z) orig/ours: raw (5,6) "),
        "{}",
        w.last()
    );

    // An adjusted value no input explains.
    let mut w = World::new();
    w.cycle();
    let mut r = w.run_to_cycle_end();
    let i = 8 + 1; // tract (1, 1)
    r.adjusted[i] = r.adjusted[i].wrapping_add(3);
    assert_eq!(w.cycle_with(&r), 1, "{}", w.last());
    assert!(
        w.last().contains("adjusted cells=64 mismatched=1 max|d|=3"),
        "{}",
        w.last()
    );
}

#[test]
fn one_wrong_scalar() {
    let mut w = World::new();
    w.cycle();
    let mut r = w.run_to_cycle_end();
    r.o.global_multiplier = 0.5;
    assert_eq!(w.cycle_with(&r), 1, "{}", w.last());
    assert!(w.last().contains("multiplier orig/ours=0.5/1!"), "{}", w.last());

    let mut w = World::new();
    w.cycle();
    let mut r = w.run_to_cycle_end();
    r.o.average = r.o.average.wrapping_add(9);
    r.o.average_getter = r.o.average;
    assert_eq!(w.cycle_with(&r), 1, "{}", w.last());
    assert!(
        w.last().contains(&format!("average orig/ours={}/", r.o.average)),
        "{}",
        w.last()
    );

    let mut w = World::new();
    w.cycle();
    let mut r = w.run_to_cycle_end();
    r.hotspots[0].x += 50;
    assert!(w.cycle_with(&r) >= 1, "{}", w.last());
    assert!(w.last().contains(" differ (orig "), "{}", w.last());
}

#[test]
fn incomplete_and_null_safety() {
    unsafe {
        let n = std::ptr::null_mut();
        assert!(fc4_flammability_shadow_create(std::ptr::null(), 0).is_null());
        assert!(!fc4_flammability_shadow_message(n, std::ptr::null()));
        assert_eq!(
            fc4_flammability_shadow_cycle(n, std::ptr::null(), 0, None, n.cast()),
            -1
        );
        fc4_flammability_shadow_destroy(n);
    }
    let mut w = World::new();
    // Messages before the baseline are not mirrored.
    let m = fc4_flammability_message {
        msg_type: msg::SEASON_CHANGE,
        ..Default::default()
    };
    assert!(!unsafe { fc4_flammability_shadow_message(w.shadow, &m) });
    let mut r = w.run_to_cycle_end();
    r.o.has_record = 0;
    assert_eq!(w.cycle_with(&r), -1);
    assert!(
        w.last()
            .contains("INCOMPLETE (the original's record (its own Write) could not be read)"),
        "{}",
        w.last()
    );
    r.o.has_record = 1;
    r.o.geometry.tract_size = 2;
    assert_eq!(w.cycle_with(&r), -1);
    assert!(w.last().contains("FreeCity4 supports only 1"), "{}", w.last());
    // A new original simulator restarts the baseline.
    let mut r = w.run_to_cycle_end();
    assert_eq!(w.cycle_with(&r), -1);
    assert!(w.last().contains("baseline (first cycle seen)"), "{}", w.last());
    r.o.simulator = 0x7777;
    assert_eq!(w.cycle_with(&r), -1);
    assert!(w.last().contains("baseline (a new original simulator)"), "{}", w.last());
    assert_eq!(w.stats().compared, 0);
}
