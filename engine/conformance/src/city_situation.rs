// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/city-situation.md`: a scripted game
//! (city-situation advices, automata, the preferences' reward list, the
//! budget's funds, the popup's button, the mission's `CheckForSuccess`)
//! and ops that drive the city situation manager. See README.md ("City
//! situation ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::city_situation::{
    msg, query_interface, AdviceInfo, CitySituationHost, CitySituationManager, CitySituationState, Popup, Script,
    SituationMessage, CLSID_CITY_SITUATION, MISSION_TYPE_NO_TARGET, REWARD_SLOTS,
};
use fc4_core::stream::{ByteReader, ByteWriter, SaveObject};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

const MANAGER_TOKEN: u64 = 0xC5;
const AUTOMATA_MANAGERS: [u64; 4] = [0x43, 0x45, 0x44, 0x46];

#[derive(Clone, Copy, Debug, Default)]
struct Auto {
    flags: u32,
    atype: u32,
    pos: (f32, f32),
    preview: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Advice {
    info: AdviceInfo,
    target: Option<u64>,
    target_count: u32,
    time_limit: f32,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    mgr: RefCell<Option<Rc<CitySituationManager>>>,
    session_seed: Cell<u32>,
    refs: RefCell<BTreeMap<u64, i64>>,
    debug: Cell<bool>,
    advices: RefCell<BTreeMap<u64, Advice>>,
    retired: RefCell<BTreeSet<u64>>,
    autos: RefCell<BTreeMap<u64, Auto>>,
    candidates: RefCell<BTreeMap<u64, Vec<u64>>>,
    /// (advice, automaton) pairs advice slot 53 accepts.
    accepts: RefCell<BTreeSet<(u64, u64)>>,
    rewards: RefCell<Vec<u32>>,
    status_answer: Cell<u32>,
    button: Cell<u32>,
    funds: Cell<i64>,
    next_handle: Cell<u32>,
    record: RefCell<Vec<u8>>,
    mark: RefCell<Option<(CitySituationState, u32)>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            mgr: RefCell::new(None),
            session_seed: Cell::new(0x1234_5678),
            refs: RefCell::new(BTreeMap::new()),
            debug: Cell::new(false),
            advices: RefCell::new(BTreeMap::new()),
            retired: RefCell::new(BTreeSet::new()),
            autos: RefCell::new(BTreeMap::new()),
            candidates: RefCell::new(BTreeMap::new()),
            accepts: RefCell::new(BTreeSet::new()),
            rewards: RefCell::new(vec![0; REWARD_SLOTS]),
            status_answer: Cell::new(3),
            button: Cell::new(0),
            funds: Cell::new(100_000),
            next_handle: Cell::new(1),
            record: RefCell::new(Vec::new()),
            mark: RefCell::new(None),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn mgr(&self) -> R<Rc<CitySituationManager>> {
        self.mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no city situation manager (citysit_create)".into())
    }

    fn new_mgr(&self) -> Rc<CitySituationManager> {
        let m = CitySituationManager::new(
            self.rc() as Rc<dyn CitySituationHost>,
            MANAGER_TOKEN,
            self.session_seed.get(),
        );
        *self.mgr.borrow_mut() = Some(m.clone());
        m
    }

    fn ev(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }

    fn advice(&self, a: u64) -> Advice {
        self.advices.borrow().get(&a).copied().unwrap_or_default()
    }
}

fn tok(t: u64) -> Json {
    ji(t as i64)
}

fn opt_tok(t: Option<u64>) -> Json {
    t.map_or(Json::Null, tok)
}

fn service(w: &World, kind: &str, name: &str) {
    w.ev(kind, vec![("service", Json::Str(name.into()))]);
}

impl CitySituationHost for World {
    fn debug_enabled(&self) -> bool {
        self.debug.get()
    }
    fn register_cheat(&self) -> bool {
        self.ev("register_cheat", vec![]);
        true
    }
    fn unregister_cheat(&self) {
        self.ev("unregister_cheat", vec![]);
    }
    fn subscribe(&self, m: u32) {
        self.ev("subscribe", vec![("type", ji(m))]);
    }
    fn unsubscribe(&self, m: u32) {
        self.ev("unsubscribe", vec![("type", ji(m))]);
    }
    fn acquire_indicator_view(&self) -> bool {
        service(self, "acquire", "indicator_view");
        true
    }
    fn release_indicator_view(&self) {
        service(self, "release_service", "indicator_view");
    }
    fn register_animation_tick(&self) {
        self.ev("register_tick", vec![]);
    }
    fn unregister_animation_tick(&self) {
        self.ev("unregister_tick", vec![]);
    }
    fn acquire_automata_managers(&self) -> Vec<u64> {
        for m in AUTOMATA_MANAGERS {
            *self.refs.borrow_mut().entry(m).or_insert(0) += 1;
        }
        self.ev(
            "acquire_automata_managers",
            vec![(
                "managers",
                Json::Array(AUTOMATA_MANAGERS.iter().map(|&m| tok(m)).collect()),
            )],
        );
        AUTOMATA_MANAGERS.to_vec()
    }
    fn acquire_lot_manager(&self) -> bool {
        service(self, "acquire", "lot_manager");
        true
    }
    fn release_lot_manager(&self) {
        service(self, "release_service", "lot_manager");
    }
    fn acquire_mysim(&self) -> bool {
        service(self, "acquire", "mysim");
        true
    }
    fn release_mysim(&self) {
        service(self, "release_service", "mysim");
    }
    fn acquire_lua(&self) -> bool {
        service(self, "acquire", "lua");
        true
    }
    fn release_lua(&self) {
        service(self, "release_service", "lua");
    }
    fn create_lua_table(&self) {
        self.ev("create_lua_table", vec![]);
    }
    fn remove_lua_table(&self) {
        self.ev("remove_lua_table", vec![]);
    }
    fn release_mission_resources(&self) {
        self.ev("release_mission_resources", vec![]);
    }
    fn show_all_indicators(&self, kind: u32, visible: bool) {
        self.ev(
            "show_all_indicators",
            vec![("kind", ji(kind)), ("visible", Json::Bool(visible))],
        );
    }
    fn preferences_rewards(&self) -> Vec<u32> {
        self.rewards.borrow().clone()
    }
    fn set_preferences_reward(&self, i: usize, g: u32) {
        if let Some(e) = self.rewards.borrow_mut().get_mut(i) {
            *e = g;
        }
    }
    fn save_preferences(&self) {
        self.ev("save_preferences", vec![]);
    }
    fn add_ref(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += 1;
    }
    fn release(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) -= 1;
    }
    fn is_situation_advice(&self, o: u64) -> bool {
        self.advices.borrow().contains_key(&o)
    }
    fn advisor_advices(&self) -> Vec<u64> {
        self.advices.borrow().keys().copied().collect()
    }
    fn find_advice(&self, guid: u32) -> Option<u64> {
        self.advices
            .borrow()
            .iter()
            .find(|(_, a)| a.info.guid == guid)
            .map(|(&t, _)| t)
    }
    fn advice_info(&self, a: u64) -> AdviceInfo {
        self.advice(a).info
    }
    fn retire_advice(&self, a: u64) {
        self.retired.borrow_mut().insert(a);
        self.ev("retire", vec![("advice", tok(a))]);
    }
    fn run_script(&self, a: u64, s: Script) {
        let name = match s {
            Script::Start => "start",
            Script::Success => "success",
            Script::Failure => "failure",
        };
        self.ev("script", vec![("advice", tok(a)), ("script", Json::Str(name.into()))]);
    }
    fn time_limit(&self, a: u64) -> f32 {
        self.advice(a).time_limit
    }
    fn bind_mission_script(&self, a: u64, v: Option<u64>) {
        self.ev("bind", vec![("advice", tok(a)), ("vehicle", opt_tok(v))]);
    }
    fn build_coverage_cells(&self, a: u64) {
        self.ev("coverage", vec![("advice", tok(a))]);
    }
    fn first_target(&self, a: u64) -> Option<u64> {
        self.advice(a).target
    }
    fn target_count(&self, a: u64) -> u32 {
        self.advice(a).target_count
    }
    fn target_name(&self, _a: Option<u64>, t: Option<u64>) -> String {
        t.map_or_else(String::new, |t| format!("target 0x{t:X}"))
    }
    fn advice_accepts_automaton(&self, a: u64, v: u64) -> bool {
        let ok = self.accepts.borrow().contains(&(a, v));
        self.ev(
            "advice_accepts",
            vec![("advice", tok(a)), ("automaton", tok(v)), ("accepted", Json::Bool(ok))],
        );
        ok
    }
    fn is_automaton(&self, o: u64) -> bool {
        self.autos.borrow().contains_key(&o)
    }
    fn occupant_has_property(&self, o: u64, _id: u32) -> bool {
        self.autos.borrow().get(&o).is_some_and(|a| a.preview)
    }
    fn candidate_vehicles(&self, a: u64) -> Vec<u64> {
        self.candidates.borrow().get(&a).cloned().unwrap_or_default()
    }
    fn automaton_flags(&self, a: u64) -> u32 {
        self.autos.borrow().get(&a).map_or(0, |x| x.flags)
    }
    fn automaton_type(&self, a: u64) -> u32 {
        self.autos.borrow().get(&a).map_or(0, |x| x.atype)
    }
    fn track_indicator(&self, a: u64) -> u32 {
        let h = self.next_handle.get();
        self.next_handle.set(h + 1);
        self.ev("track", vec![("automaton", tok(a)), ("handle", ji(h))]);
        h
    }
    fn untrack_indicator(&self, h: u32) {
        self.ev("untrack", vec![("handle", ji(h))]);
    }
    fn add_listener(&self, a: u64) {
        self.ev("listen", vec![("automaton", tok(a))]);
    }
    fn remove_listener(&self, a: u64) {
        self.ev("unlisten", vec![("automaton", tok(a))]);
    }
    fn freeze_automata(&self, m: u64, on: bool) {
        self.ev("freeze", vec![("manager", tok(m)), ("on", Json::Bool(on))]);
    }
    fn play_sound(&self, a: u64, s: u32) {
        self.ev("sound", vec![("automaton", tok(a)), ("sound", ji(s))]);
    }
    fn position(&self, o: u64) -> (f32, f32) {
        self.autos.borrow().get(&o).map_or((0.0, 0.0), |x| x.pos)
    }
    fn check_for_success(&self, _a: u64, _t: f32) -> u32 {
        self.status_answer.get()
    }
    fn run_pursuit(&self, _a: u64, _t: f32) {}
    fn start_countdown_effect(&self) {
        self.ev("countdown_start", vec![]);
    }
    fn end_countdown_effect(&self) {
        self.ev("countdown_end", vec![]);
    }
    fn outcome_effect(&self, a: u64, success: bool) {
        self.ev(
            "outcome_effect",
            vec![("advice", tok(a)), ("success", Json::Bool(success))],
        );
    }
    fn player_drive_mode(&self, v: Option<u64>, on: bool) {
        self.ev("drive", vec![("vehicle", opt_tok(v)), ("on", Json::Bool(on))]);
    }
    fn start_joyride(&self, v: Option<u64>) {
        self.ev("joyride", vec![("vehicle", opt_tok(v))]);
    }
    fn add_transient_effect(&self, x: f32, z: f32, r: f32, m: f32) {
        self.ev(
            "aura",
            vec![("x", jf(x)), ("z", jf(z)), ("radius", jf(r)), ("magnitude", jf(m))],
        );
    }
    /// budget.md §6: a deposit always succeeds; a withdrawal that would
    /// leave the funds below zero is refused.
    fn deposit_funds(&self, amount: i64) {
        let f = self.funds.get();
        let ok = amount >= 0 || f + amount >= 0;
        if ok {
            self.funds.set(f + amount);
        }
        self.ev("deposit", vec![("amount", ji(amount)), ("accepted", Json::Bool(ok))]);
    }
    fn send_activation(&self, a: u64, on: bool) {
        self.ev(
            "send",
            vec![
                ("type", ji(msg::ACTIVATION)),
                ("advice", tok(a)),
                ("data1", ji(u32::from(on))),
            ],
        );
    }
    fn post_show_popup(&self, a: Option<u64>, v: Option<u64>) {
        self.ev(
            "post",
            vec![
                ("type", ji(msg::SHOW_POPUP)),
                ("advice", opt_tok(a)),
                ("vehicle", opt_tok(v)),
            ],
        );
    }
    fn post_popup_closed(&self, a: u64) {
        self.ev("post", vec![("type", ji(msg::POPUP_CLOSED)), ("advice", tok(a))]);
    }
    fn post_initiate(&self, g: u32, b: bool) {
        self.ev(
            "post",
            vec![
                ("type", ji(msg::INITIATE_BY_GUID)),
                ("data1", ji(g)),
                ("data2", ji(u32::from(b))),
            ],
        );
    }
    fn hide_mission_status_panel(&self) {
        self.ev("hide_mission_status_panel", vec![]);
    }
    fn end_mysim_dialog(&self) {
        self.ev("end_mysim_dialog", vec![]);
    }
    fn show_popup(&self, p: Popup, a: Option<u64>, v: Option<u64>) -> u32 {
        let name = match p {
            Popup::Briefing => "briefing",
            Popup::Success => "success",
            Popup::Failure => "failure",
        };
        self.ev(
            "popup",
            vec![
                ("popup", Json::Str(name.into())),
                ("advice", opt_tok(a)),
                ("vehicle", opt_tok(v)),
            ],
        );
        self.button.get()
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "city_situation", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("citysit_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "advice" => advice(&w, op),
        "automaton" => automaton(&w, op),
        "create" => {
            if let Some(v) = opt(op, "session_seed") {
                match u32_of(v, "session_seed") {
                    Ok(s) => w.session_seed.set(s),
                    Err(e) => return Some(Err(e)),
                }
            }
            w.new_mgr();
            Ok(())
        }
        "init" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.init()))),
        "shutdown" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.shutdown()))),
        "add_ref" => w.mgr().and_then(|m| check_expect(op, ji(m.add_ref()))),
        "release" => w.mgr().and_then(|m| check_expect(op, ji(m.release()))),
        "identity" => identity(&w, op),
        "message" => message(&w, op),
        "ticks" => ticks(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "mark" => w.mgr().map(|m| {
            *w.mark.borrow_mut() = Some((m.snapshot(), m.peek_random()));
        }),
        "expect_unchanged" => expect_unchanged(&w),
        "expect" => expect(&w, op),
        "expect_world" => expect_world(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "write" => write(&w, op),
        "read" => read(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn pair_of(j: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be a pair"));
    }
    Ok((f32_of(&a[0], what)?, f32_of(&a[1], what)?))
}

/// `citysit_env`: `debug` (the debug functionality flag), `rewards` (the preferences'
/// reward list from its start; the rest is zero), `status` (what `CheckForSuccess`
/// answers, default 3), `button` (the button that closes the popup, default 0), `funds`
/// (default 100,000), `candidates` (`{"advice": [automata]}`).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "debug") {
        w.debug.set(bool_of(v, "debug")?);
    }
    if let Some(v) = opt(op, "rewards") {
        let mut r = vec![0u32; REWARD_SLOTS];
        for (i, g) in arr(v, "rewards")?.iter().enumerate().take(REWARD_SLOTS) {
            r[i] = u32_of(g, "rewards")?;
        }
        *w.rewards.borrow_mut() = r;
    }
    if let Some(v) = opt(op, "status") {
        w.status_answer.set(u32_of(v, "status")?);
    }
    if let Some(v) = opt(op, "button") {
        w.button.set(u32_of(v, "button")?);
    }
    if let Some(v) = opt(op, "funds") {
        w.funds.set(i64_of(v, "funds")?);
    }
    if let Some(v) = opt(op, "candidates") {
        for (k, list) in v.as_object().ok_or("candidates must be an object")? {
            let a = token_of(&Json::Str(k.clone()), "advice")?;
            let l = arr(list, "candidates")?
                .iter()
                .map(|e| token_of(e, "candidates"))
                .collect::<R<Vec<_>>>()?;
            w.candidates.borrow_mut().insert(a, l);
        }
    }
    Ok(())
}

