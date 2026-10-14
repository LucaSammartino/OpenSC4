// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/history.md`: a scripted simulator date and
//! pass timer, a message server that delivers `RecordHistoryData`'s send to
//! the warehouse at once (as `MessageSend` does), and ops that drive the
//! history warehouse. See README.md ("History warehouse ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::history::{answers_iid, msg, HistoryHost, HistoryWarehouse, Point, Precision, Series};
use fc4_core::julian::{civil_from_julian, julian_from_civil};
use fc4_core::stream::{ByteReader, ByteWriter};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, i64_of, jf, ji, opt, show, u32_of, EventLog, R,
};

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    warehouse: RefCell<Option<HistoryWarehouse>>,
    date: Cell<u32>,
    now: Cell<u32>,
    now_step: Cell<u32>,
    /// Whether a send reaches the warehouse (when it is subscribed).
    deliver: Cell<bool>,
    subscribed: RefCell<BTreeSet<u32>>,
    saves: RefCell<BTreeMap<String, Vec<u8>>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            date: Cell::new(julian_from_civil(2000, 1, 1) as u32),
            deliver: Cell::new(true),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn host(&self) -> Box<dyn HistoryHost> {
        Box::new(WHost(self.me.borrow().clone()))
    }

    fn with<T>(&self, f: impl FnOnce(&mut HistoryWarehouse) -> R<T>) -> R<T> {
        let mut b = self
            .warehouse
            .try_borrow_mut()
            .map_err(|_| "the warehouse is busy".to_string())?;
        let w = b.as_mut().ok_or("no warehouse (hw_create)")?;
        f(w)
    }
}

/// The world as the warehouse's host.
struct WHost(Weak<World>);

impl WHost {
    fn w(&self) -> Rc<World> {
        self.0.upgrade().expect("world alive")
    }
}

