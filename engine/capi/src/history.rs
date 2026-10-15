// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The history warehouse (`docs/specs/history.md`) over the C ABI. See the
//! "history warehouse" section of `include/fc4.h`.

use std::cell::RefCell;
use std::ffi::c_void;

use fc4_core::history::{answers_iid, HistoryHost, HistoryWarehouse, Precision};

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

#[repr(C)]
pub struct fc4_history {
    _private: [u8; 0],
}

type U = *mut c_void;

/// What the warehouse needs from the game (fc4.h: `fc4_history_host`).
/// Missing callbacks read as 0 / do nothing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_history_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub sim_date: Option<unsafe extern "C" fn(U) -> u32>,
    pub now_ms: Option<unsafe extern "C" fn(U) -> u32>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const u8, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub send: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
}

/// One point (`fc4_history_get_series`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_history_point {
    pub value: i32,
    pub date: u32,
}

/// Diagnostics (`fc4_history_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_history_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub has_cursor: u8,
    pub _pad: [u8; 2],
    pub pass_state: u32,
    pub cursor: u32,
    pub pass_counter: u32,
    pub series_count: u32,
    pub total_points: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_history_point>() == 8);
    assert!(std::mem::size_of::<fc4_history_state>() == 28);
    assert!(std::mem::size_of::<fc4_history_host>() == if cfg!(target_pointer_width = "32") { 36 } else { 72 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost(fc4_history_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

impl HistoryHost for CHost {
    fn sim_date(&self) -> u32 {
        cb!(self, sim_date, 0)
    }
    fn now_ms(&self) -> u32 {
        cb!(self, now_ms, 0)
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(self, add_agent, (), level, name.as_ptr(), name.len(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn send(&self, msg_type: u32, data1: u32, data2: u32) {
        cb!(self, send, (), msg_type, data1, data2)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// The warehouse and a copy of the host table: `RecordHistoryData` sends
/// through the table without borrowing the warehouse, so the host may
/// deliver the message back to `fc4_history_do_message` synchronously.
struct Handle {
    core: RefCell<HistoryWarehouse>,
    host: fc4_history_host,
}

/// SAFETY: `h` must be NULL or a live pointer from `fc4_history_create`.
unsafe fn handle<'a>(h: *const fc4_history) -> Option<&'a Handle> {
    (h as *const Handle).as_ref()
}

/// Runs `f` on the warehouse; the fallback for NULL, a re-entrant call or a
/// panic.
fn with_mut<R>(h: *const fc4_history, fallback: R, f: impl FnOnce(&mut HistoryWarehouse) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_history*.
    let Some(h) = (unsafe { handle(h) }) else {
        return fallback;
    };
    let Ok(mut w) = h.core.try_borrow_mut() else {
        return fallback;
    };
    guard(fallback, || f(&mut w))
}

/// As `with_mut`, read-only (allowed while another read is running).
fn with_ref<R>(h: *const fc4_history, fallback: R, f: impl FnOnce(&HistoryWarehouse) -> R) -> R {
    // SAFETY: as `with_mut`.
    let Some(h) = (unsafe { handle(h) }) else {
        return fallback;
    };
    let Ok(w) = h.core.try_borrow() else {
        return fallback;
    };
    guard(fallback, || f(&w))
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

/// Creates the warehouse (§2: no series, not initialised). The host table is
/// copied. NULL if the table is missing or too small.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_create(host: *const fc4_history_host) -> *mut fc4_history {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_history_host>() => {
            let w = HistoryWarehouse::new(Box::new(CHost(*h)));
            Box::into_raw(Box::new(Handle {
                core: RefCell::new(w),
                host: *h,
            })) as *mut fc4_history
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_history_destroy(h: *mut fc4_history) {
    if !h.is_null() {
        guard((), || drop(Box::from_raw(h as *mut Handle)));
    }
}

/// `QueryInterface` (§1): true for `cIGZUnknown`, `cISC4HistoryWarehouse` and
/// `cIGZSerializable`; false for everything else, `cIGZMessageTarget2` too.
#[no_mangle]
pub extern "C" fn fc4_history_answers_iid(iid: u32) -> bool {
    answers_iid(iid)
}

/// `Init` (slot 3, §3). True for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_init(h: *mut fc4_history) -> bool {
    with_mut(h, false, HistoryWarehouse::init)
}

/// `Shutdown` (slot 4, §3); the series stay.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_shutdown(h: *mut fc4_history) -> bool {
    with_mut(h, false, HistoryWarehouse::shutdown)
}

/// `DoMessage` (§3): record (data1 = key, data2 = value), set queue length,
/// SimNewDay (the pass step: false asks for another call this interval),
/// SimNewYear; true for everything else (and for a re-entrant call).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_do_message(h: *mut fc4_history, msg_type: u32, data1: u32, data2: u32) -> bool {
    with_mut(h, true, |w| w.do_message(msg_type, data1, data2))
}

/// `RecordHistoryData` (slot 12, §4.1): sends the record message through the
/// host's `send` (the warehouse stores it when the host delivers it to
/// `fc4_history_do_message`). Does not borrow the warehouse. Returns false
/// (HW12, the original's final `Release` leaves 0).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_record(h: *const fc4_history, key: u32, value: i32) -> bool {
    let Some(hd) = handle(h) else { return false };
    let host = CHost(hd.host);
    guard(false, || HistoryWarehouse::record_history_data(&host, key, value))
}

/// `SetHistoryDataQueueLength` (slot 13, §4.2). True for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_set_queue_length(h: *mut fc4_history, key: u32, n: u32) -> bool {
    with_mut(h, false, |w| w.set_history_data_queue_length(key, n))
}

/// `Coalesce(key, from, to, granularity)` (slot 9, §4.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_coalesce(
    h: *mut fc4_history,
    key: u32,
    from: u32,
    to: u32,
    granularity: i32,
) -> bool {
    with_mut(h, false, |w| w.coalesce(key, from, to, granularity))
}

/// `Dispose(key)` (slot 10, §4.5).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_dispose(h: *mut fc4_history, key: u32) -> bool {
    with_mut(h, false, |w| w.dispose(key))
}

/// `Dispose(key, from, to)` (slot 11, §4.5).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_dispose_range(h: *mut fc4_history, key: u32, from: u32, to: u32) -> bool {
    with_mut(h, false, |w| w.dispose_range(key, from, to))
}

/// `GetCount` (slot 5, §4.6). `*out` is untouched on false.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_count(
    h: *const fc4_history,
    key: u32,
    from: u32,
    to: u32,
    out: *mut f32,
) -> bool {
    with_ref(h, false, |w| match w.get_count(key, from, to) {
        Some(v) => {
            put(out, v);
            true
        }
        None => false,
    })
}