/// `citysit_advice`: a city-situation advice `advice` with `guid`, `flags`,
/// `mission_type` (default 2, no target), `end_delay`, `success_money`, `failure_money`,
/// `success_aura` / `failure_aura` ([radius, magnitude]), `target` (the first target a
/// target mission finds; absent = none), `target_count`, `time_limit` (seconds).
fn advice(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "advice")?, "advice")?;
    let mut a = w.advices.borrow().get(&t).copied().unwrap_or(Advice {
        info: AdviceInfo {
            mission_type: MISSION_TYPE_NO_TARGET,
            ..AdviceInfo::default()
        },
        ..Advice::default()
    });
    if let Some(v) = opt(op, "guid") {
        a.info.guid = u32_of(v, "guid")?;
    }
    if let Some(v) = opt(op, "flags") {
        a.info.flags = u32_of(v, "flags")?;
    }
    if let Some(v) = opt(op, "mission_type") {
        a.info.mission_type = u32_of(v, "mission_type")?;
    }
    if let Some(v) = opt(op, "end_delay") {
        a.info.end_delay = f32_of(v, "end_delay")?;
    }
    if let Some(v) = opt(op, "success_money") {
        a.info.success_money = i64_of(v, "success_money")? as i32;
    }
    if let Some(v) = opt(op, "failure_money") {
        a.info.failure_money = i64_of(v, "failure_money")? as i32;
    }
    if let Some(v) = opt(op, "success_aura") {
        a.info.success_aura = pair_of(v, "success_aura")?;
    }
    if let Some(v) = opt(op, "failure_aura") {
        a.info.failure_aura = pair_of(v, "failure_aura")?;
    }
    if let Some(v) = op.get("target") {
        a.target = opt(op, "target").map(|_| token_of(v, "target")).transpose()?;
    }
    if let Some(v) = opt(op, "target_count") {
        a.target_count = u32_of(v, "target_count")?;
    }
    if let Some(v) = opt(op, "time_limit") {
        a.time_limit = f32_of(v, "time_limit")?;
    }
    if let Some(v) = opt(op, "accepts") {
        let mut acc = w.accepts.borrow_mut();
        acc.retain(|&(x, _)| x != t);
        for e in arr(v, "accepts")? {
            acc.insert((t, token_of(e, "accepts")?));
        }
    }
    w.advices.borrow_mut().insert(t, a);
    Ok(())
}

