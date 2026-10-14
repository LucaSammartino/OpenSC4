// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The 24-hour clock (`docs/specs/clock24.md`) over the C ABI. See the
//! "24-hour clock" section of `include/fc4.h`.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::clock24::{answers_iid, Clock24, ClockHost, ClockTuning};

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

#[repr(C)]
pub struct fc4_clock24 {
    _private: [u8; 0],
}

type U = *mut c_void;

/// What the clock needs from the game (fc4.h: `fc4_clock24_host`). Missing
/// callbacks read as 0 / false / do nothing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_clock24_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub now_micros: Option<unsafe extern "C" fn(U) -> u64>,
    pub post: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
    pub add_system_service: Option<unsafe extern "C" fn(U)>,
    pub remove_system_service: Option<unsafe extern "C" fn(U)>,
    pub add_to_on_idle: Option<unsafe extern "C" fn(U)>,
    pub remove_from_on_idle: Option<unsafe extern "C" fn(U)>,
    pub has_cheat_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub register_cheat: Option<unsafe extern "C" fn(U, u32, *const u8, usize)>,
    pub add_cheat_notification: Option<unsafe extern "C" fn(U, u32)>,
    pub remove_cheat_notification: Option<unsafe extern "C" fn(U, u32)>,
    pub unregister_cheat: Option<unsafe extern "C" fn(U, u32)>,
}

/// The "Main Simulator" exemplar's clock properties (§6); a `has_` flag of 0
/// is a missing property.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_clock24_tuning {
    pub struct_size: u32,
    pub has_magnification: u8,
    pub has_night_start: u8,
    pub has_night_end: u8,
    pub _pad: u8,
    pub magnification: f32,
    pub night_start: u32,
    pub night_end: u32,
}

/// Diagnostics (`fc4_clock24_get_state`): every field of §2 and the timer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_clock24_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub day_night_notifications: u8,
    pub timer_running: u8,
    pub _pad: u8,
    pub pause_count: i32,
    pub hidden_pause_count: i32,
    pub magnification: f32,
    pub min_magnification: f32,
    pub max_magnification: f32,
    pub seconds: u32,
    pub visual_seconds: u32,
    pub night_start: u32,
    pub night_end: u32,
    pub visual_mode: i32,
    pub timer_elapsed_us: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_clock24_tuning>() == 20);
    assert!(std::mem::size_of::<fc4_clock24_state>() == 56);
    assert!(std::mem::size_of::<fc4_clock24_host>() == if cfg!(target_pointer_width = "32") { 52 } else { 104 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_clock24_host,
    /// The tuning `fc4_clock24_init` was given, read by the core's `Init`.
    tuning: Rc<Cell<Option<ClockTuning>>>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

impl ClockHost for CHost {
    fn now_micros(&self) -> u64 {
        cb!(self, now_micros, 0)
    }
    fn post(&self, msg_type: u32, data2: u32, data3: u32) {
        cb!(self, post, (), msg_type, data2, data3)
    }
    fn add_system_service(&self) {
        cb!(self, add_system_service, ())
    }
    fn remove_system_service(&self) {
        cb!(self, remove_system_service, ())
    }
    fn add_to_on_idle(&self) {
        cb!(self, add_to_on_idle, ())
    }
    fn remove_from_on_idle(&self) {
        cb!(self, remove_from_on_idle, ())
    }
    fn has_cheat_manager(&self) -> bool {
        cb!(self, has_cheat_manager, false)
    }
    fn register_cheat(&self, id: u32, name: &str) {
        cb!(self, register_cheat, (), id, name.as_ptr(), name.len())
    }
    fn add_cheat_notification(&self, id: u32) {
        cb!(self, add_cheat_notification, (), id)
    }
    fn remove_cheat_notification(&self, id: u32) {
        cb!(self, remove_cheat_notification, (), id)
    }
    fn unregister_cheat(&self, id: u32) {
        cb!(self, unregister_cheat, (), id)
    }
    fn tuning(&self) -> Option<ClockTuning> {
        self.tuning.get()
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    core: RefCell<Clock24>,
    tuning: Rc<Cell<Option<ClockTuning>>>,
}

/// SAFETY: `c` must be NULL or a live pointer from `fc4_clock24_create`.
unsafe fn handle<'a>(c: *const fc4_clock24) -> Option<&'a Handle> {
    (c as *const Handle).as_ref()
}

/// Runs `f` on the clock; the fallback for NULL, a re-entrant call or a
/// panic.
fn with_mut<R>(c: *const fc4_clock24, fallback: R, f: impl FnOnce(&mut Clock24) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_clock24*.
    let Some(h) = (unsafe { handle(c) }) else {
        return fallback;
    };
    let Ok(mut k) = h.core.try_borrow_mut() else {
        return fallback;
    };
    guard(fallback, || f(&mut k))
}

/// As `with_mut`, read-only.
fn with_ref<R>(c: *const fc4_clock24, fallback: R, f: impl FnOnce(&Clock24) -> R) -> R {
    // SAFETY: as `with_mut`.
    let Some(h) = (unsafe { handle(c) }) else {
        return fallback;
    };
    let Ok(k) = h.core.try_borrow() else {
        return fallback;
    };
    guard(fallback, || f(&k))
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

/// Creates the clock (§2: noon, magnification 100, night 20 → 4). The host
/// table is copied. NULL if the table is missing or too small.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_create(host: *const fc4_clock24_host) -> *mut fc4_clock24 {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_clock24_host>() => {
            let tuning = Rc::new(Cell::new(None));
            let c = Clock24::new(Box::new(CHost {
                h: *h,
                tuning: tuning.clone(),
            }));
            Box::into_raw(Box::new(Handle {
                core: RefCell::new(c),
                tuning,
            })) as *mut fc4_clock24
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_destroy(c: *mut fc4_clock24) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut Handle)));
    }
}