/// `GetTrend` (slot 6, §4.7) at the caller's x87 precision (`precision_bits`
/// 24, or anything else for 53). Outputs untouched on false.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_trend(
    h: *const fc4_history,
    key: u32,
    from: u32,
    to: u32,
    precision_bits: u32,
    slope: *mut f32,
    intercept: *mut f32,
) -> bool {
    let p = Precision::from_bits(precision_bits);
    with_ref(h, false, |w| match w.get_trend(key, from, to, p) {
        Some((s, i)) => {
            put(slope, s);
            put(intercept, i);
            true
        }
        None => false,
    })
}

/// `GetPoint` (slot 8, §4.7). `*out` untouched on false.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_point(
    h: *const fc4_history,
    key: u32,
    date: u32,
    precision_bits: u32,
    out: *mut f32,
) -> bool {
    let p = Precision::from_bits(precision_bits);
    with_ref(h, false, |w| match w.get_point(key, date, p) {
        Some(v) => {
            put(out, v);
            true
        }
        None => false,
    })
}

/// `GetMinMaxAverage` (slot 7, §4.8). Outputs untouched on false.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_min_max_average(
    h: *const fc4_history,
    key: u32,
    from: u32,
    to: u32,
    precision_bits: u32,
    min: *mut f32,
    max: *mut f32,
    avg: *mut f32,
) -> bool {
    let p = Precision::from_bits(precision_bits);
    with_ref(h, false, |w| match w.get_min_max_average(key, from, to, p) {
        Some((a, b, c)) => {
            put(min, a);
            put(max, b);
            put(avg, c);
            true
        }
        None => false,
    })
}

/// Diagnostics; false for NULL or an in-use object.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_state(h: *const fc4_history, out: *mut fc4_history_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_ref(h, false, |w| {
        *o = fc4_history_state {
            struct_size: std::mem::size_of::<fc4_history_state>() as u32,
            initialized: w.is_initialized().into(),
            has_cursor: w.cursor().is_some().into(),
            _pad: [0; 2],
            pass_state: w.pass_state(),
            cursor: w.cursor().unwrap_or(0),
            pass_counter: w.pass_counter(),
            series_count: w.series_count() as u32,
            total_points: w.total_points() as u32,
        };
        true
    })
}

/// The keys in ascending order: copies at most `capacity` (out may be NULL)
/// and returns the key count.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_keys(h: *const fc4_history, out: *mut u32, capacity: usize) -> usize {
    with_ref(h, 0, |w| {
        let keys = w.keys();
        if !out.is_null() {
            for (i, k) in keys.iter().take(capacity).enumerate() {
                *out.add(i) = *k;
            }
        }
        keys.len()
    })
}

/// One series: its queue length and its points in list order (at most
/// `capacity` copied; `out` may be NULL), `*count` = the point count. False
/// for a missing key (outputs untouched).
#[no_mangle]
pub unsafe extern "C" fn fc4_history_get_series(
    h: *const fc4_history,
    key: u32,
    max_count: *mut u32,
    out: *mut fc4_history_point,
    capacity: usize,
    count: *mut usize,
) -> bool {
    with_ref(h, false, |w| {
        let Some(s) = w.series(key) else { return false };
        put(max_count, s.max_count);
        put(count, s.points.len());
        if !out.is_null() {
            for (i, p) in s.points.iter().take(capacity).enumerate() {
                *out.add(i) = fc4_history_point {
                    value: p.value,
                    date: p.date,
                };
            }
        }
        true
    })
}

/// `Write` (§7): version 2.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_write(h: *const fc4_history, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_ref(h, false, |w| w.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<std::rc::Rc<dyn fc4_core::stream::SaveObject>> {
    None
}

/// `Read` (§7): versions 0 to 2; the series are added to the existing ones.
#[no_mangle]
pub unsafe extern "C" fn fc4_history_read(h: *mut fc4_history, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_mut(h, false, |w| {
        w.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
