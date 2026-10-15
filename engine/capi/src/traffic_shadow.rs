// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic shadow (fc4.h "traffic shadow"): tooling, no spec behaviour.
//! With the ORIGINAL traffic simulator running, each delivery its agents get
//! (day, month, the one-shot) is wrapped:
//!
//! 1. [`fc4_traffic_shadow_before`]: the original's §6 record captured by the
//!    plugin (its own `Write`, the capture format of
//!    [`fc4_pollution_capture`]); the copy (FreeCity4's simulator, private to
//!    the shadow) starts on it at the first delivery, and resyncs on it when
//!    something between two deliveries changed one side only; the inputs the
//!    original's delivery may change are snapshotted;
//! 2. the original's delivery, unchanged;
//! 3. [`fc4_traffic_shadow_after`]: the record again; the copy runs to where
//!    the original stopped and its record, trip lists, holder properties and
//!    grids are compared bitwise with the original's (core
//!    `traffic_simulator::shadow`).
//!
//! The copy reads the game through the host table (read callbacks only: the
//! shadow keeps every write, post, budget call and reference to itself) and
//! the original's grids through the grid table's `geometry` and `get` only;
//! it never creates, writes, shuts down or releases a game grid.

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::TractGeometry;
use fc4_core::traffic_simulator::shadow::{Capture, CapturedRef, Delivery, LiveGrids, TrafficShadow};
use fc4_core::traffic_simulator::{Message, TrafficSimHost};

use crate::guard;
use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::pollution_shadow::{fc4_pollution_capture, fc4_pollution_capture_ref};
use crate::stream::fc4_bytes_sink;
use crate::traffic::{fc4_traffic_host, host_ok, CHost};

/// Opaque handle (fc4.h `fc4_traffic_shadow`).
#[repr(C)]
pub struct fc4_traffic_shadow {
    _private: [u8; 0],
}

/// Totals since creation (fc4.h `fc4_traffic_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_traffic_shadow_stats {
    pub struct_size: u32,
    /// Deliveries finished (compared or not).
    pub deliveries: u32,
    /// ... compared.
    pub compared: u32,
    /// ... compared without a difference.
    pub clean: u32,
    /// ... not compared (a capture failed, the copy could not read it).
    pub incomplete: u32,
    /// Deliveries whose record before differed from the copy's (resynced).
    pub resyncs: u32,
    /// The last delivery's difference count (-1: not compared).
    pub last_mismatches: i32,
    pub reserved: u32,
    /// Record fields that differed.
    pub record_fields: u64,
    pub trip_lists: u64,
    pub trip_mismatches: u64,
    pub props: u64,
    pub prop_mismatches: u64,
    pub grid_cells: u64,
    pub grid_mismatches: u64,
    /// Units (slices) the copy ran.
    pub units: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_traffic_shadow_stats>() == 96);
};

/// The original's grids through the grid table (read only).
struct Live {
    api: fc4_grid_api,
}

impl LiveGrids for Live {
    fn geometry(&self, token: u64) -> Option<TractGeometry> {
        let f = self.api.geometry?;
        if token == 0 {
            return None;
        }
        let mut g = fc4_grid_geometry::default();
        // SAFETY: valid callback (create's precondition); `g` outlives the call.
        unsafe { f(self.api.user, token, &mut g) };
        Some(TractGeometry {
            tract_size: g.tract_size,
            shift: g.shift,
            count_x: g.count_x,
            count_z: g.count_z,
            width_x: g.width_x,
            width_z: g.width_z,
            inv_width_x: g.inv_width_x,
            inv_width_z: g.inv_width_z,
        })
    }
    fn value(&self, token: u64, class_id: u32, tx: i32, tz: i32) -> f64 {
        match self.api.get {
            // SAFETY: valid callback on a live game grid.
            Some(f) => unsafe { f(self.api.user, token, class_id, tx, tz) },
            None => 0.0,
        }
    }
}

struct Shadow {
    core: TrafficShadow,
    live: Live,
}

fn with_shadow<R>(s: *mut fc4_traffic_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_traffic_shadow*.
    match unsafe { (s as *mut Shadow).as_mut() } {
        Some(sh) => guard(fallback, || f(sh)),
        None => fallback,
    }
}

/// A capture as the core's (`None`: no capture, or the original's `Write`
/// failed).
unsafe fn capture_of(c: *const fc4_pollution_capture) -> Option<Capture> {
    let c = c.as_ref()?;
    if (c.struct_size as usize) < std::mem::size_of::<fc4_pollution_capture>() || c.ok == 0 {
        return None;
    }
    if c.bytes.is_null() || c.len == 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts(c.bytes, c.len).to_vec();
    let refs: &[fc4_pollution_capture_ref] = if c.refs.is_null() || c.ref_count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(c.refs, c.ref_count)
    };
    let refs = refs
        .iter()
        .map(|r| CapturedRef {
            grid: r.grid,
            occupant: r.occupant,
            department: r.department,
            class_id: r.class_id,
        })
        .collect();
    Some(Capture { bytes, refs })
}

