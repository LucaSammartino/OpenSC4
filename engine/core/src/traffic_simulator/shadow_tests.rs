// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests of the traffic shadow (tooling): our own simulator over the stub
//! game stands in for the original, its record captured in the game's
//! capture format; the shadow must find nothing to report.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use super::*;
use crate::grid::{SimGrid, TractGeometry};
use crate::stream::{OutStream, RefValue, SaveObject};
use crate::traffic_simulator::shadow::{
    compare_record, compare_record_split, grid_class, CanonicalWriter, Capture, CapturedRef, Delivery, LiveGrids,
    TrafficShadow,
};

/// The capture stream of the plugin, for the stand-in original: an object
/// of ours (a grid) is identified by its address, a host object by its
/// token (a department or a building).
#[derive(Default)]
struct CaptureWriter {
    bytes: Vec<u8>,
    refs: Vec<CapturedRef>,
    ids: Vec<u64>,
    objects: HashMap<u64, Rc<dyn SaveObject>>,
}

impl CaptureWriter {
    fn emit(&mut self, b: &[u8]) -> bool {
        self.bytes.extend_from_slice(b);
        true
    }
    fn number(&mut self, token: u64, r: CapturedRef) -> u32 {
        match self.ids.iter().position(|&t| t == token) {
            Some(i) => i as u32 + 1,
            None => {
                self.ids.push(token);
                self.refs.push(r);
                self.ids.len() as u32
            }
        }
    }
}

impl OutStream for CaptureWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.emit(&[v])
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.emit(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.put_u32(v.len() as u32);
        self.emit(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.put_u32(0),
            RefValue::Host { token, class_id } => {
                let n = self.number(
                    *token,
                    CapturedRef {
                        grid: 0,
                        occupant: *token,
                        department: *token,
                        class_id: *class_id,
                    },
                );
                self.put_u32(n) && self.put_u32(*class_id)
            }
            RefValue::Object(o) => {
                let token = Rc::as_ptr(o) as *const () as usize as u64;
                let class_id = o.class_id();
                self.objects.insert(token, o.clone());
                let n = self.number(
                    token,
                    CapturedRef {
                        grid: token,
                        occupant: 0,
                        department: 0,
                        class_id,
                    },
                );
                self.put_u32(n) && self.put_u32(class_id)
            }
        }
    }
    fn error(&self) -> i32 {
        0
    }
}

/// The stand-in original's grids, read as the plugin reads the game's.
#[derive(Default)]
struct Live {
    objects: HashMap<u64, Rc<dyn SaveObject>>,
}

impl Live {
    fn with<R>(&self, token: u64, f: impl FnOnce(&dyn Any) -> R) -> Option<R> {
        let o = self.objects.get(&token)?.clone();
        let any = o.as_any_rc();
        Some(f(&*any))
    }
}

impl LiveGrids for Live {
    fn geometry(&self, token: u64) -> Option<TractGeometry> {
        self.with(token, |a| {
            if let Some(g) = a.downcast_ref::<SimGrid<u8>>() {
                Some(g.geometry())
            } else if let Some(g) = a.downcast_ref::<SimGrid<u32>>() {
                Some(g.geometry())
            } else {
                a.downcast_ref::<SimGrid<f32>>().map(|g| g.geometry())
            }
        })
        .flatten()
    }
    fn value(&self, token: u64, _class_id: u32, tx: i32, tz: i32) -> f64 {
        self.with(token, |a| {
            if let Some(g) = a.downcast_ref::<SimGrid<u8>>() {
                f64::from(g.get(tx, tz))
            } else if let Some(g) = a.downcast_ref::<SimGrid<u32>>() {
                f64::from(g.get(tx, tz))
            } else if let Some(g) = a.downcast_ref::<SimGrid<f32>>() {
                f64::from(g.get(tx, tz))
            } else {
                0.0
            }
        })
        .unwrap_or(0.0)
    }
}

/// The original's record as the plugin captures it, and its grids.
fn capture(s: &TrafficSimulator, live: &mut Live) -> Capture {
    let mut w = CaptureWriter::default();
    assert!(s.write(&mut w));
    for (t, o) in w.objects.drain() {
        live.objects.insert(t, o);
    }
    Capture {
        bytes: w.bytes,
        refs: w.refs,
    }
}

fn delivery(msg_type: u32, julian: i32) -> Delivery {
    Delivery {
        msg_type,
        julian,
        ..Delivery::default()
    }
}

/// One delivery to the stand-in original, wrapped by the shadow.
fn deliver(sh: &mut TrafficShadow, orig: &TrafficSimulator, live: &mut Live, msg_type: u32, julian: i32) -> i32 {
    let r0 = capture(orig, live);
    sh.before(delivery(msg_type, julian), Some(r0), live);
    orig.do_message(&Message {
        msg_type,
        data1: 0,
        data2: 0,
        data3: 0,
        object: 0,
    });
    let r1 = capture(orig, live);
    let out = sh.after(Some(r1), live);
    for l in &out.lines {
        println!("{l}");
    }
    out.mismatches
}

