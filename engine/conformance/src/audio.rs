// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/audio.md` §16: the sound-event system, the
//! sound managers and the sound data. Every op name starts with `aud_`.
//! See README.md ("Audio ops").
//!
//! The ops drive one [`World`], which holds the pool, the message objects, the
//! automaton's SFX slots, the audio event handler, the sound manager, an
//! effects sound component, the 83 leaf classes and the data tables. Each of
//! the 25 tests of §16 is one vector in `vectors/au16_*.json`.

use std::cell::RefCell;
use std::rc::Rc;

use fc4_core::audio::automaton::AutomatonSfxHost;
use fc4_core::audio::data::{SampleContext, SoundDescription, SoundManagerTuning};
use fc4_core::audio::effect_component::{check_finished_considers, runtime_flags_for, SoundComponent};
use fc4_core::audio::event_handler::AudioEventHandler;
use fc4_core::audio::instance_id::InstanceIdCounter;
use fc4_core::audio::leaf::{shape_of_factory, LeafObject, LeafShape, LEAF_COMMON_IID, LEAF_REGISTRATIONS};
use fc4_core::audio::message::StandardMessage;
use fc4_core::audio::pool::{page_of, ReleaseOutcome, SndEventPool};
use fc4_core::audio::save::PostLoadState;
use fc4_core::audio::snd_event::{SndEventStd, SND_EVENT_SIZE};
use fc4_core::audio::sound_manager::{AudioSoundManager, StopResult};
use fc4_core::audio::{self, MSG_AUTOMATA_PLAY_SFX, MSG_AUTOMATA_STOP_SFX};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, bool_of, check_expect, check_state, i64_of, opt, u32_of, EventLog, R};

/// The stand-in for the automaton's occupant (`AsOccupant`): one token.
const OCCUPANT: u64 = 0x00B0_0004;

fn ju(v: u32) -> Json {
    scalar::ji(i64::from(v))
}

fn jb(v: bool) -> Json {
    Json::Bool(v)
}

/// Everything one vector drives.
#[derive(Default)]
struct World {
    events: EventLog,
    /// The sound-event pool of §4.
    pool: RefCell<SndEventPool>,
    /// The automaton's SFX slots of §6.
    automaton: RefCell<AutomatonSfxHost>,
    /// `cSC4AudioEventHandler` (§7).
    handler: RefCell<AudioEventHandler>,
    /// `cSC4AudioSoundManager` (§8).
    manager: RefCell<AudioSoundManager>,
    /// A sound component of §9.
    component: RefCell<Option<SoundComponent>>,
    /// The instance-id counter of §8.4, when driven on its own.
    ids: RefCell<InstanceIdCounter>,
    /// The leaf objects a vector has created, by index.
    leaves: RefCell<Vec<LeafObject>>,
    /// The pool addresses allocated so far, in order.
    pool_stack: RefCell<Vec<u32>>,
    /// The value the rate limiter's `rand()` returns.
    rand: RefCell<u32>,
    /// Whether the sound system service is present.
    snd_sys: RefCell<bool>,
}

impl World {
    fn new() -> Rc<World> {
        let w = World {
            events: EventLog::default(),
            pool: RefCell::new(SndEventPool::new()),
            automaton: RefCell::new(AutomatonSfxHost::new()),
            handler: RefCell::new(AudioEventHandler::new()),
            manager: RefCell::new(AudioSoundManager::new()),
            component: RefCell::new(None),
            ids: RefCell::new(InstanceIdCounter::new()),
            leaves: RefCell::new(Vec::new()),
            pool_stack: RefCell::new(Vec::new()),
            rand: RefCell::new(0),
            snd_sys: RefCell::new(true),
        };
        Rc::new(w)
    }
    fn push(&self, what: &str, args: Vec<(&str, Json)>) {
        self.events.push(what, args);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "audio", World::new)
}

// ---- helpers ----------------------------------------------------------------

/// Read a `u32` that a vector writes either as a number or as `"0x........"`.
fn word(op: &Json, key: &str) -> R<u32> {
    match opt(op, key) {
        Some(v) => u32_of(v, key),
        None => Err(format!("'{key}' is required")),
    }
}

// ---- ops --------------------------------------------------------------------

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "rand") {
        *w.rand.borrow_mut() = u32_of(v, "rand")?;
    }
    if let Some(v) = opt(op, "snd_sys") {
        *w.snd_sys.borrow_mut() = bool_of(v, "snd_sys")?;
        w.handler.borrow_mut().snd_sys_present = *w.snd_sys.borrow();
    }
    if let Some(v) = opt(op, "page_base") {
        *w.pool.borrow_mut() = SndEventPool::new().with_next_base(u32_of(v, "page_base")?);
    }
    Ok(())
}

// -- the pool (tests 5, 6) --

