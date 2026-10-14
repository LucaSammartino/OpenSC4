// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill shadow (fc4.h "landfill shadow"): tooling, no spec
//! behaviour. With the ORIGINAL landfill developer running, each of its
//! monthly passes (the SimNewMonth its agent receives) is wrapped:
//!
//! 1. before: the plugin captures the original's §6.1 record (its own
//!    `Write` into a capture stream; each zot reference already queried for
//!    `cISC4Occupant`) and reads its slot 7 `GetMaintenanceCosts`;
//!    [`fc4_landfill_shadow_begin`] loads the record into a private core
//!    developer, initialises it in shadow mode (no game call that changes
//!    anything; no reference taken), delivers PostCityInit (the department,
//!    zone `Init`), restores the original's `maintenance` (not saved, §2.1;
//!    §4.7.1 step 3 reads it), remembers the garbage supply and the
//!    active-landfill flag of every cell of every zone rectangle, and runs
//!    FreeCity4's month (§4.7.1) on those inputs. Every write stays inside
//!    the shadow (`landfill.rs` shadow mode): the supply changes, the free
//!    capacity, the active-landfill flags, the landfill put, the line-item
//!    expenses, the tiles drawn and removed, the zot placements;
//! 2. the original's delivery, unchanged;
//! 3. after: the plugin captures the record again and reads slot 7 again;
//!    [`fc4_landfill_shadow_finish`] compares the zones (rectangle,
//!    connected, every heap's valid / active / capacity / contained), the
//!    free capacity we reported with pollution's `GetGarbageCapacity`, the
//!    garbage we moved with the live supply's change, our `maintenance` with
//!    the original's, and the active-landfill flags we would have left with
//!    the live ones. One line per month.
//!
//! Not readable, hence not compared: the C `rand()` state (process-wide in
//! the original; it only picks the orientation of interior tiles), where a
//! zot is placed (the zone record keeps only the reference), the history
//! values sent, and the effects.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::julian::civil_from_julian;
use fc4_core::landfill::{msg, LandfillDeveloper, LandfillMessage, IID_OCCUPANT};
use fc4_core::stream::{ByteReader, InStream, RefValue};

use crate::guard;
use crate::landfill::{fc4_landfill_host, host_ok, CHost, Rec};
use crate::pollution_shadow::{fc4_pollution_capture, fc4_pollution_capture_ref};
use crate::stream::fc4_bytes_sink;

/// Totals since creation (fc4.h `fc4_landfill_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_landfill_shadow_stats {
    pub struct_size: u32,
    /// Months finished (compared or not).
    pub months: u32,
    /// ... compared without a difference.
    pub clean: u32,
    /// ... not compared (a capture failed, our run failed).
    pub incomplete: u32,
    /// The last month's difference count (-1: not compared).
    pub last_mismatches: i32,
    pub reserved: u32,
    pub zones_compared: u64,
    pub heaps_compared: u64,
    pub heap_mismatches: u64,
    pub cells_compared: u64,
    pub cell_mismatches: u64,
    /// Zone-count, zone, capacity, garbage-moved and maintenance differences.
    pub field_mismatches: u64,
}

const _: () = assert!(std::mem::size_of::<fc4_landfill_shadow_stats>() == 72);

// ---- the captured record ---------------------------------------------------------------------------

/// A captured record as a core input stream: a reference read with
/// `cISC4Occupant` is answered with that (borrowed) token.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_pollution_capture_ref],
}

impl InStream for CaptureReader<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.r.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.r.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.r.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.r.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.r.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.r.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.r.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.r.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.get_ref_as(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let id = self.r.get_u32()?;
        if id == 0 {
            return Some(RefValue::Null);
        }
        let class_id = self.r.get_u32()?;
        let Some(e) = self.refs.get(id as usize - 1) else {
            self.r.set_error();
            return None;
        };
        let token = if iid == IID_OCCUPANT { e.occupant } else { 0 };
        if token == 0 {
            // An object without the interface reads as null.
            return Some(RefValue::Null);
        }
        Some(RefValue::Host { token, class_id })
    }
    fn skip(&mut self, n: usize) -> bool {
        self.r.skip(n)
    }
    fn error(&self) -> i32 {
        self.r.error()
    }
    fn set_error(&mut self) {
        self.r.set_error();
    }
}

/// A capture copied out of the caller's buffers.
struct Capture {
    bytes: Vec<u8>,
    refs: Vec<fc4_pollution_capture_ref>,
}