/// A city of ten lots holding trips to examine, the original at three units
/// per slice: the shadow's copy (one unit per slice) stops where the
/// original did, every delivery compares clean, and the month line says so.
#[test]
fn shadow_follows_its_original() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    orig.set_traffic_units_per_slice(3);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    let mut states = Vec::new();
    for day in 0..40 {
        let julian = 2_460_000 + day as i32;
        w.day.set(day % 28 + 1);
        let n = deliver(&mut sh, &orig, &mut live, msg::DAY, julian);
        assert_eq!(n, 0, "day {day}");
        states.push(orig.cycle_state());
        if day % 28 == 27 {
            assert_eq!(deliver(&mut sh, &orig, &mut live, msg::MONTH, julian), 0, "month");
        }
    }
    assert!(sh.started());
    assert_eq!(sh.stats.incomplete, 0);
    assert_eq!(sh.stats.resyncs, 0, "no drift between deliveries");
    assert!(sh.stats.trip_lists > 0 && sh.stats.grid_cells > 0);
    // The original went through the trip phases and beyond.
    assert!(states.contains(&cycle_state::FULL_TRIPS) || states.contains(&cycle_state::REROUTE_TRIPS));
    // The copy wrote nothing to the game: the stub's lists are the original's.
    assert!(sh.host().posted.borrow().is_empty());
}

/// A month delivery gives a month line.
#[test]
fn shadow_month_line() {
    let w = World::new();
    let orig = sim(&w);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000), 0);
    let r0 = capture(&orig, &mut live);
    sh.before(delivery(msg::MONTH, 2_460_001), Some(r0), &live);
    orig.do_message(&Message {
        msg_type: msg::MONTH,
        data1: 0,
        data2: 0,
        data3: 0,
        object: 0,
    });
    let out = sh.after(Some(capture(&orig, &mut live)), &live);
    assert_eq!(out.mismatches, 0);
    let line = out.lines.last().expect("a month line");
    assert!(line.starts_with("traffic shadow month to "), "{line}");
    assert!(line.contains(": clean;"), "{line}");
    assert!(line.contains("deliveries=2 (day 1, month 1, other 0)"), "{line}");
}

/// A change to the original between two deliveries is found by the record
/// check, named, and the copy resyncs.
#[test]
fn shadow_resyncs_on_drift() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000), 0);
    orig.update_state(|s| s.pass_total = s.pass_total.wrapping_add(7));
    let r0 = capture(&orig, &mut live);
    sh.before(delivery(msg::DAY, 2_460_001), Some(r0), &live);
    orig.do_message(&Message {
        msg_type: msg::DAY,
        data1: 0,
        data2: 0,
        data3: 0,
        object: 0,
    });
    let out = sh.after(Some(capture(&orig, &mut live)), &live);
    assert_eq!(sh.stats.resyncs, 1);
    assert!(out.mismatches >= 1);
    assert!(
        out.lines.iter().any(|l| l.contains("between deliveries")),
        "{:?}",
        out.lines
    );
    // After the resync the copy follows again.
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_002), 0);
}

/// The record comparison names the field a byte belongs to.
#[test]
fn compare_record_names_fields() {
    let w = World::new();
    let s = sim(&w);
    let mut ours = CanonicalWriter::new();
    assert!(s.write(&mut ours));
    assert!(ours.marks.len() > 3, "write_record marks its fields");
    assert!(compare_record(&ours.bytes, &ours).is_empty());
    let mut orig = ours.bytes.clone();
    let (name, start, _) = ours.fields()[1];
    orig[start] ^= 0x40;
    let d = compare_record(&orig, &ours);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, name);
    // A shorter record: the first differing field and the length note.
    let short = &ours.bytes[..ours.bytes.len() - 1];
    let d = compare_record(short, &ours);
    assert!(d.iter().any(|(f, _)| *f == "(record length)"));
}

/// TS59-TS61: the pass counters, the re-route context cell, the day index
/// and the statistics byte are compared exactly (a difference is a
/// mismatch); a field still unmodelled (`~`) is a known kind, by its name.
#[test]
fn record_split_knows_only_unmodelled_fields() {
    let w = World::new();
    let s = sim(&w);
    let mut ours = CanonicalWriter::new();
    assert!(s.write(&mut ours));
    let start = |name: &str| ours.fields().into_iter().find(|f| f.0 == name).unwrap().1;
    for name in [
        "pass counters",
        "re-route context cell",
        "day index",
        "statistics (published)",
    ] {
        let mut orig = ours.bytes.clone();
        orig[start(name)] ^= 1;
        let (rest, known) = compare_record_split(&orig, &ours);
        assert_eq!(rest.len(), 1, "{name}");
        assert_eq!(rest[0].0, name);
        assert!(known.is_empty(), "{name}");
    }
    let mut orig = ours.bytes.clone();
    orig[start("~lot iterator")] ^= 1;
    let (rest, known) = compare_record_split(&orig, &ours);
    assert!(rest.is_empty());
    assert_eq!(known, vec!["~lot iterator"]);
}