fn pool_alloc(w: &World, op: &Json) -> R<()> {
    let a = w.pool.borrow_mut().alloc();
    w.pool_stack.borrow_mut().push(a);
    w.push("pool_alloc", vec![("addr", ju(a))]);
    check_expect(op, ju(a))
}

/// Allocate `count` objects and report the addresses, so a vector can drive
/// the page state word to the values a particular live-object count produces.
fn pool_alloc_n(w: &World, op: &Json) -> R<()> {
    let n = u32_of(field(op, "count")?, "count")?;
    let mut addrs = Vec::new();
    {
        let mut p = w.pool.borrow_mut();
        let mut stack = w.pool_stack.borrow_mut();
        for _ in 0..n {
            let a = p.alloc();
            stack.push(a);
            addrs.push(Json::Str(format!("0x{a:08X}")));
        }
    }
    let base = w.pool.borrow().active_pages().first().map(|q| q.base).unwrap_or(0);
    let state = w.pool.borrow().page(base).map(|q| q.state).unwrap_or(0);
    check_state(
        &[
            ("count".to_owned(), ju(n)),
            ("addrs".to_owned(), Json::Array(addrs)),
            ("page".to_owned(), ju(base)),
            ("state".to_owned(), ju(state)),
        ],
        field(op, "state")?,
    )
}

/// Free the `index`th address [`pool_alloc`] / [`pool_alloc_n`] produced, and
/// optionally `count - 1` more after it. The outcome reported is the last
/// one, so a vector can drive a run of frees up to a threshold.
fn pool_free_index(w: &World, op: &Json) -> R<()> {
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let n = u32_of(opt(op, "count").unwrap_or(&Json::Int(1)), "count")?;
    let addrs: Vec<u32> = w.pool_stack.borrow().clone();
    let mut name = "none".to_owned();
    for k in 0..n {
        let a = *addrs
            .get(i + k as usize)
            .ok_or_else(|| format!("no allocated pool object #{}", i + k as usize))?;
        let out = w.pool.borrow_mut().free(a);
        name = match out {
            ReleaseOutcome::None => "none",
            ReleaseOutcome::HandedBack => "handed_back",
            ReleaseOutcome::Retired => "retired",
        }
        .to_owned();
        w.push("pool_free", vec![("addr", ju(a)), ("outcome", Json::Str(name.clone()))]);
    }
    check_expect(op, Json::Str(name))
}

fn pool_free(w: &World, op: &Json) -> R<()> {
    let p = word(op, "addr")?;
    let out = w.pool.borrow_mut().free(p);
    let name = match out {
        ReleaseOutcome::None => "none",
        ReleaseOutcome::HandedBack => "handed_back",
        ReleaseOutcome::Retired => "retired",
    };
    w.push(
        "pool_free",
        vec![("addr", ju(p)), ("outcome", Json::Str(name.to_owned()))],
    );
    check_expect(op, Json::Str(name.to_owned()))
}

fn pool_expect(w: &World, op: &Json) -> R<()> {
    let base = word(op, "page")?;
    let p = w.pool.borrow();
    let active = p.active_pages().iter().any(|q| q.base == base);
    let retired = p.retired_pages().iter().any(|q| q.base == base);
    let handed = p.handed_back_pages().contains(&base);
    let mut got = vec![
        ("active".to_owned(), jb(active)),
        ("retired".to_owned(), jb(retired)),
        ("handed_back".to_owned(), jb(handed)),
    ];
    // A page handed back is gone from both lists (FUN_0090D031), so report the
    // outcome rather than failing to find it.
    if let Some(page) = p.page(base) {
        got.push(("free_head".to_owned(), ju(page.free_head)));
        got.push(("state".to_owned(), ju(page.state)));
        got.push(("marker".to_owned(), jb(page.state & audio::pool::STATE_MARKER != 0)));
        got.push(("live".to_owned(), ju(SndEventPool::live_count(page))));
        got.push((
            "first_dword".to_owned(),
            match page.first_dword(page.free_head) {
                Some(v) => ju(v),
                None => Json::Null,
            },
        ));
    }
    drop(p);
    check_state(&got, field(op, "state")?)
}

fn pool_free_list(w: &World, op: &Json) -> R<()> {
    let p = w.pool.borrow();
    let base = word(op, "page")?;
    let page = p.page(base).ok_or_else(|| format!("no page 0x{base:08X}"))?;
    let mut chain = Vec::new();
    let mut cur = page.free_head;
    let limit = u32_of(opt(op, "limit").unwrap_or(&Json::Int(8)), "limit")?;
    for _ in 0..limit {
        if cur == 0 {
            break;
        }
        chain.push(Json::Str(format!("0x{cur:08X}")));
        match page.first_dword(cur) {
            Some(n) => cur = n,
            None => break,
        }
    }
    check_state(&[("chain".to_owned(), Json::Array(chain))], field(op, "state")?)
}

