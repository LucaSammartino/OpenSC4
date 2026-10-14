// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/clock24.md`: a fake microsecond clock the
//! vector sets, a logged framework (system services, on-idle list, cheat code
//! manager, posted messages) and the "Main Simulator" exemplar, with ops that
//! drive the 24-hour clock. See README.md ("24-hour clock ops").

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use fc4_core::clock24::{
    answers_iid, msg, Clock24, ClockHost, ClockTuning, CHEAT_STOPWATCH, CHEAT_WHAT_TIME_IZ_IT, SHIPPED_TUNING,
};
use fc4_core::stream::{ByteReader, ByteWriter};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    clock: RefCell<Option<Clock24>>,
    now: Cell<u64>,
    cheat_manager: Cell<bool>,
    tuning: Cell<Option<ClockTuning>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            clock: RefCell::new(None),
            now: Cell::new(1_000_000),
            cheat_manager: Cell::new(true),
            tuning: Cell::new(Some(SHIPPED_TUNING)),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn with<T>(&self, f: impl FnOnce(&mut Clock24) -> R<T>) -> R<T> {
        let mut b = self
            .clock
            .try_borrow_mut()
            .map_err(|_| "the clock is busy".to_string())?;
        let c = b.as_mut().ok_or("no clock (ck_create)")?;
        f(c)
    }
}

/// The world as the clock's host.
struct CHost(Weak<World>);

impl CHost {
    fn w(&self) -> Rc<World> {
        self.0.upgrade().expect("world alive")
    }
    fn call(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.w().events.push(kind, fields);
    }
}

/// FreeCity4's names of the posted messages (§3.3).
fn message_name(m: u32) -> &'static str {
    match m {
        msg::HOUR_CHANGED => "HourChanged",
        msg::TIME_TICK => "TimeTick",
        msg::VISUAL_HOUR_CHANGED => "VisualHourChanged",
        msg::VISUAL_TIME_TICK => "VisualTimeTick",
        msg::DAY_NIGHT_CHANGED => "DayNightChanged",
        msg::PAUSE_CHANGED => "PauseChanged",
        msg::HIDDEN_PAUSE_CHANGED => "HiddenPauseChanged",
        _ => "other",
    }
}