/// The grid class IDs are simgrid's.
#[test]
fn grid_classes() {
    assert_eq!(<u8 as crate::grid::GridElem>::CLSID, grid_class::U8);
    assert_eq!(<u32 as crate::grid::GridElem>::CLSID, grid_class::U32);
    assert_eq!(<f32 as crate::grid::GridElem>::CLSID, grid_class::F32);
}

/// The pacing does not matter: an original at 1, 2 or 7 units per slice is
/// followed as well.
#[test]
fn shadow_follows_any_pacing() {
    for units in [1, 2, 7] {
        let w = World::new();
        ten_sources(&w);
        let orig = sim(&w);
        orig.set_traffic_units_per_slice(units);
        let mut live = Live::default();
        let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
        for day in 0..30 {
            w.day.set(day % 28 + 1);
            let n = deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000 + day as i32);
            assert_eq!(n, 0, "units {units} day {day}");
        }
    }
}

/// The native safe iterator advances before yielding the last lot (zones
/// §8.3.1). A frame can append a lot while that source is still being worked;
/// the exhausted cursor must not visit it in the current phase.
#[test]
fn shadow_follows_append_after_last_source_yield() {
    for phase in [cycle_state::CONFIRM_TRIPS, cycle_state::FULL_TRIPS] {
        let w = World::new();
        ten_sources(&w);
        let orig = sim(&w);
        orig.set_traffic_units_per_slice(1);
        orig.enter_cycle_state(phase);
        orig.update_state(|s| s.shadow_lazy = true);
        let mut live = Live::default();
        let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
        let mut reached_last = false;
        for step in 0..80 {
            let r0 = capture(&orig, &mut live);
            sh.before(delivery(msg::DAY, 2_460_000), Some(r0), &live);
            orig.shadow_day_start();
            orig.shadow_slice();
            // Stand in for the native cursor independently of SourceIter:
            // the final node's next pointer was the sentinel when yielded.
            reached_last = orig.read_state(|s| s.iter.last_lot == Some(109));
            if reached_last {
                orig.update_state(|s| s.iter.lots_done = true);
            }
            let r1 = capture(&orig, &mut live);
            let out = sh.after(Some(r1), &live);
            assert_eq!(out.mismatches, 0, "phase {phase} step {step}: {:?}", out.lines);
            if reached_last {
                break;
            }
        }
        assert!(reached_last);
        assert_eq!(orig.cycle_state(), phase);
        w.add_lot(110, Lot::new(CellRect::new(4, 5, 4, 5), 610));
        w.set_trips(Source::Lot(110), vec![TripData::new(1, 0, 0, 0, status::SEARCH)]);
        // The rest of this phase ignores the new lot, including when the
        // clock retries the unfinished day before advancing to another day.
        for step in 0..8 {
            let r0 = capture(&orig, &mut live);
            sh.before(delivery(msg::DAY, 2_460_000), Some(r0), &live);
            orig.shadow_day_start();
            orig.shadow_slice();
            let r1 = capture(&orig, &mut live);
            let out = sh.after(Some(r1), &live);
            assert_eq!(out.mismatches, 0, "phase {phase} retry {step}: {:?}", out.lines);
            if orig.cycle_state() != phase {
                break;
            }
        }
        assert_ne!(orig.cycle_state(), phase);
        assert_eq!(sh.stats.resyncs, 0);
    }
}