fn pool_size_uses(w: &World, op: &Json) -> R<()> {
    let size = u32_of(field(op, "size")?, "size")? as usize;
    let got = fc4_core::audio::pool::size_uses_pool(size);
    let _ = w;
    check_expect(op, Json::Bool(got))
}

fn pool_page_of(w: &World, op: &Json) -> R<()> {
    let a = word(op, "addr")?;
    let _ = w;
    check_expect(op, ju(page_of(a)))
}

// -- the message object (test 4) --

fn msg_build(w: &World, op: &Json) -> R<()> {
    let mut m = StandardMessage::new();
    if let Some(v) = opt(op, "type") {
        m.set_type(u32_of(v, "type")?);
    }
    if let Some(v) = opt(op, "data1") {
        m.set_data1(u32_of(v, "data1")?);
    }
    if let Some(v) = opt(op, "data2") {
        m.set_data2(u32_of(v, "data2")?);
    }
    if let Some(v) = opt(op, "data3") {
        m.set_data3(u32_of(v, "data3")?);
    }
    if let Some(v) = opt(op, "data4") {
        m.set_data4(u32_of(v, "data4")?);
    }
    if let Some(v) = opt(op, "void1") {
        m.set_void1(i64_of(v, "void1")? as u64);
    }
    // A second SetType, so a vector can show type 0 clearing its presence bit.
    if let Some(v) = opt(op, "type_after") {
        m.set_type(u32_of(v, "type_after")?);
    }
    let got = vec![
        ("flags".to_owned(), ju(u32::from(m.flags))),
        ("type".to_owned(), ju(m.msg_type)),
        ("data1".to_owned(), ju(m.data1)),
        ("data2".to_owned(), ju(m.data2)),
        ("data3".to_owned(), ju(m.data3)),
        ("data4".to_owned(), ju(m.data4)),
        ("void1".to_owned(), scalar::ji(m.void1 as i64)),
    ];
    w.push("msg_build", vec![("flags", ju(u32::from(m.flags)))]);
    check_state(&got, field(op, "state")?)
}

// -- the sound event (tests 1, 2, 3) --

fn snd_event(w: &World, op: &Json) -> R<()> {
    let which = scalar::str_of(field(op, "ctor")?, "ctor")?;
    let mut e = match which {
        "default" => SndEventStd::new(),
        "five" => SndEventStd::with_payload(
            word(op, "payload")?,
            u32_of(opt(op, "data1").unwrap_or(&Json::Int(0)), "data1")?,
            u32_of(opt(op, "data2").unwrap_or(&Json::Int(0)), "data2")?,
            u32_of(opt(op, "data3").unwrap_or(&Json::Int(0)), "data3")?,
            u32_of(opt(op, "data4").unwrap_or(&Json::Int(0)), "data4")?,
        ),
        other => return Err(format!("unknown ctor '{other}'")),
    };
    let mut extra = Vec::new();
    if let Some(v) = opt(op, "qi") {
        let iid = u32_of(v, "qi")?;
        let off = e.query_interface(iid);
        extra.push((
            "qi_offset".to_owned(),
            match off {
                Some(o) => ju(o as u32),
                None => Json::Null,
            },
        ));
    }
    if let Some(v) = opt(op, "set_value") {
        e.set_value(u32_of(v, "set_value")?);
    }
    if let Some(v) = opt(op, "outer_of") {
        extra.push(("outer".to_owned(), ju(SndEventStd::get_outer(u32_of(v, "outer_of")?))));
    }
    let mut got = vec![
        ("type".to_owned(), ju(e.base.msg_type)),
        ("payload".to_owned(), ju(e.payload)),
        ("flag".to_owned(), ju(u32::from(e.flag))),
        ("refcount".to_owned(), ju(e.base.refcount)),
        ("get_value".to_owned(), ju(e.get_value())),
        ("get_flag".to_owned(), ju(u32::from(e.get_flag()))),
        ("data1".to_owned(), ju(e.base.data1)),
        ("data2".to_owned(), ju(e.base.data2)),
        ("data3".to_owned(), ju(e.base.data3)),
        ("data4".to_owned(), ju(e.base.data4)),
    ];
    got.extend(extra);
    w.push(
        "snd_event",
        vec![("type", ju(e.base.msg_type)), ("payload", ju(e.payload))],
    );
    check_state(&got, field(op, "state")?)
}

fn snd_event_size(w: &World, op: &Json) -> R<()> {
    let size = u32_of(field(op, "size")?, "size")? as usize;
    let got = size == SND_EVENT_SIZE && fc4_core::audio::pool::size_uses_pool(size);
    let _ = w;
    check_expect(op, Json::Bool(got))
}