impl HistoryHost for WHost {
    fn sim_date(&self) -> u32 {
        self.w().date.get()
    }
    fn now_ms(&self) -> u32 {
        let w = self.w();
        let t = w.now.get();
        w.now.set(t.wrapping_add(w.now_step.get()));
        t
    }
    fn subscribe(&self, msg_type: u32) {
        let w = self.w();
        w.subscribed.borrow_mut().insert(msg_type);
        w.events.push("subscribe", vec![("msg", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        let w = self.w();
        w.subscribed.borrow_mut().remove(&msg_type);
        w.events.push("unsubscribe", vec![("msg", ji(msg_type))]);
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.w().events.push(
            "add_agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.w().events.push("remove_agent", vec![]);
    }
    /// `MessageSend`: logged, then delivered to the warehouse at once when it
    /// is subscribed (the warehouse is not borrowed by `RecordHistoryData`).
    fn send(&self, msg_type: u32, data1: u32, data2: u32) {
        let w = self.w();
        let subscribed = w.subscribed.borrow().contains(&msg_type);
        w.events.push(
            "send",
            vec![
                ("msg", ji(msg_type)),
                ("data1", ji(data1)),
                ("data2", ji(data2 as i32)),
                ("delivered", Json::Bool(subscribed && w.deliver.get())),
            ],
        );
        if subscribed && w.deliver.get() {
            if let Ok(mut b) = w.warehouse.try_borrow_mut() {
                if let Some(h) = b.as_mut() {
                    h.do_message(msg_type, data1, data2);
                }
            }
        }
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "history", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("hw_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => {
            *w.warehouse.borrow_mut() = Some(HistoryWarehouse::new(w.host()));
            w.subscribed.borrow_mut().clear();
            Ok(())
        }
        "series" => set_series(&w, op),
        "pass" => w.with(|h| {
            h.set_pass(
                u32_of(field(op, "state")?, "state")?,
                opt(op, "cursor").map(|c| u32_of(c, "cursor")).transpose()?,
                opt(op, "counter").map_or(Ok(0), |c| u32_of(c, "counter"))?,
            );
            Ok(())
        }),
        "call" => call(&w, op),
        "days" => days(&w, op),
        "expect" => expect(&w, op),
        "record" => record(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

/// A date: a Julian day number, or `"YYYY-MM-DD"`.
fn date_of(j: &Json, what: &str) -> R<u32> {
    if let Some(s) = j.as_str() {
        let p: Vec<&str> = s.split('-').collect();
        if p.len() == 3 && p[0].len() == 4 {
            let n = |x: &str| x.parse::<i32>().map_err(|e| format!("{what}: {e}"));
            return Ok(julian_from_civil(n(p[0])?, n(p[1])?, n(p[2])?) as u32);
        }
    }
    u32_of(j, what)
}

fn civil(d: u32) -> String {
    let c = civil_from_julian(d as i32);
    format!("{:04}-{:02}-{:02}", c.year, c.month, c.day)
}

/// `[[date, value]]` → points.
fn points_of(j: &Json) -> R<Vec<Point>> {
    arr(j, "points")?
        .iter()
        .map(|p| {
            let a = arr(p, "point")?;
            if a.len() != 2 {
                return Err("a point is [date, value]".into());
            }
            Ok(Point::new(date_of(&a[0], "date")?, i64_of(&a[1], "value")? as i32))
        })
        .collect()
}

fn points_json(p: &[Point], civil_dates: bool) -> Json {
    Json::Array(
        p.iter()
            .map(|p| {
                let d = if civil_dates {
                    Json::Str(civil(p.date))
                } else {
                    ji(p.date)
                };
                Json::Array(vec![d, ji(p.value)])
            })
            .collect(),
    )
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "date") {
        w.date.set(date_of(v, "date")?);
    }
    if let Some(v) = opt(op, "now") {
        w.now.set(u32_of(v, "now")?);
    }
    if let Some(v) = opt(op, "now_step") {
        w.now_step.set(u32_of(v, "now_step")?);
    }
    if let Some(v) = opt(op, "deliver") {
        w.deliver.set(bool_of(v, "deliver")?);
    }
    Ok(())
}

/// `hw_series`: `key`, or `keys: [first, count]` for `count` consecutive
/// keys with the same points.
fn set_series(w: &World, op: &Json) -> R<()> {
    let (first, count) = match opt(op, "keys") {
        Some(k) => {
            let k = arr(k, "keys")?;
            if k.len() != 2 {
                return Err("'keys' is [first, count]".into());
            }
            (u32_of(&k[0], "keys")?, u32_of(&k[1], "keys")?)
        }
        None => (u32_of(field(op, "key")?, "key")?, 1),
    };
    let points = opt(op, "points").map_or(Ok(Vec::new()), points_of)?;
    let max_count = opt(op, "max_count").map_or(Ok(0), |v| u32_of(v, "max_count"))?;
    w.with(|h| {
        for i in 0..count {
            h.set_series(
                first.wrapping_add(i),
                Series {
                    points: points.clone(),
                    max_count,
                },
            );
        }
        Ok(())
    })
}

fn precision(op: &Json) -> R<Precision> {
    Ok(Precision::from_bits(
        opt(op, "precision").map_or(Ok(24), |v| u32_of(v, "precision"))?,
    ))
}

/// Checks `expect_bits` (`["0x…"]`, the f32 bit patterns of the results).
fn check_bits(op: &Json, got: &[f32]) -> R<()> {
    let Some(want) = opt(op, "expect_bits") else {
        return Ok(());
    };
    let want = arr(want, "expect_bits")?;
    let got_bits: Vec<u32> = got.iter().map(|v| v.to_bits()).collect();
    let want_bits = want.iter().map(|b| u32_of(b, "expect_bits")).collect::<R<Vec<u32>>>()?;
    if got_bits == want_bits {
        Ok(())
    } else {
        Err(format!(
            "bits {:08X?} ({got:?}), expected {want_bits:08X?}",
            got_bits.as_slice()
        ))
    }
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = scalar::str_of(field(op, "method")?, "method")?;
    let a = |i| arg(op, i);
    let u = |i| -> R<u32> { u32_of(a(i)?, "argument") };
    let d = |i| -> R<u32> { date_of(a(i)?, "date") };
    let got = match method {
        // RecordHistoryData only sends; the world delivers it without the
        // warehouse being borrowed.
        "record" => {
            let host = WHost(w.me.borrow().clone());
            if w.warehouse.borrow().is_none() {
                return Err("no warehouse (hw_create)".into());
            }
            Json::Bool(HistoryWarehouse::record_history_data(
                &host,
                u(0)?,
                i64_of(a(1)?, "value")? as i32,
            ))
        }
        "query_interface" => Json::Bool(answers_iid(u(0)?)),
        _ => w.with(|h| {
            Ok(match method {
                "init" => Json::Bool(h.init()),
                "shutdown" => Json::Bool(h.shutdown()),
                "message" => {
                    let data = |i| a(i).map_or(Ok(0), |v| u32_of(v, "data"));
                    Json::Bool(h.do_message(u(0)?, data(1)?, data(2)?))
                }
                "new_day" => Json::Bool(h.do_message(msg::SIM_NEW_DAY, 0, 0)),
                "new_year" => Json::Bool(h.do_message(msg::SIM_NEW_YEAR, 0, 0)),
                "store" => {
                    h.store(u(0)?, i64_of(a(1)?, "value")? as i32);
                    Json::Null
                }
                "set_size" | "set_queue_length" => Json::Bool(h.set_history_data_queue_length(u(0)?, u(1)?)),
                "coalesce" => Json::Bool(h.coalesce(u(0)?, d(1)?, d(2)?, i64_of(a(3)?, "granularity")? as i32)),
                "dispose" => Json::Bool(h.dispose(u(0)?)),
                "dispose_range" => Json::Bool(h.dispose_range(u(0)?, d(1)?, d(2)?)),
                "thinning_step" => Json::Bool(h.thinning_step()),
                "variable_coalesce" => {
                    h.variable_coalesce(u(0)?);
                    Json::Null
                }
                "get_count" => match h.get_count(u(0)?, d(1)?, d(2)?) {
                    Some(v) => {
                        check_bits(op, &[v])?;
                        jf(v)
                    }
                    None => Json::Null,
                },
                "get_trend" => match h.get_trend(u(0)?, d(1)?, d(2)?, precision(op)?) {
                    Some((s, i)) => {
                        check_bits(op, &[s, i])?;
                        Json::Array(vec![jf(s), jf(i)])
                    }
                    None => Json::Null,
                },
                "get_point" => match h.get_point(u(0)?, d(1)?, precision(op)?) {
                    Some(v) => {
                        check_bits(op, &[v])?;
                        jf(v)
                    }
                    None => Json::Null,
                },
                "get_min_max_average" => match h.get_min_max_average(u(0)?, d(1)?, d(2)?, precision(op)?) {
                    Some((a, b, c)) => {
                        check_bits(op, &[a, b, c])?;
                        Json::Array(vec![jf(a), jf(b), jf(c)])
                    }
                    None => Json::Null,
                },
                other => return Err(format!("unknown method '{other}'")),
            })
        })?,
    };
    check_expect(op, got)
}

/// `hw_days`: runs the days `from` … `to` as the simulator would: a year
/// change delivers SimNewYear, each day delivers SimNewDay until it returns
/// true, then `record` (`[key, "index"]` records the day's index from `from`;
/// `[key, value]` a constant) is stored through the record message.
fn days(w: &World, op: &Json) -> R<()> {
    let from = date_of(field(op, "from")?, "from")?;
    let to = date_of(field(op, "to")?, "to")?;
    let rec = opt(op, "record").map(|r| arr(r, "record")).transpose()?;
    let mut year = civil_from_julian(w.date.get() as i32).year;
    for day in from..=to {
        w.date.set(day);
        let y = civil_from_julian(day as i32).year;
        w.with(|h| {
            if y != year {
                h.do_message(msg::SIM_NEW_YEAR, 0, 0);
            }
            let mut calls = 0;
            while !h.do_message(msg::SIM_NEW_DAY, 0, 0) {
                calls += 1;
                if calls > 1000 {
                    return Err(format!("SimNewDay keeps returning false on {}", civil(day)));
                }
            }
            if let Some(r) = rec {
                let key = u32_of(&r[0], "record key")?;
                let value = if r[1].as_str() == Some("index") {
                    (day - from) as i32
                } else {
                    i64_of(&r[1], "record value")? as i32
                };
                h.do_message(msg::RECORD, key, value as u32);
            }
            Ok(())
        })?;
        year = y;
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    w.with(|h| {
        if let Some(want) = opt(op, "state") {
            let got = vec![
                ("initialized".to_owned(), Json::Bool(h.is_initialized())),
                ("pass_state".to_owned(), ji(h.pass_state())),
                ("cursor".to_owned(), h.cursor().map_or(Json::Null, ji)),
                ("pass_counter".to_owned(), ji(h.pass_counter())),
                ("series_count".to_owned(), ji(h.series_count() as i64)),
                ("total_points".to_owned(), ji(h.total_points() as i64)),
                ("keys".to_owned(), Json::Array(h.keys().into_iter().map(ji).collect())),
            ];
            check_state(&got, want)?;
        }
        if let Some(k) = opt(op, "series") {
            let key = u32_of(k, "series")?;
            let s = h.series(key);
            if let Some(m) = opt(op, "missing") {
                if s.is_none() != bool_of(m, "missing")? {
                    return Err(format!("series 0x{key:X}: missing = {}", s.is_none()));
                }
            }
            if let Some(s) = s {
                if let Some(want) = opt(op, "points") {
                    let want = points_of(want)?;
                    if s.points != want {
                        return Err(format!(
                            "series 0x{key:X}: points {}, expected {}",
                            show(&points_json(&s.points, true)),
                            show(&points_json(&want, true))
                        ));
                    }
                }
                if let Some(want) = opt(op, "dates") {
                    let want = arr(want, "dates")?
                        .iter()
                        .map(|d| date_of(d, "date"))
                        .collect::<R<Vec<u32>>>()?;
                    let got: Vec<u32> = s.points.iter().map(|p| p.date).collect();
                    if got != want {
                        return Err(format!(
                            "series 0x{key:X}: dates {:?}, expected {:?}",
                            got.iter().map(|&d| civil(d)).collect::<Vec<_>>(),
                            want.iter().map(|&d| civil(d)).collect::<Vec<_>>()
                        ));
                    }
                }
                if let Some(want) = opt(op, "values") {
                    let want = arr(want, "values")?
                        .iter()
                        .map(|v| i64_of(v, "value").map(|v| v as i32))
                        .collect::<R<Vec<i32>>>()?;
                    let got: Vec<i32> = s.points.iter().map(|p| p.value).collect();
                    if got != want {
                        return Err(format!("series 0x{key:X}: values {got:?}, expected {want:?}"));
                    }
                }
                if let Some(want) = opt(op, "count") {
                    let n = i64_of(want, "count")?;
                    if s.points.len() as i64 != n {
                        return Err(format!("series 0x{key:X}: {} points, expected {n}", s.points.len()));
                    }
                }
                if let Some(want) = opt(op, "max_count") {
                    let m = u32_of(want, "max_count")?;
                    if s.max_count != m {
                        return Err(format!("series 0x{key:X}: maxCount {}, expected {m}", s.max_count));
                    }
                }
            } else if opt(op, "missing").is_none() {
                return Err(format!("series 0x{key:X} is missing"));
            }
        }
        Ok(())
    })
}

fn write(h: &HistoryWarehouse) -> Vec<u8> {
    let mut out = ByteWriter::new();
    h.write(&mut out);
    out.into_bytes()
}

/// `hw_record`: `hex` compares `Write`; `read` (hex) reads into the current
/// warehouse (`expect` = the result), `remaining?` = the unread bytes.
fn record(w: &World, op: &Json) -> R<()> {
    if let Some(want) = opt(op, "hex") {
        let want = hex_decode(scalar::str_of(want, "hex")?)?;
        let got = w.with(|h| Ok(write(h)))?;
        if got != want {
            return Err(format!("record {}, expected {}", hex_encode(&got), hex_encode(&want)));
        }
    }
    if let Some(r) = opt(op, "read") {
        let bytes = hex_decode(scalar::str_of(r, "read")?)?;
        let (ok, remaining) = w.with(|h| {
            let mut rd = ByteReader::new(&bytes);
            let ok = h.read(&mut rd);
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

/// `hw_save`: `Write` into a slot.
fn save(w: &World, op: &Json) -> R<()> {
    let slot = scalar::str_of(field(op, "slot")?, "slot")?.to_owned();
    let bytes = w.with(|h| Ok(write(h)))?;
    w.saves.borrow_mut().insert(slot, bytes);
    Ok(())
}

/// `hw_load`: a fresh warehouse reads a slot (`version?` patches the leading
/// u16, `truncate?` drops bytes from the end).
fn load(w: &World, op: &Json) -> R<()> {
    let slot = scalar::str_of(field(op, "slot")?, "slot")?;
    let mut bytes = w
        .saves
        .borrow()
        .get(slot)
        .cloned()
        .ok_or_else(|| format!("no save slot '{slot}'"))?;
    if let Some(v) = opt(op, "version") {
        let v = u32_of(v, "version")? as u16;
        bytes[..2].copy_from_slice(&v.to_le_bytes());
    }
    if let Some(n) = opt(op, "truncate") {
        let n = i64_of(n, "truncate")? as usize;
        bytes.truncate(bytes.len().saturating_sub(n));
    }
    let mut h = HistoryWarehouse::new(w.host());
    let ok = h.read(&mut ByteReader::new(&bytes));
    *w.warehouse.borrow_mut() = Some(h);
    w.subscribed.borrow_mut().clear();
    check_expect(op, Json::Bool(ok))
}

/// `hw_expect_save`: `slot` with `hex`, `same_as` (another slot) or `length`.
fn expect_save(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let slot = scalar::str_of(field(op, "slot")?, "slot")?;
    let got = saves.get(slot).ok_or_else(|| format!("no save slot '{slot}'"))?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(scalar::str_of(h, "hex")?)?;
        if *got != want {
            return Err(format!(
                "slot {slot}: {}, expected {}",
                hex_encode(got),
                hex_encode(&want)
            ));
        }
    }
    if let Some(o) = opt(op, "same_as") {
        let other = scalar::str_of(o, "same_as")?;
        let b = saves.get(other).ok_or_else(|| format!("no save slot '{other}'"))?;
        if got != b {
            return Err(format!(
                "slots {slot} and {other} differ: {} / {}",
                hex_encode(got),
                hex_encode(b)
            ));
        }
    }
    if let Some(n) = opt(op, "length") {
        let n = i64_of(n, "length")?;
        if got.len() as i64 != n {
            return Err(format!("slot {slot}: {} bytes, expected {n}", got.len()));
        }
    }
    Ok(())
}