/// `citysit_automaton`: `automaton` with `flags` (default 2, drivable), `type` (default 1),
/// `position` ([x, z]), `preview` (property 0x029244DB present). `remove: true` forgets it.
fn automaton(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "automaton")?, "automaton")?;
    if opt(op, "remove").and_then(boolean).unwrap_or(false) {
        w.autos.borrow_mut().remove(&t);
        return Ok(());
    }
    let mut a = w.autos.borrow().get(&t).copied().unwrap_or(Auto {
        flags: 2,
        atype: 1,
        ..Auto::default()
    });
    if let Some(v) = opt(op, "flags") {
        a.flags = u32_of(v, "flags")?;
    }
    if let Some(v) = opt(op, "type") {
        a.atype = u32_of(v, "type")?;
    }
    if let Some(v) = opt(op, "position") {
        a.pos = pair_of(v, "position")?;
    }
    if let Some(v) = opt(op, "preview") {
        a.preview = bool_of(v, "preview")?;
    }
    w.autos.borrow_mut().insert(t, a);
    Ok(())
}

/// `citysit_identity`: `iids` (`{"0xIID": offset or null}`) and `class`.
fn identity(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "iids") {
        for (k, want) in v.as_object().ok_or("iids must be an object")? {
            let iid = u32_of(&Json::Str(k.clone()), "iid")?;
            let got = query_interface(iid).map_or(Json::Null, ji);
            if !scalar::matches(&got, want, false) {
                return Err(format!("QueryInterface({k}) = {}", scalar::show(&got)));
            }
        }
    }
    if let Some(v) = opt(op, "class") {
        let want = u32_of(v, "class")?;
        let got = w.mgr().map_or(CLSID_CITY_SITUATION, |m| m.class_id());
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

fn type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("AdviceAdded") => msg::ADVICE_ADDED,
        Some("AdviceRemoved") => msg::ADVICE_REMOVED,
        Some("AdviceRemovedAll") => msg::ADVICE_REMOVED_ALL,
        Some("PopupClosed") => msg::POPUP_CLOSED,
        Some("ShowPopup") => msg::SHOW_POPUP,
        Some("Window") => msg::WINDOW,
        Some("InitiateByGuid") => msg::INITIATE_BY_GUID,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        Some("AnimationTick") => msg::ANIMATION_TICK,
        Some("AutomatonNotify") => msg::AUTOMATON_NOTIFY,
        Some("PlayerLeftVehicle") => msg::PLAYER_LEFT_VEHICLE,
        Some("Cheat") => msg::CHEAT,
        Some("PopupNoAdvice") => msg::POPUP_NO_ADVICE,
        _ => u32_of(j, "type")?,
    })
}