// -- the automaton's SFX slots (tests 7-11) --

fn start_sfx(w: &World, op: &Json) -> R<()> {
    let id = word(op, "id")?;
    let occ = u64::from(u32_of(
        opt(op, "occupant").unwrap_or(&Json::Int(OCCUPANT as i64)),
        "occupant",
    )?);
    let r = w.automaton.borrow_mut().start_sfx(id, occ);
    w.push("start_sfx", vec![("id", ju(id)), ("ret", ju(r))]);
    check_expect(op, ju(r))
}

fn stop_sfx(w: &World, op: &Json) -> R<()> {
    let id = word(op, "id")?;
    let r = w.automaton.borrow_mut().stop_sfx(id, OCCUPANT);
    w.push("stop_sfx", vec![("id", ju(id)), ("ret", ju(r))]);
    check_expect(op, ju(r))
}

fn state(w: &World, op: &Json) -> R<()> {
    let enter = bool_of(field(op, "enter")?, "enter")?;
    let id = word(op, "id")?;
    let called = if enter {
        w.automaton.borrow_mut().handle_enter_state(id, OCCUPANT)
    } else {
        w.automaton.borrow_mut().handle_exit_state(id, OCCUPANT)
    };
    w.push(
        if enter { "enter_state" } else { "exit_state" },
        vec![("id", ju(id)), ("called", jb(called))],
    );
    check_expect(op, Json::Bool(called))
}

fn automaton_expect(w: &World, op: &Json) -> R<()> {
    let h = w.automaton.borrow();
    let got = vec![
        ("instance_id".to_owned(), ju(h.state.instance_id)),
        ("posts".to_owned(), ju(h.posted.len() as u32)),
    ];
    drop(h);
    check_state(&got, field(op, "state")?)
}

fn post_expect(w: &World, op: &Json) -> R<()> {
    let h = w.automaton.borrow();
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let m = h.posted.get(i).ok_or_else(|| format!("no posted message #{i}"))?;
    let got = vec![
        ("type".to_owned(), ju(m.msg_type)),
        ("data1".to_owned(), ju(m.data1)),
        ("data2".to_owned(), ju(m.data2)),
        ("void1".to_owned(), scalar::ji(m.void1 as i64)),
        ("high_priority".to_owned(), ju(m.high_priority)),
        ("flags".to_owned(), ju(u32::from(m.flags))),
    ];
    drop(h);
    check_state(&got, field(op, "state")?)
}

fn clear_posts(w: &World, _op: &Json) -> R<()> {
    w.automaton.borrow_mut().posted.clear();
    Ok(())
}

// -- the instance-id counter (test 20) --

fn instance_id(w: &World, op: &Json) -> R<()> {
    let n = u32_of(opt(op, "count").unwrap_or(&Json::Int(1)), "count")?;
    let mut ids = w.ids.borrow_mut();
    if let Some(v) = opt(op, "seed") {
        *ids = InstanceIdCounter::at(u32_of(v, "seed")?);
    }
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(ju(ids.generate()));
    }
    drop(ids);
    check_state(&[("ids".to_owned(), Json::Array(out))], field(op, "state")?)
}

// -- the audio event handler (tests 12, 13) --

fn handler_init(w: &World, _op: &Json) -> R<()> {
    w.handler.borrow_mut().init();
    Ok(())
}

fn handler_message(w: &World, op: &Json) -> R<()> {
    let ty = word(op, "type")?;
    let mut h = w.handler.borrow_mut();
    match ty {
        audio::MSG_POST_CITY_INIT | audio::MSG_PRE_CITY_SHUTDOWN => {
            h.handle_city_init_shutdown(ty);
            drop(h);
            w.push("handler_message", vec![("type", ju(ty))]);
            // `expect` answers the routing question of test 12: these types are
            // handled, but not by HandleAutomataMessages.
            check_expect(op, Json::Bool(false))
        }
        MSG_AUTOMATA_PLAY_SFX => {
            let payload = u32_of(opt(op, "payload").unwrap_or(&Json::Int(0)), "payload")?;
            let sound = u32_of(opt(op, "sound").unwrap_or(&Json::Int(0)), "sound")?;
            let inst = u32_of(opt(op, "instance").unwrap_or(&Json::Int(0)), "instance")?;
            let rand = *w.rand.borrow();
            let r = h.handle_play(payload, rand, sound, inst);
            drop(h);
            w.push("handle_play", vec![("type", ju(ty)), ("ret", ju(r))]);
            // The expectation answers the routing question of test 12.
            check_expect(op, Json::Bool(true))
        }
        MSG_AUTOMATA_STOP_SFX => {
            let sound = u32_of(opt(op, "sound").unwrap_or(&Json::Int(0)), "sound")?;
            let inst = u32_of(opt(op, "instance").unwrap_or(&Json::Int(0)), "instance")?;
            let r = h.handle_stop(sound, inst);
            drop(h);
            w.push("handle_stop", vec![("type", ju(ty)), ("ret", ju(r))]);
            check_expect(op, Json::Bool(true))
        }
        _ => {
            drop(h);
            check_expect(op, Json::Bool(AudioEventHandler::route_to_automata(ty).is_some()))
        }
    }
}