/// A current source removed between deliveries is still owned and completed.
#[test]
fn shadow_retains_removed_current_source_until_completion() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    orig.set_traffic_units_per_slice(1);
    orig.enter_cycle_state(cycle_state::FULL_TRIPS);
    orig.update_state(|s| s.shadow_lazy = true);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    let r0 = capture(&orig, &mut live);
    sh.before(delivery(msg::DAY, 2_460_000), Some(r0), &live);
    orig.shadow_day_start();
    orig.shadow_slice();
    let out = sh.after(Some(capture(&orig, &mut live)), &live);
    assert_eq!(out.mismatches, 0, "{:?}", out.lines);
    let src = orig.read_state(|s| s.current).unwrap();
    assert_eq!(src, Source::Lot(100));
    assert_eq!(sh.copy().read_state(|s| s.current), Some(src));
    // One native reference and one independent shadow reference. Without
    // the latter the glue used to reject this token at the next delivery.
    assert_eq!(w.refs.borrow().get(&100).copied(), Some(2));
    let lot = w.lot(100).unwrap();
    w.retired_lots.borrow_mut().insert(100, lot);
    w.lots.borrow_mut().retain(|(id, _)| *id != 100);
    assert!(!w.lots().contains(&100));
    for day in 1..80 {
        assert_eq!(
            deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000 + day),
            0,
            "removed held source, day {day}"
        );
        if orig.cycle_state() != cycle_state::FULL_TRIPS {
            break;
        }
    }
    assert_ne!(orig.cycle_state(), cycle_state::FULL_TRIPS);
    assert_eq!(w.refs.borrow().get(&100).copied(), Some(0));
    assert_eq!(sh.stats.resyncs, 0);
    // Destruction also balances a still-held source on early shutdown.
    let r0 = capture(&orig, &mut live);
    sh.before(delivery(msg::DAY, 2_460_100), Some(r0), &live);
    drop(sh);
    orig.shutdown();
    assert!(w.refs.borrow().values().all(|&n| n == 0));
}

/// A trip list and a grid value the original left differently from the
/// copy are reported.
#[test]
fn shadow_reports_trips_and_grids() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000), 0);
    let r0 = capture(&orig, &mut live);
    sh.before(delivery(msg::DAY, 2_460_001), Some(r0), &live);
    orig.do_message(&Message {
        msg_type: msg::DAY,
        data1: 0,
        data2: 0,
        data3: 0,
        object: 0,
    });
    // "The original" wrote one more trip, and one more unit of traffic.
    let mut t = w.get_trips(Source::Lot(100));
    t.push(TripData::new(9, 0, 0, 0, status::SEARCH));
    w.set_trips(Source::Lot(100), t);
    let g = orig.volume_grid(0).unwrap();
    g.set(1, 1, g.get(1, 1) + 1.0);
    let out = sh.after(Some(capture(&orig, &mut live)), &live);
    assert_eq!(sh.stats.trip_mismatches, 1);
    assert_eq!(sh.stats.grid_mismatches, 1);
    assert_eq!(out.mismatches, 2, "{:?}", out.lines);
    let line = &out.lines[0];
    assert!(line.contains("trips of lot (0, 6)"), "{line}");
    assert!(line.contains("grid volume 0: 1 of"), "{line}");
}

/// The copy keeps the original's hash orders across a Read: keys sharing a
/// bucket (the damage sets, the edge densities) are written in the
/// original's order after the copy reads its record.
#[test]
fn shadow_keeps_hash_orders() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    orig.update_damage(|d| {
        for k in [5u16, 198, 391, 7] {
            d.road.insert_cell(k);
        }
    });
    orig.network().update_state(|n| {
        for k in [0x0102_0003u32, 0x0102_0003 + 193, 0x0102_0003 + 386] {
            n.edge_density.entry(k, || crate::traffic_network::EDGE_DENSITY_ZERO);
        }
    });
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000), 0);
    assert_eq!(deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_001), 0);
    assert_eq!(sh.stats.resyncs, 0);
}

/// An original stopped mid-scan by its deadline (CreateMaps resting at a
/// cell) is followed: the copy's scan stops at the same cell.
#[test]
fn shadow_follows_a_scan_stopped_mid_way() {
    let w = World::new();
    ten_sources(&w);
    let orig = sim(&w);
    let mut live = Live::default();
    let mut sh = TrafficShadow::new(w.clone(), 0, 8, 8);
    let mut day = 0u32;
    while orig.cycle_state() != cycle_state::CREATE_MAPS {
        assert!(day < 200, "the original never reached CreateMaps");
        w.day.set(day % 28 + 1);
        assert_eq!(
            deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000 + day as i32),
            0,
            "day {day}"
        );
        day += 1;
    }
    let (x, z) = orig.read_state(|s| (s.scan.x0 + 2, s.scan.z0 + 1));
    orig.update_state(|s| s.shadow_scan_stop = Some((cycle_state::CREATE_MAPS, x, z)));
    w.day.set(day % 28 + 1);
    assert_eq!(
        deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000 + day as i32),
        0,
        "the stop"
    );
    assert_eq!(orig.cycle_state(), cycle_state::CREATE_MAPS);
    assert_eq!(orig.read_state(|s| (s.scan.x, s.scan.z)), (x, z));
    orig.update_state(|s| s.shadow_scan_stop = None);
    day += 1;
    w.day.set(day % 28 + 1);
    assert_eq!(
        deliver(&mut sh, &orig, &mut live, msg::DAY, 2_460_000 + day as i32),
        0,
        "the rest"
    );
    assert_ne!(orig.cycle_state(), cycle_state::CREATE_MAPS);
    assert_eq!(sh.stats.resyncs, 0);
}