/// Copies a capture; `Err` with the reason when it is not usable.
unsafe fn capture_of(c: *const fc4_pollution_capture) -> Result<Capture, &'static str> {
    let Some(c) = c.as_ref() else {
        return Err("no capture");
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_pollution_capture>() {
        return Err("capture struct too small");
    }
    if c.ok == 0 {
        return Err("the original's Write failed");
    }
    if c.bytes.is_null() || c.len == 0 {
        return Err("empty record");
    }
    let bytes = std::slice::from_raw_parts(c.bytes, c.len).to_vec();
    let refs = if c.refs.is_null() || c.ref_count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(c.refs, c.ref_count).to_vec()
    };
    Ok(Capture { bytes, refs })
}

/// A developer loaded from a capture (never initialised).
fn load(host: &Rc<CHost>, c: &Capture) -> Result<Rc<LandfillDeveloper>, &'static str> {
    let dev = LandfillDeveloper::new(host.clone(), 0);
    let mut r = CaptureReader {
        r: ByteReader::new(&c.bytes),
        refs: &c.refs,
    };
    if !dev.read(&mut r) {
        return Err("record version not 3");
    }
    if r.error() != 0 {
        return Err("record truncated");
    }
    Ok(dev)
}

// ---- the shadow -------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_landfill_shadow`).
#[repr(C)]
pub struct fc4_landfill_shadow {
    _private: [u8; 0],
}

/// Our run of one month.
struct Run {
    dev: Rc<LandfillDeveloper>,
    supply_before: u32,
    /// The active-landfill flag of every cell of every zone rectangle before
    /// the original's month (`None`: not readable).
    pre_active: BTreeMap<(i32, i32), Option<bool>>,
    maintenance_before: i64,
    /// `UpdateBudgetRequirements` on the original's tiles before the month
    /// (informational: the original's own value is stale after a terrain
    /// change, §4.5.1).
    maintenance_recomputed: i64,
}

struct Pending {
    julian: i32,
    run: Result<Run, String>,
}

struct Shadow {
    host: Rc<CHost>,
    max_diffs: usize,
    pending: Option<Pending>,
    stats: fc4_landfill_shadow_stats,
}

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

fn show<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| "n/a".to_string(), |v| v.to_string())
}

/// The month's comparison.
#[derive(Default)]
struct Diff {
    mismatches: u32,
    first: Vec<String>,
    max_diffs: usize,
}

impl Diff {
    fn note(&mut self, what: String) {
        self.mismatches += 1;
        if self.first.len() < self.max_diffs {
            self.first.push(what);
        }
    }
}

impl Shadow {
    fn begin(&mut self, julian: i32, before: Result<Capture, &'static str>, maintenance: i64) -> i32 {
        let run = before
            .map_err(|e| format!("before: {e}"))
            .and_then(|c| self.run(&c, maintenance));
        let r = if run.is_ok() { 0 } else { -1 };
        self.pending = Some(Pending { julian, run });
        r
    }

    fn run(&self, before: &Capture, maintenance: i64) -> Result<Run, String> {
        let host = &self.host;
        host.reset();
        let dev = load(host, before).map_err(|e| format!("before: {e}"))?;
        dev.init();
        dev.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
        let maintenance_recomputed = dev.get_maintenance_costs();
        dev.set_maintenance(maintenance);
        host.reset();
        let mut pre_active = BTreeMap::new();
        for z in dev.zone_list() {
            let r = z.rect();
            for x in r.x1..=r.x2 {
                for zz in r.z1..=r.z2 {
                    pre_active.entry((x, zz)).or_insert_with(|| host.live_active(x, zz));
                }
            }
        }
        let supply_before = host.live_supply();
        dev.do_message(&LandfillMessage::of_type(msg::SIM_NEW_MONTH));
        Ok(Run {
            dev,
            supply_before,
            pre_active,
            maintenance_before: maintenance,
            maintenance_recomputed,
        })
    }