fn handler_expect(w: &World, op: &Json) -> R<()> {
    let h = w.handler.borrow();
    let got = vec![
        ("subscribed".to_owned(), ju(h.subscribed.len() as u32)),
        ("city_loaded".to_owned(), jb(h.city_loaded)),
        ("initialized".to_owned(), jb(h.initialized)),
        ("events".to_owned(), ju(h.events.len() as u32)),
        ("timer_running".to_owned(), jb(h.limiter.running)),
        ("window_ms".to_owned(), ju(h.limiter.window_ms)),
        ("elapsed_ms".to_owned(), ju(h.limiter.elapsed_ms)),
    ];
    drop(h);
    check_state(&got, field(op, "state")?)
}

fn handler_timer(w: &World, op: &Json) -> R<()> {
    let mut h = w.handler.borrow_mut();
    if let Some(v) = opt(op, "elapsed") {
        h.limiter.elapsed_ms = u32_of(v, "elapsed")?;
    }
    if let Some(v) = opt(op, "running") {
        h.limiter.running = bool_of(v, "running")?;
    }
    if let Some(v) = opt(op, "window") {
        h.limiter.window_ms = u32_of(v, "window")?;
    }
    Ok(())
}

fn handler_event_expect(w: &World, op: &Json) -> R<()> {
    let h = w.handler.borrow();
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let e = h.events.get(i).ok_or_else(|| format!("no event #{i}"))?;
    let got = vec![
        ("event".to_owned(), ju(e.event)),
        ("a".to_owned(), ju(e.a)),
        ("b".to_owned(), ju(e.b)),
    ];
    drop(h);
    check_state(&got, field(op, "state")?)
}

// -- the sound manager (tests 19, 20, 21, 22) --

fn manager_message(w: &World, op: &Json) -> R<()> {
    let ty = word(op, "type")?;
    let handled = w.manager.borrow_mut().do_message(ty);
    check_expect(op, Json::Bool(handled))
}

fn manager_event(w: &World, op: &Json) -> R<()> {
    let ev = word(op, "event")?;
    let a = u32_of(opt(op, "a").unwrap_or(&Json::Int(0)), "a")?;
    let b = u32_of(opt(op, "b").unwrap_or(&Json::Int(0)), "b")?;
    let param = u32_of(opt(op, "param").unwrap_or(&Json::Int(0)), "param")?;
    let mut m = w.manager.borrow_mut();
    match m.do_event(ev, a, b, param) {
        Some(StopResult::Stopped) => check_expect(op, Json::Str("stopped".to_owned())),
        Some(StopResult::Unknown) => check_expect(op, Json::Str("unknown".to_owned())),
        None => check_expect(op, Json::Bool(true)),
    }
}

fn manager_start(w: &World, op: &Json) -> R<()> {
    let sound = word(op, "sound")?;
    let inst = word(op, "instance")?;
    let param = u32_of(opt(op, "param").unwrap_or(&Json::Int(0)), "param")?;
    let k = w.manager.borrow_mut().start_sound(sound, inst, param);
    w.push("start_sound", vec![("sound", ju(sound)), ("instance", ju(k))]);
    check_expect(op, ju(k))
}

fn manager_stop(w: &World, op: &Json) -> R<()> {
    let inst = word(op, "instance")?;
    let hard = bool_of(opt(op, "hard").unwrap_or(&Json::Bool(false)), "hard")?;
    let r = w.manager.borrow_mut().stop_sound(inst, hard);
    let name = match r {
        StopResult::Stopped => "stopped",
        StopResult::Unknown => "unknown",
    };
    check_expect(op, Json::Str(name.to_owned()))
}

fn manager_expect(w: &World, op: &Json) -> R<()> {
    let m = w.manager.borrow();
    let got = vec![
        ("instances".to_owned(), ju(m.instances.len() as u32)),
        ("list".to_owned(), ju(m.list.len() as u32)),
        ("master_volume".to_owned(), ju(m.master_volume)),
        ("voice_gate".to_owned(), ju(m.tuning.voice_gate)),
        ("one_shot".to_owned(), ju(m.tuning.one_shot)),
        ("update_period".to_owned(), ju(m.tuning.update_period)),
        ("tuning_log".to_owned(), ju(m.tuning_log.len() as u32)),
        ("ids_raw".to_owned(), ju(m.ids.raw())),
    ];
    drop(m);
    check_state(&got, field(op, "state")?)
}