unsafe fn emit(lines: &[String], sink: fc4_bytes_sink, ctx: *mut c_void) {
    if let Some(f) = sink {
        for l in lines {
            f(ctx, l.as_ptr(), l.len());
        }
    }
}

/// Creates a shadow over a host table. The shadow calls only the table's
/// read callbacks (its writes, posts, budget and reference calls are kept
/// inside it; they may be NULL) and the grid table's `geometry` and `get`.
/// `session_seed` seeds the copy's generator (its draws are not the
/// original's: §4.15 draws are not compared). `max_diffs`: differences named
/// per line (0 → 8, at most 64); `detail_lines`: delivery lines per month
/// (the month line counts every delivery; 0 → 8). NULL when the table is
/// incomplete.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_create(
    host: *const fc4_traffic_host,
    session_seed: u32,
    max_diffs: u32,
    detail_lines: u32,
) -> *mut fc4_traffic_shadow {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        let api = h.grids;
        let inner: Rc<dyn TrafficSimHost> = Rc::new(CHost::new(h));
        let max_diffs = match max_diffs {
            0 => 8,
            n => n.min(64) as usize,
        };
        let detail_lines = if detail_lines == 0 { 8 } else { detail_lines };
        Box::into_raw(Box::new(Shadow {
            core: TrafficShadow::new(inner, session_seed, max_diffs, detail_lines),
            live: Live { api },
        })) as *mut fc4_traffic_shadow
    })
}

/// Destroys a shadow (NULL is ignored). Nothing is released to the game: the
/// copy holds no game reference.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_destroy(s: *mut fc4_traffic_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

/// Before the original's delivery of `msg_type` (its data1..3; `julian` the
/// game date for the lines) with its record `before` (NULL or not ok: the
/// delivery is reported INCOMPLETE by the after call). Returns 0, or -1 for
/// a NULL shadow.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_before(
    s: *mut fc4_traffic_shadow,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    julian: i32,
    before: *const fc4_pollution_capture,
) -> i32 {
    let c = capture_of(before);
    with_shadow(s, -1, |sh| {
        let d = Delivery {
            msg_type,
            data1,
            data2,
            data3,
            julian,
        };
        sh.core.before(d, c, &sh.live);
        0
    })
}

/// After the original's delivery, with its record `after`: the copy's run
/// and the comparison. The log lines go to `sink`, one call per line: a line
/// for a delivery with a difference or not compared (at most `detail_lines`
/// a month) and, after a month delivery, the month line. Returns the
/// delivery's difference count, -1 when nothing was compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_after(
    s: *mut fc4_traffic_shadow,
    after: *const fc4_pollution_capture,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let c = capture_of(after);
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| {
        let o = sh.core.after(c, &sh.live);
        lines = o.lines;
        o.mismatches
    });
    emit(&lines, sink, ctx);
    r
}

/// A message the original receives outside its agent deliveries (§3.1: the
/// occupant, lot, neighbour, budget, tuning and ordinance messages), with
/// its object (the occupant or lot token; borrowed for the call): the copy
/// handles it as the original does. Its writes are not compared. False for
/// a NULL shadow or before the copy started.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_message(
    s: *mut fc4_traffic_shadow,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    object: u64,
) -> bool {
    with_shadow(s, false, |sh| {
        if !sh.core.started() {
            return false;
        }
        sh.core.message(&Message {
            msg_type,
            data1,
            data2,
            data3,
            object,
        });
        true
    })
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shadow_get_stats(
    s: *mut fc4_traffic_shadow,
    out: *mut fc4_traffic_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        let t = sh.core.stats;
        *o = fc4_traffic_shadow_stats {
            struct_size: std::mem::size_of::<fc4_traffic_shadow_stats>() as u32,
            deliveries: t.deliveries,
            compared: t.compared,
            clean: t.clean,
            incomplete: t.incomplete,
            resyncs: t.resyncs,
            last_mismatches: t.last_mismatches,
            reserved: 0,
            record_fields: t.record_fields,
            trip_lists: t.trip_lists,
            trip_mismatches: t.trip_mismatches,
            props: t.props,
            prop_mismatches: t.prop_mismatches,
            grid_cells: t.grid_cells,
            grid_mismatches: t.grid_mismatches,
            units: t.units,
        };
        true
    })
}