/// `QueryInterface` (§1): the clock's four IIDs and the system-service
/// interface `0x287FB697` *[inferred]*.
#[no_mangle]
pub extern "C" fn fc4_clock24_answers_iid(iid: u32) -> bool {
    answers_iid(iid)
}

/// `Init` (slot 3, §4.2). `tuning` = the "Main Simulator" exemplar's clock
/// properties (copied); NULL = no exemplar (nothing changes). False for a
/// tuning struct that is too small.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_init(c: *mut fc4_clock24, tuning: *const fc4_clock24_tuning) -> bool {
    let t = match tuning.as_ref() {
        Some(t) if (t.struct_size as usize) >= std::mem::size_of::<fc4_clock24_tuning>() => Some(ClockTuning {
            magnification: (t.has_magnification != 0).then_some(t.magnification),
            night_start: (t.has_night_start != 0).then_some(t.night_start),
            night_end: (t.has_night_end != 0).then_some(t.night_end),
        }),
        Some(_) => return false,
        None => None,
    };
    let Some(h) = handle(c) else { return false };
    h.tuning.set(t);
    with_mut(c, false, Clock24::init)
}

/// `Shutdown` (slot 4, §4.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_shutdown(c: *mut fc4_clock24) -> bool {
    with_mut(c, false, Clock24::shutdown)
}

/// `OnIdle` (system-service slot 11, §4.3); the argument is not used. True.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_on_idle(c: *mut fc4_clock24, _idle_count: u32) -> bool {
    with_mut(c, true, Clock24::on_idle)
}

/// `DoMessage` (§3.2, §4.8): the cheat message `0x230E27AC` with data1 = the
/// cheat ID and `text` / `text_len` = the typed text (NULL = none). True.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_do_message(
    c: *mut fc4_clock24,
    msg_type: u32,
    data1: u32,
    text: *const u8,
    text_len: usize,
) -> bool {
    let t: Option<&[u8]> = if text.is_null() {
        None
    } else if text_len == 0 {
        Some(&[])
    } else {
        Some(std::slice::from_raw_parts(text, text_len))
    };
    with_mut(c, true, |k| k.do_message(msg_type, data1, t))
}

/// `SetVisualTimeMode` (slot 5) / `GetVisualTimeMode` (slot 6).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_visual_time_mode(c: *mut fc4_clock24, mode: i32) {
    with_mut(c, (), |k| k.set_visual_time_mode(mode))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_visual_time_mode(c: *const fc4_clock24) -> i32 {
    with_ref(c, 0, Clock24::visual_time_mode)
}

/// `GetTimePercentage(float* out)` (slot 7; the game's form, not the SDK's).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_time_percentage(c: *const fc4_clock24, out: *mut f32) {
    with_ref(c, (), |k| put(out, k.time_percentage()))
}

/// `SetTimePercentage(float&)` (slot 8).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_time_percentage(c: *mut fc4_clock24, p: f32) {
    with_mut(c, (), |k| k.set_time_percentage(p))
}

/// `GetVisualTimePercentage(float* out)` (slot 9).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_visual_time_percentage(c: *const fc4_clock24, out: *mut f32) {
    with_ref(c, (), |k| put(out, k.visual_time_percentage()))
}

/// `GetHour` (slot 10).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_hour(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::hour)
}

/// `SetHour` (slot 11).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_hour(c: *mut fc4_clock24, hour: u32) {
    with_mut(c, (), |k| k.set_hour(hour))
}

/// `GetVisualHour` (slot 12).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_visual_hour(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::visual_hour)
}