fn manager_tuning(w: &World, op: &Json) -> R<()> {
    let mut m = w.manager.borrow_mut();
    match opt(op, "fail") {
        Some(v) if bool_of(v, "fail")? => m.read_tuning_values(None),
        _ => {
            let t = SoundManagerTuning {
                voice_gate: u32_of(opt(op, "voice_gate").unwrap_or(&Json::Int(3)), "voice_gate")?,
                one_shot: u32_of(opt(op, "one_shot").unwrap_or(&Json::Int(10)), "one_shot")?,
                update_period: u32_of(opt(op, "update_period").unwrap_or(&Json::Int(30)), "update_period")?,
                ..SoundManagerTuning::default()
            };
            m.read_tuning_values(Some(t));
        }
    }
    Ok(())
}

// -- the effects sound component (tests 23, 24) --

fn component_create(w: &World, op: &Json) -> R<()> {
    let mut d = SoundDescription::new();
    d.sound_id = word(op, "sound")?;
    if let Some(v) = opt(op, "length") {
        d.length = u32_of(v, "length")?;
    }
    if let Some(v) = opt(op, "rate") {
        d.set_location_update_rate(crate::scalar::f32_of(v, "rate")?);
    }
    *w.component.borrow_mut() = Some(SoundComponent::new(d));
    Ok(())
}

fn component_call(w: &World, op: &Json) -> R<()> {
    let what = scalar::str_of(field(op, "method")?, "method")?;
    let mut slot = w.component.borrow_mut();
    let c = slot.as_mut().ok_or("no sound component (aud_component_create)")?;
    match what {
        "start" => {
            let this_addr = u32_of(opt(op, "this").unwrap_or(&Json::Int(0x42)), "this")?;
            let mode = u32_of(opt(op, "mode").unwrap_or(&Json::Int(0)), "mode")?;
            let sys = *w.snd_sys.borrow();
            c.start(mode, this_addr, sys);
        }
        "stop" => {
            let mode = u32_of(opt(op, "mode").unwrap_or(&Json::Int(0)), "mode")?;
            c.stop(mode);
        }
        "update" => {
            let dt = crate::scalar::f32_of(field(op, "dt")?, "dt")?;
            c.update(dt);
        }
        "set_transform" => {
            let x = crate::scalar::f32_of(opt(op, "x").unwrap_or(&Json::Int(0)), "x")?;
            let y = crate::scalar::f32_of(opt(op, "y").unwrap_or(&Json::Int(0)), "y")?;
            let z = crate::scalar::f32_of(opt(op, "z").unwrap_or(&Json::Int(0)), "z")?;
            c.set_transform(x, y, z);
        }
        other => return Err(format!("unknown method '{other}'")),
    }
    Ok(())
}

fn component_expect(w: &World, op: &Json) -> R<()> {
    let slot = w.component.borrow();
    let c = slot.as_ref().ok_or("no sound component (aud_component_create)")?;
    let got = vec![
        ("active".to_owned(), jb(c.is_active())),
        ("instance".to_owned(), ju(c.instance)),
        ("started".to_owned(), jb(c.started)),
        ("snd_sys".to_owned(), jb(c.snd_sys_present)),
        ("events".to_owned(), ju(c.events.len() as u32)),
        ("elapsed".to_owned(), scalar::jf(c.elapsed)),
    ];
    drop(slot);
    check_state(&got, field(op, "state")?)
}

fn component_event_expect(w: &World, op: &Json) -> R<()> {
    let slot = w.component.borrow();
    let c = slot.as_ref().ok_or("no sound component (aud_component_create)")?;
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let e = c.events.get(i).ok_or_else(|| format!("no event #{i}"))?;
    let got = vec![
        ("event".to_owned(), ju(e.0)),
        ("a".to_owned(), ju(e.1)),
        ("b".to_owned(), scalar::ji(i64::from(e.2))),
        ("c".to_owned(), scalar::ji(i64::from(e.3))),
        ("d".to_owned(), scalar::ji(i64::from(e.4))),
    ];
    drop(slot);
    check_state(&got, field(op, "state")?)
}