/// `citysit_message`: `type` (a name of README.md or an ID), `data1`, `data2`, `data3`
/// (or `ms`: data3 = the float bits of that many milliseconds), `object`, `void1`, `text`.
fn message(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let tok_opt = |k: &str| opt(op, k).map(|v| token_of(v, k)).transpose();
    let data3 = match opt(op, "ms") {
        Some(v) => f32_of(v, "ms")?.to_bits(),
        None => opt(op, "data3").map_or(Ok(0), |v| u32_of(v, "data3"))?,
    };
    let sm = SituationMessage {
        msg_type: type_of(field(op, "type")?)?,
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data2: opt(op, "data2").map_or(Ok(0), |v| u32_of(v, "data2"))?,
        data3,
        object: tok_opt("object")?,
        void1: tok_opt("void1")?,
        text: opt(op, "text")
            .map(|v| str_of(v, "text").map(str::to_owned))
            .transpose()?,
    };
    check_expect(op, Json::Bool(m.do_message(&sm)))
}

/// `citysit_ticks`: `count` animation-tick messages of `ms` milliseconds each.
fn ticks(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let n = i64_of(field(op, "count")?, "count")?;
    let ms = f32_of(field(op, "ms")?, "ms")?;
    let sm = SituationMessage {
        msg_type: msg::ANIMATION_TICK,
        data3: ms.to_bits(),
        ..SituationMessage::default()
    };
    for _ in 0..n {
        m.do_message(&sm);
    }
    Ok(())
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_bool(op: &Json, i: usize) -> R<bool> {
    bool_of(arg(op, i)?, "argument")
}

fn a_tok(op: &Json, i: usize) -> R<Option<u64>> {
    match arg(op, i)? {
        Json::Null => Ok(None),
        v => token_of(v, "argument").map(Some),
    }
}

/// `citysit_call`: `method` with `args`; `expect` checks the result (see README.md for the
/// method list).
fn call(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let got = match str_of(field(op, "method")?, "method")? {
        "initiate_situation" => Json::Bool(m.initiate_situation(
            a_tok(op, 0)?.ok_or("advice must not be null")?,
            a_tok(op, 1)?,
            a_bool(op, 2)?,
        )),
        "initiate_situation_by_guid" => {
            Json::Bool(m.initiate_situation_by_guid(a_u32(op, 0)?, a_bool(op, 1)?, a_u32(op, 2)?))
        }
        "initiate_situation_for_occupant" => Json::Bool(m.initiate_situation_for_occupant(
            a_tok(op, 0)?.ok_or("occupant must not be null")?,
            match op.get("args").and_then(Json::as_array).map_or(0, |a| a.len()) {
                0 | 1 => None,
                _ => a_tok(op, 1)?,
            },
        )),
        "activate_situation" => {
            m.activate_situation();
            Json::Null
        }
        "deactivate_situation" => {
            m.deactivate_situation();
            Json::Null
        }
        "end_situation" => {
            m.end_situation();
            Json::Null
        }
        "handle_status_changed" => {
            m.handle_status_changed(a_u32(op, 0)?);
            Json::Null
        }
        "get_mission_success_count" => ji(m.success_count(a_u32(op, 0)?)),
        "lua_activate_situation" => Json::Bool(m.lua_activate_situation(a_u32(op, 0)?, a_bool(op, 1)?)),
        "get_active_situation" => opt_tok(m.active_situation()),
        "get_current_status" => ji(m.current_status()),
        "get_active_auto" => opt_tok(m.active_auto()),
        "get_current_target" => opt_tok(m.current_target()),
        "get_current_target_index" => ji(m.current_target_index()),
        "get_num_targets" => ji(m.num_targets()),
        "get_num_targets_remaining" => ji(m.num_targets_remaining()),
        "get_current_time_limit_seconds" => jf(m.time_limit_seconds()),
        "get_current_situation_time" => jf(m.situation_time()),
        "get_current_time_remaining_seconds" => jf(m.time_remaining_seconds()),
        "get_current_target_name" => Json::Str(m.current_target_name()),
        "get_situation_aura" => m
            .situation_aura()
            .map_or(Json::Null, |(r, g)| Json::Array(vec![jf(r), jf(g)])),
        "get_situation_money" => m.situation_money().map_or(Json::Null, ji),
        "is_situation_available" => Json::Bool(m.is_situation_available(a_tok(op, 0)?.unwrap_or(0))),
        "set_max_csi" => {
            m.set_max_csi(a_u32(op, 0)? as u8);
            Json::Null
        }
        "get_max_csi" => ji(m.max_csi()),
        "get_tracked_automata_count" => ji(m.tracked_automata_count()),
        "set_csi_visible" => {
            m.set_csi_visible(a_bool(op, 0)?);
            Json::Null
        }
        "get_csi_visible" => Json::Bool(m.csi_visible()),
        "handle_tuning_parameters_changed" => Json::Bool(m.handle_tuning_parameters_changed()),
        "countdown_is_in_progress" => Json::Bool(m.countdown_in_progress()),
        "reset_tutorials" => {
            m.reset_tutorials();
            Json::Null
        }
        "initiate_player_drive_mode" => {
            m.initiate_player_drive_mode(a_bool(op, 0)?);
            Json::Null
        }
        "random_uniform_u32" => ji(m.random_uniform_u32(a_u32(op, 0)?)),
        other => return Err(format!("unknown city situation method '{other}'")),
    };
    check_expect(op, got)
}

/// `citysit_set` (a test hook): `success_counts` (`{"0xGUID": count}`, replaces the map),
/// `loaded_from_save`.
fn set(w: &World, op: &Json) -> R<()> {
    let counts = match opt(op, "success_counts") {
        Some(v) => {
            let mut m = BTreeMap::new();
            for (k, c) in v.as_object().ok_or("success_counts must be an object")? {
                m.insert(u32_of(&Json::Str(k.clone()), "guid")?, u32_of(c, k)? as u16);
            }
            Some(m)
        }
        None => None,
    };
    let loaded = opt(op, "loaded_from_save")
        .map(|v| bool_of(v, "loaded_from_save"))
        .transpose()?;
    w.mgr()?.with_state(|s| {
        if let Some(c) = counts {
            s.success_counts = c;
        }
        if let Some(l) = loaded {
            s.loaded_from_save = l;
        }
    });
    Ok(())
}

/// `citysit_expect_unchanged`: the state and the generator are as at `citysit_mark`.
fn expect_unchanged(w: &World) -> R<()> {
    let m = w.mgr()?;
    let mark = w.mark.borrow().clone().ok_or("no citysit_mark")?;
    if m.snapshot() != mark.0 {
        return Err("the state changed".into());
    }
    if m.peek_random() != mark.1 {
        return Err("the generator was drawn".into());
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.mgr()?.snapshot();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("ref_count".to_owned(), ji(s.ref_count)),
        ("indicators_visible".to_owned(), Json::Bool(s.indicators_visible)),
        ("loaded_from_save".to_owned(), Json::Bool(s.loaded_from_save)),
        ("active_situation".to_owned(), opt_tok(s.active_situation)),
        (
            "automata_managers".to_owned(),
            Json::Array(s.automata_managers.iter().map(|&t| tok(t)).collect()),
        ),
        (
            "track_list".to_owned(),
            Json::Array(
                s.track_list
                    .iter()
                    .map(|e| {
                        Json::Object(vec![
                            ("advice".to_owned(), tok(e.advice)),
                            ("automaton".to_owned(), opt_tok(e.automaton)),
                            ("handle".to_owned(), ji(e.handle)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "tracked".to_owned(),
            Json::Array(s.tracked.iter().map(|&t| tok(t)).collect()),
        ),
        (
            "success_counts".to_owned(),
            Json::Object(
                s.success_counts
                    .iter()
                    .map(|(&g, &c)| (format!("0x{g:08X}"), ji(c)))
                    .collect(),
            ),
        ),
        ("status".to_owned(), ji(s.status)),
        ("active_automaton".to_owned(), opt_tok(s.active_automaton)),
        ("current_target".to_owned(), opt_tok(s.current_target)),
        ("num_targets".to_owned(), ji(s.num_targets)),
        ("target_index".to_owned(), ji(s.target_index)),
        ("end_delay".to_owned(), jf(s.end_delay)),
        ("elapsed".to_owned(), jf(s.elapsed)),
        ("countdown".to_owned(), jf(s.countdown)),
        ("time_limit".to_owned(), jf(s.time_limit)),
        ("just_activated".to_owned(), Json::Bool(s.just_activated)),
        ("warning_given".to_owned(), Json::Bool(s.warning_given)),
        ("max_csi".to_owned(), ji(s.max_csi)),
        ("window".to_owned(), opt_tok(s.window)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `citysit_expect_world`: `funds`, `rewards` (the reward list up to its last non-zero
/// entry), `retired` (advices retired from advisor 29).
fn expect_world(w: &World, op: &Json) -> R<()> {
    let r = w.rewards.borrow();
    let last = r.iter().rposition(|&g| g != 0).map_or(0, |i| i + 1);
    let got = vec![
        ("funds".to_owned(), ji(w.funds.get())),
        (
            "rewards".to_owned(),
            Json::Array(r[..last].iter().map(|&g| ji(g)).collect()),
        ),
        (
            "retired".to_owned(),
            Json::Array(w.retired.borrow().iter().map(|&t| tok(t)).collect()),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `citysit_expect_refs`: `{"token": references}` taken by the manager.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = token_of(&Json::Str(k.clone()), "token")?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

/// `citysit_write`: `expect`, `hex`.
fn write(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let mut out = ByteWriter::new();
    check_expect(op, Json::Bool(m.write(&mut out)))?;
    let bytes = out.into_bytes();
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if bytes != want {
            return Err(format!("record {}, expected {}", hex_encode(&bytes), hex_encode(&want)));
        }
    }
    *w.record.borrow_mut() = bytes;
    Ok(())
}

/// `citysit_read`: `hex` (or the last written record) into a new constructed manager (or
/// the current one with `reuse`); `expect`; `consumed` (the bytes read).
fn read(w: &World, op: &Json) -> R<()> {
    let bytes = match opt(op, "hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => w.record.borrow().clone(),
    };
    let m = if opt(op, "reuse").and_then(boolean).unwrap_or(false) {
        w.mgr()?
    } else {
        w.new_mgr()
    };
    let mut r = ByteReader::new(&bytes);
    let ok = m.read(&mut r);
    if let Some(v) = opt(op, "consumed") {
        if r.position() as i64 != i64_of(v, "consumed")? {
            return Err(format!("read {} bytes", r.position()));
        }
    }
    check_expect(op, Json::Bool(ok))
}