    fn finish(&mut self, after: Result<Capture, &'static str>, maintenance: i64, lines: &mut Vec<String>) -> i32 {
        self.stats.months += 1;
        let Some(p) = self.pending.take() else {
            self.stats.incomplete += 1;
            self.stats.last_mismatches = -1;
            lines.push("landfill shadow: INCOMPLETE (finish without begin); not compared".to_string());
            return -1;
        };
        let prefix = format!("landfill shadow month to {}", date(p.julian));
        let result = p.run.and_then(|run| {
            let after = after.map_err(|e| format!("after: {e}"))?;
            let theirs = load(&self.host, &after).map_err(|e| format!("after: {e}"))?;
            Ok(self.compare(&run, &theirs, maintenance))
        });
        match result {
            Ok((diff, line)) => {
                self.stats.last_mismatches = diff.mismatches as i32;
                if diff.mismatches == 0 {
                    self.stats.clean += 1;
                }
                lines.push(format!("{prefix}: {line}"));
                diff.mismatches as i32
            }
            Err(e) => {
                self.stats.incomplete += 1;
                self.stats.last_mismatches = -1;
                lines.push(format!("{prefix}: INCOMPLETE ({e}); not compared"));
                -1
            }
        }
    }

    fn compare(&mut self, run: &Run, theirs: &LandfillDeveloper, maintenance_after: i64) -> (Diff, String) {
        let host = &self.host;
        let rec: Rec = host.rec();
        let mut d = Diff {
            max_diffs: self.max_diffs,
            ..Default::default()
        };
        let mut field = 0u64;

        // Zones and heaps.
        let ours = run.dev.zone_list();
        let orig = theirs.zone_list();
        if ours.len() != orig.len() {
            field += 1;
            d.note(format!("zones {}/{}", orig.len(), ours.len()));
        }
        let (mut heaps, mut heap_bad, mut max_contained) = (0u64, 0u64, 0u64);
        for (i, (o, b)) in orig.iter().zip(ours.iter()).enumerate() {
            self.stats.zones_compared += 1;
            if o.rect() != b.rect() {
                field += 1;
                let (r, s) = (o.rect(), b.rect());
                d.note(format!(
                    "zone {i} rect ({},{})-({},{})/({},{})-({},{})",
                    r.x1, r.z1, r.x2, r.z2, s.x1, s.z1, s.x2, s.z2
                ));
            }
            if o.connected() != b.connected() {
                field += 1;
                d.note(format!("zone {i} connected {}/{}", o.connected(), b.connected()));
            }
            let (oh, bh) = (o.heaps(), b.heaps());
            if oh.len() != bh.len() {
                field += 1;
                d.note(format!("zone {i} heaps {}/{}", oh.len(), bh.len()));
            }
            for (j, (x, y)) in oh.iter().zip(bh.iter()).enumerate() {
                heaps += 1;
                let dc = (i64::from(x.contained) - i64::from(y.contained)).unsigned_abs();
                max_contained = max_contained.max(dc);
                if x != y {
                    heap_bad += 1;
                    d.note(format!(
                        "zone {i} heap {j} ({},{}) valid {}/{} active {}/{} capacity {}/{} contained {}/{}",
                        x.hx,
                        x.hz,
                        u8::from(x.valid),
                        u8::from(y.valid),
                        u8::from(x.active),
                        u8::from(y.active),
                        x.capacity,
                        y.capacity,
                        x.contained,
                        y.contained
                    ));
                }
            }
        }
        self.stats.heaps_compared += heaps;
        self.stats.heap_mismatches += heap_bad;

        // The free capacity reported to pollution (§4.7.1 step 5).
        let live_capacity = host.live_capacity();
        if let (Some(a), Some(b)) = (live_capacity, rec.capacity) {
            if a != b {
                field += 1;
                d.note(format!("free capacity {a}/{b}"));
            }
        }
        // The garbage moved out of pollution's supply.
        let supply_after = host.live_supply();
        let moved_orig = i64::from(run.supply_before) - i64::from(supply_after);
        let moved_ours = -rec.supply_delta;
        if moved_orig != moved_ours {
            field += 1;
            d.note(format!("garbage moved {moved_orig}/{moved_ours}"));
        }
        // The maintenance booked (§4.9.3).
        let maint_ours = run.dev.get_maintenance_costs();
        if maint_ours != maintenance_after {
            field += 1;
            d.note(format!("maintenance {maintenance_after}/{maint_ours}"));
        }
        // The active-landfill flags (§4.8.2).
        let cells: BTreeSet<(i32, i32)> = run.pre_active.keys().chain(rec.active.keys()).copied().collect();
        let (mut compared, mut cell_bad, mut unreadable) = (0u64, 0u64, 0u64);
        for (x, z) in cells {
            let expected = rec
                .active
                .get(&(x, z))
                .copied()
                .or_else(|| run.pre_active.get(&(x, z)).copied().flatten());
            let live = host.live_active(x, z);
            match (live, expected) {
                (Some(l), Some(e)) => {
                    compared += 1;
                    if l != e {
                        cell_bad += 1;
                        d.note(format!("active ({x},{z}) {}/{}", u8::from(l), u8::from(e)));
                    }
                }
                _ => unreadable += 1,
            }
        }
        self.stats.cells_compared += compared;
        self.stats.cell_mismatches += cell_bad;
        self.stats.field_mismatches += field;

        let status = if d.mismatches == 0 {
            "clean".to_string()
        } else {
            format!("mismatches={}", d.mismatches)
        };
        let first = if d.first.is_empty() {
            "none".to_string()
        } else {
            d.first.join(", ")
        };
        let mut line = format!(
            "{status}; zones orig/ours {}/{}; heaps compared={heaps} mismatched={heap_bad} max|d contained|={max_contained}; \
             free capacity orig/ours {}/{}; garbage moved orig/ours {moved_orig}/{moved_ours} (supply {} -> {}); \
             maintenance orig/ours {maintenance_after}/{maint_ours} (before {}, recomputed {}); active cells \
             compared={compared} mismatched={cell_bad}; ours: put in landfill {} in {} calls, tiles drawn {} removed {}, \
             zots placed {} removed {} (not placed: shadow), history values {}; first differences: {first}",
            orig.len(),
            ours.len(),
            show(live_capacity),
            show(rec.capacity),
            run.supply_before,
            supply_after,
            run.maintenance_before,
            run.maintenance_recomputed,
            rec.put_in_landfill,
            rec.put_calls,
            rec.tiles_created,
            rec.tiles_removed,
            rec.zots_added,
            rec.zots_removed,
            rec.history.len(),
        );
        if unreadable > 0 {
            line.push_str(&format!("; {unreadable} active-landfill cells not readable"));
        }
        if live_capacity.is_none() {
            line.push_str("; free capacity not readable");
        }
        (d, line)
    }
}