impl ClockHost for CHost {
    fn now_micros(&self) -> u64 {
        self.w().now.get()
    }
    fn post(&self, msg_type: u32, data2: u32, data3: u32) {
        self.call(
            "post",
            vec![
                ("msg", ji(msg_type)),
                ("name", Json::Str(message_name(msg_type).into())),
                ("data2", ji(data2)),
                ("data3", ji(data3)),
            ],
        );
    }
    fn add_system_service(&self) {
        self.call("add_system_service", vec![]);
    }
    fn remove_system_service(&self) {
        self.call("remove_system_service", vec![]);
    }
    fn add_to_on_idle(&self) {
        self.call("add_to_on_idle", vec![]);
    }
    fn remove_from_on_idle(&self) {
        self.call("remove_from_on_idle", vec![]);
    }
    fn has_cheat_manager(&self) -> bool {
        self.w().cheat_manager.get()
    }
    fn register_cheat(&self, id: u32, name: &str) {
        self.call("register_cheat", vec![("id", ji(id)), ("name", Json::Str(name.into()))]);
    }
    fn add_cheat_notification(&self, id: u32) {
        self.call("add_cheat_notification", vec![("id", ji(id))]);
    }
    fn remove_cheat_notification(&self, id: u32) {
        self.call("remove_cheat_notification", vec![("id", ji(id))]);
    }
    fn unregister_cheat(&self, id: u32) {
        self.call("unregister_cheat", vec![("id", ji(id))]);
    }
    fn tuning(&self) -> Option<ClockTuning> {
        self.w().tuning.get()
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "clock24", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ck_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => {
            *w.clock.borrow_mut() = Some(Clock24::new(Box::new(CHost(w.me.borrow().clone()))));
            Ok(())
        }
        "set" => set(&w, op),
        "idle" => idle(&w, op),
        "call" => call(&w, op),
        "expect" => expect(&w, op),
        "record" => record(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

/// `"shipped"`, null (no exemplar) or `{magnification?, night_start?,
/// night_end?}` (absent keys are missing properties).
fn tuning_of(j: &Json) -> R<Option<ClockTuning>> {
    match j {
        Json::Null => Ok(None),
        Json::Str(s) if s == "shipped" => Ok(Some(SHIPPED_TUNING)),
        Json::Object(_) => Ok(Some(ClockTuning {
            magnification: opt(j, "magnification")
                .map(|v| f32_of(v, "magnification"))
                .transpose()?,
            night_start: opt(j, "night_start").map(|v| u32_of(v, "night_start")).transpose()?,
            night_end: opt(j, "night_end").map(|v| u32_of(v, "night_end")).transpose()?,
        })),
        _ => Err("'tuning' must be \"shipped\", null or an object".into()),
    }
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "now") {
        w.now.set(i64_of(v, "now")? as u64);
    }
    if let Some(v) = opt(op, "cheat_manager") {
        w.cheat_manager.set(bool_of(v, "cheat_manager")?);
    }
    if let Some(v) = op.get("tuning") {
        w.tuning.set(tuning_of(v)?);
    }
    Ok(())
}

/// A time of day: seconds, or `"HH:MM:SS"`.
fn time_of(j: &Json, what: &str) -> R<u32> {
    if let Some(s) = j.as_str() {
        let p: Vec<&str> = s.split(':').collect();
        if p.len() == 3 {
            let n = |x: &str| x.parse::<u32>().map_err(|e| format!("{what}: {e}"));
            return Ok(n(p[0])? * 3600 + n(p[1])? * 60 + n(p[2])?);
        }
    }
    u32_of(j, what)
}

/// `ck_set` (test support, no messages): `seconds`, `visual_seconds`
/// (`"HH:MM:SS"` or a number), `night` (`[start, end]`).
fn set(w: &World, op: &Json) -> R<()> {
    w.with(|c| {
        let s = c.state();
        let seconds = opt(op, "seconds").map_or(Ok(s.seconds), |v| time_of(v, "seconds"))?;
        let visual = opt(op, "visual_seconds").map_or(Ok(s.visual_seconds), |v| time_of(v, "visual_seconds"))?;
        c.set_times(seconds, visual);
        if let Some(n) = opt(op, "night") {
            let a = scalar::arr(n, "night")?;
            c.set_night_hours(u32_of(&a[0], "night")?, u32_of(&a[1], "night")?);
        }
        Ok(())
    })
}

/// `ck_idle`: `elapsed?` sets the timer's elapsed time (µs) by moving the
/// clock (the timer must be running, or already show that time); `advance?`
/// moves the clock first; `count?` (1) repeats (`advance` before each call).
/// Then `OnIdle`; `expect?` = its result.
fn idle(w: &World, op: &Json) -> R<()> {
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    for _ in 0..count {
        if let Some(a) = opt(op, "advance") {
            w.now.set(w.now.get() + i64_of(a, "advance")? as u64);
        }
        if let Some(e) = opt(op, "elapsed") {
            let e = i64_of(e, "elapsed")? as u64;
            let (running, had) = w.with(|c| Ok((c.timer().running, c.timer_elapsed_us())))?;
            if had != e {
                if !running || had > e {
                    return Err(format!("the timer shows {had} µs (running: {running}); cannot set {e}"));
                }
                w.now.set(w.now.get() + (e - had));
            }
        }
        let r = w.with(|c| Ok(c.on_idle()))?;
        check_expect(op, Json::Bool(r))?;
    }
    Ok(())
}

/// A cheat by ID or by name.
fn cheat_id(j: &Json) -> R<u32> {
    match j.as_str() {
        Some("WhatTimeIzIt") => Ok(CHEAT_WHAT_TIME_IZ_IT),
        Some("Stopwatch") => Ok(CHEAT_STOPWATCH),
        _ => u32_of(j, "cheat"),
    }
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let a = |i| arg(op, i);
    let u = |i| -> R<u32> { u32_of(a(i)?, "argument") };
    let f = |i| -> R<f32> { f32_of(a(i)?, "argument") };
    let b = |i| -> R<bool> { bool_of(a(i)?, "argument") };
    let got = w.with(|c| {
        Ok(match method {
            "init" => Json::Bool(c.init()),
            "shutdown" => Json::Bool(c.shutdown()),
            "on_idle" => Json::Bool(c.on_idle()),
            "query_interface" => Json::Bool(answers_iid(u(0)?)),
            "set_visual_time_mode" => {
                c.set_visual_time_mode(i64_of(a(0)?, "mode")? as i32);
                Json::Null
            }
            "visual_time_mode" => ji(c.visual_time_mode()),
            "time_percentage" => jf(c.time_percentage()),
            "visual_time_percentage" => jf(c.visual_time_percentage()),
            "set_time_percentage" => {
                c.set_time_percentage(f(0)?);
                Json::Null
            }
            "hour" => ji(c.hour()),
            "set_hour" => {
                c.set_hour(u(0)?);
                Json::Null
            }
            "visual_hour" => ji(c.visual_hour()),
            "minute" => ji(c.minute()),
            "set_minute" => ji(c.set_minute(u(0)?)),
            "visual_minute" => ji(c.visual_minute()),
            "second" => ji(c.second()),
            "set_seconds" => ji(c.set_seconds(u(0)?)),
            "visual_second" => ji(c.visual_second()),
            "event_hour" => ji(c.event_hour(u(0)?)),
            "time_magnification" => jf(c.time_magnification()),
            "set_time_magnification" => Json::Bool(c.set_time_magnification(f(0)?)),
            "time_magnification_min_max" => {
                let (lo, hi) = c.time_magnification_min_max();
                Json::Array(vec![jf(lo), jf(hi)])
            }
            "set_time_magnification_min_max" => Json::Bool(c.set_time_magnification_min_max(f(0)?, f(1)?)),
            "pause" => Json::Bool(c.pause(b(0)?)),
            "is_paused" => Json::Bool(c.is_paused()),
            "hidden_pause" => Json::Bool(c.hidden_pause(b(0)?)),
            "is_hidden_paused" => Json::Bool(c.is_hidden_paused()),
            "is_night" => Json::Bool(c.is_night()),
            "is_night_hour" => Json::Bool(c.is_night_hour(u(0)?)),
            "night_hours" => Json::Array((0..24).filter(|&h| c.is_night_hour(h)).map(ji).collect()),
            "enable_notifications" => {
                c.enable_day_night_transition_notifications(b(0)?);
                Json::Null
            }
            "change_visual_time" => {
                c.change_visual_time(time_of(a(0)?, "visual time")?);
                Json::Null
            }
            // [cheat, text]: the cheat message 0x230E27AC.
            "cheat" => {
                let text = str_of(a(1)?, "text")?;
                Json::Bool(c.do_message(msg::CHEAT, cheat_id(a(0)?)?, Some(text.as_bytes())))
            }
            // [type, data1, text or null]
            "message" => {
                let text = a(2).ok().and_then(Json::as_str);
                Json::Bool(c.do_message(u(0)?, u(1)?, text.map(str::as_bytes)))
            }
            other => return Err(format!("unknown method '{other}'")),
        })
    })?;
    if let Some(bits) = opt(op, "expect_bits") {
        let want = u32_of(bits, "expect_bits")?;
        let got_bits = match &got {
            Json::Float(v) => (*v as f32).to_bits(),
            other => return Err(format!("expect_bits on a non-float result {}", scalar::show(other))),
        };
        if got_bits != want {
            return Err(format!("bits 0x{got_bits:08X}, expected 0x{want:08X}"));
        }
    }
    check_expect(op, got)
}

fn expect(w: &World, op: &Json) -> R<()> {
    let want = field(op, "state")?;
    w.with(|c| {
        let s = c.state();
        let got = vec![
            ("initialized".to_owned(), Json::Bool(s.initialized)),
            ("pause_count".to_owned(), ji(s.pause_count)),
            ("hidden_pause_count".to_owned(), ji(s.hidden_pause_count)),
            ("magnification".to_owned(), jf(s.magnification)),
            ("magnification_bits".to_owned(), ji(s.magnification.to_bits())),
            ("min_magnification".to_owned(), jf(s.min_magnification)),
            ("min_magnification_bits".to_owned(), ji(s.min_magnification.to_bits())),
            ("max_magnification".to_owned(), jf(s.max_magnification)),
            ("seconds".to_owned(), ji(s.seconds)),
            ("visual_seconds".to_owned(), ji(s.visual_seconds)),
            ("night_start".to_owned(), ji(s.night_start)),
            ("night_end".to_owned(), ji(s.night_end)),
            ("notifications".to_owned(), Json::Bool(s.day_night_notifications)),
            ("visual_mode".to_owned(), ji(s.visual_mode)),
            ("timer_running".to_owned(), Json::Bool(s.timer.running)),
            ("timer_elapsed".to_owned(), ji(c.timer_elapsed_us() as i64)),
        ];
        check_state(&got, want)
    })
}

/// `ck_record`: `hex` compares `Write`; `read` (hex) reads into the current
/// clock (`expect` = the result), `remaining?` = the unread bytes.
fn record(w: &World, op: &Json) -> R<()> {
    if let Some(want) = opt(op, "hex") {
        let want = hex_decode(str_of(want, "hex")?)?;
        let got = w.with(|c| {
            let mut out = ByteWriter::new();
            c.write(&mut out);
            Ok(out.into_bytes())
        })?;
        if got != want {
            return Err(format!("record {}, expected {}", hex_encode(&got), hex_encode(&want)));
        }
    }
    if let Some(r) = opt(op, "read") {
        let bytes = hex_decode(str_of(r, "read")?)?;
        let (ok, remaining) = w.with(|c| {
            let mut rd = ByteReader::new(&bytes);
            let ok = c.read(&mut rd);
            Ok((ok, rd.remaining()))
        })?;
        check_expect(op, Json::Bool(ok))?;
        if let Some(n) = opt(op, "remaining") {
            let n = i64_of(n, "remaining")?;
            if remaining as i64 != n {
                return Err(format!("{remaining} bytes left, expected {n}"));
            }
        }
    }
    Ok(())
}