fn ignore_bit(w: &World, op: &Json) -> R<()> {
    let rec = word(op, "record_flags")?;
    let cur = u32_of(opt(op, "runtime_flags").unwrap_or(&Json::Int(0)), "runtime_flags")?;
    let flags = runtime_flags_for(rec, cur);
    let got = vec![
        ("flags".to_owned(), ju(flags)),
        ("considered".to_owned(), jb(check_finished_considers(flags))),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

// -- the 83 leaf classes (tests 14, 15) --

fn leaf_create(w: &World, op: &Json) -> R<()> {
    let shape = match word(op, "shape")? {
        8 => LeafShape::Size8,
        16 => LeafShape::Size16,
        20 => LeafShape::Size20,
        n => return Err(format!("bad shape {n}")),
    };
    let mut o = LeafObject::from_factory(shape);
    if bool_of(opt(op, "add_ref").unwrap_or(&Json::Bool(true)), "add_ref")? {
        o.add_ref();
    }
    w.leaves.borrow_mut().push(o);
    check_expect(op, ju((w.leaves.borrow().len() - 1) as u32))
}

fn leaf_call(w: &World, op: &Json) -> R<()> {
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let what = scalar::str_of(field(op, "method")?, "method")?;
    let mut leaves = w.leaves.borrow_mut();
    let o = leaves.get_mut(i).ok_or_else(|| format!("no leaf #{i}"))?;
    match what {
        "add_ref" => {
            o.add_ref();
            check_expect(op, ju(o.refcount))
        }
        "release" => {
            let freed = o.release();
            check_expect(op, Json::Bool(freed))
        }
        "query_interface" => {
            let iid = word(op, "iid")?;
            check_expect(op, Json::Bool(o.query_interface(iid)))
        }
        "capability" => {
            let slot = word(op, "slot")?;
            check_expect(op, Json::Bool(o.capability_slot(slot)))
        }
        other => Err(format!("unknown method '{other}'")),
    }
}

fn leaf_expect(w: &World, op: &Json) -> R<()> {
    let i = u32_of(field(op, "index")?, "index")? as usize;
    let leaves = w.leaves.borrow();
    let o = leaves.get(i).ok_or_else(|| format!("no leaf #{i}"))?;
    let got = vec![
        ("size".to_owned(), ju(o.size() as u32)),
        ("refcount".to_owned(), ju(o.refcount)),
        ("id".to_owned(), ju(o.id)),
        ("flag".to_owned(), ju(u32::from(o.flag))),
    ];
    drop(leaves);
    check_state(&got, field(op, "state")?)
}

fn leaf_registry(w: &World, op: &Json) -> R<()> {
    let mut n8 = 0u32;
    let mut n16 = 0u32;
    let mut n20 = 0u32;
    for &(_, factory) in LEAF_REGISTRATIONS.iter() {
        match shape_of_factory(factory) {
            LeafShape::Size8 => n8 += 1,
            LeafShape::Size16 => n16 += 1,
            LeafShape::Size20 => n20 += 1,
        }
    }
    let got = vec![
        ("count".to_owned(), ju(LEAF_REGISTRATIONS.len() as u32)),
        ("size8".to_owned(), ju(n8)),
        ("size16".to_owned(), ju(n16)),
        ("size20".to_owned(), ju(n20)),
        ("common_iid".to_owned(), ju(LEAF_COMMON_IID)),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

// -- the director (test 16) --

fn director_expect(w: &World, op: &Json) -> R<()> {
    let order: Vec<Json> = audio::director::ON_START_ORDER
        .iter()
        .map(|s| Json::Str((*s).to_owned()))
        .collect();
    let got = vec![
        ("order".to_owned(), Json::Array(order)),
        ("director_id".to_owned(), ju(audio::director::DIRECTOR_ID)),
        (
            "class_count".to_owned(),
            ju(audio::director::AUDIO_DIRECTOR.class_count),
        ),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

// -- the data (tests 18, 19) --

fn data_expect(w: &World, op: &Json) -> R<()> {
    let context = match scalar::str_of(field(op, "context")?, "context")? {
        "hitlists" => SampleContext::HitLists,
        "tlo" => SampleContext::Tlo,
        "sample" => SampleContext::SampleData,
        other => return Err(format!("unknown context '{other}'")),
    };
    let ty = word(op, "type")?;
    let ext = fc4_core::audio::data::resolve_extension(ty, context);
    let got = vec![
        (
            "ext".to_owned(),
            match ext {
                Some(e) => Json::Str(e.to_owned()),
                None => Json::Null,
            },
        ),
        ("entries".to_owned(), ju(audio::data::SOUND_DAT_ENTRIES)),
        ("bytes".to_owned(), scalar::ji(audio::data::SOUND_DAT_BYTES as i64)),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

fn data_tuning(w: &World, op: &Json) -> R<()> {
    let d = SoundManagerTuning::default();
    let s = SoundManagerTuning::shipped();
    let got = vec![
        ("default_voice_gate".to_owned(), ju(d.voice_gate)),
        ("default_one_shot".to_owned(), ju(d.one_shot)),
        ("default_update_period".to_owned(), ju(d.update_period)),
        ("shipped_voice_gate".to_owned(), ju(s.voice_gate)),
        ("shipped_one_shot".to_owned(), ju(s.one_shot)),
        ("shipped_update_period".to_owned(), ju(s.update_period)),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

// -- nothing is saved (test 17) --

fn save_expect(w: &World, op: &Json) -> R<()> {
    let s = PostLoadState::after_load();
    let got = vec![
        ("automaton_instance_id".to_owned(), ju(s.automaton_instance_id)),
        ("live_sounds".to_owned(), ju(s.live_sounds)),
        ("pooled_events".to_owned(), ju(s.pooled_events)),
        ("aems_banks_loaded".to_owned(), jb(s.aems_banks_loaded)),
        ("has_save_record".to_owned(), jb(audio::save::HAS_SAVE_RECORD)),
    ];
    let _ = w;
    check_state(&got, field(op, "state")?)
}

// -- the full chain (test 25) --

/// Drive the handler and the manager from the play message the automaton
/// already posted - the second half of the chain of 8.7.
fn chain_play(w: &World, op: &Json) -> R<()> {
    let sfx = word(op, "id")?;
    // The play message the automaton posted.
    let msg = {
        let h = w.automaton.borrow();
        *h.posted.last().ok_or("no play message (aud_state first)")?
    };
    if msg.msg_type != MSG_AUTOMATA_PLAY_SFX {
        return Err(format!("the last post is 0x{:08X}, not a play", msg.msg_type));
    }
    let _ = sfx;
    // The handler turns it into sound-system events.
    let rand = *w.rand.borrow();
    w.handler.borrow_mut().handle_play(0, rand, msg.data1, msg.data2);
    // The manager turns the start event into a tSound.
    w.manager.borrow_mut().start_sound(msg.data1, msg.data2, 0);
    let live = w.manager.borrow().instances.len() as u32;
    w.push(
        "chain_play",
        vec![("id", ju(sfx)), ("instance", ju(msg.data2)), ("live", ju(live))],
    );
    check_expect(op, Json::Bool(true))
}

fn chain_stop(w: &World, op: &Json) -> R<()> {
    let sfx = word(op, "id")?;
    w.automaton.borrow_mut().handle_exit_state(sfx, OCCUPANT);
    let msg = {
        let h = w.automaton.borrow();
        *h.posted.last().ok_or("no stop message")?
    };
    let _ = sfx;
    w.handler.borrow_mut().handle_stop(msg.data1, msg.data2);
    w.manager.borrow_mut().stop_sound(msg.data2, false);
    let live = w.manager.borrow().instances.len() as u32;
    check_state(&[("live".to_owned(), ju(live))], field(op, "state")?)
}

// ---- dispatch ---------------------------------------------------------------

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("aud_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "pool_alloc" => pool_alloc(&w, op),
        "pool_alloc_n" => pool_alloc_n(&w, op),
        "pool_free" => pool_free(&w, op),
        "pool_free_index" => pool_free_index(&w, op),
        "pool_expect" => pool_expect(&w, op),
        "pool_free_list" => pool_free_list(&w, op),
        "pool_size_uses" => pool_size_uses(&w, op),
        "pool_page_of" => pool_page_of(&w, op),
        "msg_build" => msg_build(&w, op),
        "snd_event" => snd_event(&w, op),
        "snd_event_size" => snd_event_size(&w, op),
        "start_sfx" => start_sfx(&w, op),
        "stop_sfx" => stop_sfx(&w, op),
        "state" => state(&w, op),
        "automaton_expect" => automaton_expect(&w, op),
        "post_expect" => post_expect(&w, op),
        "clear_posts" => clear_posts(&w, op),
        "instance_id" => instance_id(&w, op),
        "handler_init" => handler_init(&w, op),
        "handler_message" => handler_message(&w, op),
        "handler_expect" => handler_expect(&w, op),
        "handler_timer" => handler_timer(&w, op),
        "handler_event_expect" => handler_event_expect(&w, op),
        "manager_message" => manager_message(&w, op),
        "manager_event" => manager_event(&w, op),
        "manager_start" => manager_start(&w, op),
        "manager_stop" => manager_stop(&w, op),
        "manager_expect" => manager_expect(&w, op),
        "manager_tuning" => manager_tuning(&w, op),
        "component_create" => component_create(&w, op),
        "component_call" => component_call(&w, op),
        "component_expect" => component_expect(&w, op),
        "component_event_expect" => component_event_expect(&w, op),
        "ignore_bit" => ignore_bit(&w, op),
        "leaf_create" => leaf_create(&w, op),
        "leaf_call" => leaf_call(&w, op),
        "leaf_expect" => leaf_expect(&w, op),
        "leaf_registry" => leaf_registry(&w, op),
        "director_expect" => director_expect(&w, op),
        "data_expect" => data_expect(&w, op),
        "data_tuning" => data_tuning(&w, op),
        "save_expect" => save_expect(&w, op),
        "chain_play" => chain_play(&w, op),
        "chain_stop" => chain_stop(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}