fn with_shadow<R>(s: *mut fc4_landfill_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_landfill_shadow*.
    match unsafe { (s as *mut Shadow).as_mut() } {
        Some(sh) => guard(fallback, || f(sh)),
        None => fallback,
    }
}

unsafe fn emit(lines: &[String], sink: fc4_bytes_sink, ctx: *mut c_void) {
    if let Some(f) = sink {
        for l in lines {
            f(ctx, l.as_ptr(), l.len());
        }
    }
}

/// Creates a shadow over a host table. The shadow only calls the table's
/// read callbacks (tuning and tile exemplars, city, terrain, pollution
/// reads including `garbage_capacity` and `is_active_landfill`, the
/// department and its line item, occupant reads, zone reads, traffic, the
/// zot orientation and the view level); the write callbacks may be NULL.
/// `max_diffs`: the differences listed per month (0 → 8, at most 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shadow_create(
    host: *const fc4_landfill_host,
    max_diffs: u32,
) -> *mut fc4_landfill_shadow {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        let max_diffs = match max_diffs {
            0 => 8,
            n => n.min(64) as usize,
        };
        Box::into_raw(Box::new(Shadow {
            host: Rc::new(CHost::new(h, true)),
            max_diffs,
            pending: None,
            stats: fc4_landfill_shadow_stats {
                struct_size: std::mem::size_of::<fc4_landfill_shadow_stats>() as u32,
                last_mismatches: -1,
                ..Default::default()
            },
        })) as *mut fc4_landfill_shadow
    })
}

/// Destroys a shadow (NULL is ignored).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shadow_destroy(s: *mut fc4_landfill_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

/// Before the original's SimNewMonth (`julian` = the message's data1):
/// loads `before` (the original's record), restores `maintenance` (its slot
/// 7) and runs ours. 0 = ran; -1 = not run (the reason is reported by the
/// finish).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shadow_begin(
    s: *mut fc4_landfill_shadow,
    julian: i32,
    before: *const fc4_pollution_capture,
    maintenance: i64,
) -> i32 {
    let c = capture_of(before);
    with_shadow(s, -1, |sh| sh.begin(julian, c, maintenance))
}

/// After the original's SimNewMonth: compares (see the module notes) with
/// `after` (its record) and `maintenance` (its slot 7), and hands the month
/// line to `sink`. Returns the difference count, -1 when not compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shadow_finish(
    s: *mut fc4_landfill_shadow,
    after: *const fc4_pollution_capture,
    maintenance: i64,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let c = capture_of(after);
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.finish(c, maintenance, &mut lines));
    emit(&lines, sink, ctx);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shadow_get_stats(
    s: *mut fc4_landfill_shadow,
    out: *mut fc4_landfill_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}