/// `GetMinute` (slot 13).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_minute(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::minute)
}

/// `SetMinute` (slot 14): returns the new `seconds`.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_minute(c: *mut fc4_clock24, minute: u32) -> u32 {
    with_mut(c, 0, |k| k.set_minute(minute))
}

/// `GetVisualMinute` (slot 15).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_visual_minute(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::visual_minute)
}

/// `GetSeconds` (slot 16): the second within the minute.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_seconds(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::second)
}

/// `SetSeconds` (slot 17): returns `GetMinute()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_seconds(c: *mut fc4_clock24, seconds: u32) -> u32 {
    with_mut(c, 0, |k| k.set_seconds(seconds))
}

/// `GetVisualSeconds` (slot 18).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_visual_seconds(c: *const fc4_clock24) -> u32 {
    with_ref(c, 0, Clock24::visual_second)
}

/// `GetEventHour(index)` (slot 19): night start (0), night end (1), else 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_event_hour(c: *const fc4_clock24, index: u32) -> u32 {
    with_ref(c, 0, |k| k.event_hour(index))
}

/// `GetTimeMagnification(float&)` (slot 20): writes `*out` and returns its
/// bit pattern.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_time_magnification(c: *const fc4_clock24, out: *mut f32) -> u32 {
    with_ref(c, 0, |k| {
        let m = k.time_magnification();
        put(out, m);
        m.to_bits()
    })
}

/// `SetTimeMagnification` (slot 21).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_time_magnification(c: *mut fc4_clock24, m: f32) -> bool {
    with_mut(c, false, |k| k.set_time_magnification(m))
}

/// `GetTimeMagnificationMinMax(float&, float&)` (slot 22): returns the bits
/// of the maximum.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_time_magnification_min_max(
    c: *const fc4_clock24,
    min: *mut f32,
    max: *mut f32,
) -> u32 {
    with_ref(c, 0, |k| {
        let (a, b) = k.time_magnification_min_max();
        put(min, a);
        put(max, b);
        b.to_bits()
    })
}

/// `SetTimeMagnificationMinMax` (slot 23).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_set_time_magnification_min_max(c: *mut fc4_clock24, min: f32, max: f32) -> bool {
    with_mut(c, false, |k| k.set_time_magnification_min_max(min, max))
}

/// `Pause` (slot 24).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_pause(c: *mut fc4_clock24, on: bool) -> bool {
    with_mut(c, false, |k| k.pause(on))
}

/// `IsPaused` (slot 25).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_is_paused(c: *const fc4_clock24) -> bool {
    with_ref(c, false, Clock24::is_paused)
}

/// `HiddenPause` (slot 26).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_hidden_pause(c: *mut fc4_clock24, on: bool) -> bool {
    with_mut(c, false, |k| k.hidden_pause(on))
}

/// `IsHiddenPaused` (slot 27).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_is_hidden_paused(c: *const fc4_clock24) -> bool {
    with_ref(c, false, Clock24::is_hidden_paused)
}

/// `IsNight` (slot 28).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_is_night(c: *const fc4_clock24) -> bool {
    with_ref(c, false, Clock24::is_night)
}

/// `EnableDayNightTransitionNotifications` (slot 29).
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_enable_day_night_notifications(c: *mut fc4_clock24, on: bool) {
    with_mut(c, (), |k| k.enable_day_night_transition_notifications(on))
}

/// Diagnostics; false for NULL or an in-use object.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_get_state(c: *const fc4_clock24, out: *mut fc4_clock24_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_ref(c, false, |k| {
        let s = k.state();
        *o = fc4_clock24_state {
            struct_size: std::mem::size_of::<fc4_clock24_state>() as u32,
            initialized: s.initialized.into(),
            day_night_notifications: s.day_night_notifications.into(),
            timer_running: s.timer.running.into(),
            _pad: 0,
            pause_count: s.pause_count,
            hidden_pause_count: s.hidden_pause_count,
            magnification: s.magnification,
            min_magnification: s.min_magnification,
            max_magnification: s.max_magnification,
            seconds: s.seconds,
            visual_seconds: s.visual_seconds,
            night_start: s.night_start,
            night_end: s.night_end,
            visual_mode: s.visual_mode,
            timer_elapsed_us: k.timer_elapsed_us(),
        };
        true
    })
}

/// `Write` (§7): version 2, 26 bytes.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_write(c: *const fc4_clock24, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_ref(c, false, |k| k.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<std::rc::Rc<dyn fc4_core::stream::SaveObject>> {
    None
}

/// `Read` (§7): no version check; version 1 skips its timer record.
#[no_mangle]
pub unsafe extern "C" fn fc4_clock24_read(c: *mut fc4_clock24, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_mut(c, false, |k| {
        k.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
