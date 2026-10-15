// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! [`ScriptedHost`]: an [`AdvisorHost`] whose world is plain data and which
//! records every side effect, for the unit tests, the conformance vectors
//! and the C ABI tests.
//!
//! Lua is replaced by literal trigger texts: a compiled trigger evaluates
//! its text through [`ScriptedHost::triggers`] (set by the test), else as a
//! literal (`true`, `false`, `nil`, a number, `"str"`, `error`).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::stream::RefValue;

use super::subject::AdviceSubject;
use super::{
    Advice, AdviceEntry, Advisor, AdvisorEntry, AdvisorHost, ConditionalStatus, Item, ListSrc, LuaResult, MsgObj,
    OutMessage, SimDate, Src, SubjectInit, SubjectState, Target, Val,
};

/// One recorded side effect.
#[derive(Clone, Debug, PartialEq)]
pub enum HostCall {
    Post {
        msg: u32,
        data1: u32,
        obj: String,
    },
    Send {
        msg: u32,
        data1: u32,
        obj: String,
    },
    UpdateSelf {
        advisor: i32,
        data1: u32,
        data2: u32,
    },
    Subscribe {
        target: String,
        msg: u32,
    },
    Unsubscribe {
        target: String,
        msg: u32,
    },
    Global {
        name: String,
        value: f64,
    },
    GlobalString {
        name: String,
        value: Vec<u8>,
    },
    Local {
        name: String,
        value: f64,
        subject: Option<SubjectState>,
    },
    Effect {
        id: u32,
        assert: bool,
    },
    Rebuild {
        guid: u32,
    },
    History {
        history_type: u32,
        value: i32,
    },
    Status {
        guid: u32,
        status: ConditionalStatus,
    },
    Cheat {
        id: u32,
        on: bool,
    },
    Window {
        start: bool,
    },
    Ticker,
    CollectGarbage,
    /// `register_api`, `reload_scripts`, `set_tutorial_lua`, `create_tags`,
    /// `register_detokenizer`, `close_lua`.
    Lua(String),
    CancelDeal {
        object: u64,
    },
    SetOrdinance {
        id: u32,
        on: bool,
    },
}

/// An occupant of the scripted city.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Occupant {
    pub occupant_type: u32,
    pub conditional: Option<bool>,
    pub key_instance: Option<u32>,
    pub groups: Vec<u32>,
}

/// An ordinance of the scripted city.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ordinance {
    pub on: bool,
    pub available: bool,
    /// Enact fee, repeal fee, monthly cost.
    pub amounts: [i64; 3],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
}

/// The scripted host.
pub struct ScriptedHost {
    /// `None`: the `advisors` global is not a table.
    pub advisors: RefCell<Option<Vec<AdvisorEntry>>>,
    pub advices: RefCell<Vec<AdviceEntry>>,
    /// Trigger text → result (overrides the literal reading).
    pub triggers: RefCell<BTreeMap<Vec<u8>, LuaResult>>,
    /// Texts that fail to compile.
    pub bad_triggers: RefCell<Vec<Vec<u8>>>,
    compiled: RefCell<BTreeMap<i32, Vec<u8>>>,
    next_ref: Cell<i32>,
    /// Released trigger references.
    pub released_triggers: RefCell<Vec<i32>>,
    /// The trigger texts called, in order.
    pub trigger_calls: RefCell<Vec<Vec<u8>>>,
    /// Called with the text before a trigger returns (re-entrancy tests).
    #[allow(clippy::type_complexity)]
    pub trigger_hook: RefCell<Option<Box<dyn Fn(&[u8])>>>,
    /// `advices[i]` condition results (reward advices), by table index.
    pub conditions: RefCell<BTreeMap<i32, LuaResult>>,
    pub popup: RefCell<Option<BTreeMap<String, f64>>>,
    /// The `game` table's numbers.
    pub globals: RefCell<BTreeMap<String, f64>>,
    pub debug: Cell<bool>,
    pub has_city: Cell<bool>,
    pub established: Cell<bool>,
    pub difficulty: Cell<Option<i32>>,
    pub cells: Cell<(u32, u32)>,
    pub date: Cell<SimDate>,
    pub start_year: Cell<i32>,
    pub paused: Cell<bool>,
    pub now_ms: Cell<u32>,
    pub values: RefCell<BTreeMap<Src, Val>>,
    pub lists: RefCell<BTreeMap<ListSrc, Vec<Item>>>,
    pub occupants: RefCell<BTreeMap<u64, Occupant>>,
    pub statuses: RefCell<BTreeMap<u32, ConditionalStatus>>,
    pub reward_subjects: RefCell<BTreeMap<u32, SubjectInit>>,
    pub deals: RefCell<BTreeMap<u64, u32>>,
    /// History type → (value, trend slope, count).
    pub history: RefCell<BTreeMap<u32, (f64, f32, f64)>>,
    /// The days the history reads asked for: (type, from, to).
    pub history_days: RefCell<Vec<(u32, i32, i32)>>,
    pub ordinances: RefCell<BTreeMap<u32, Ordinance>>,
    pub tutorial: Cell<Option<u32>>,
    /// Host-object reference counts.
    pub refs: RefCell<BTreeMap<u64, i64>>,
    pub log: RefCell<Vec<HostCall>>,
    /// Tags returned by `create_tags`.
    pub tags: Cell<(i32, i32)>,
}

impl Default for ScriptedHost {
    fn default() -> Self {
        ScriptedHost {
            advisors: RefCell::new(Some(Vec::new())),
            advices: RefCell::default(),
            triggers: RefCell::default(),
            bad_triggers: RefCell::default(),
            compiled: RefCell::default(),
            next_ref: Cell::new(1),
            released_triggers: RefCell::default(),
            trigger_calls: RefCell::default(),
            trigger_hook: RefCell::new(None),
            conditions: RefCell::default(),
            popup: RefCell::new(None),
            globals: RefCell::default(),
            debug: Cell::new(false),
            has_city: Cell::new(true),
            established: Cell::new(true),
            difficulty: Cell::new(Some(0)),
            cells: Cell::new((64, 64)),
            date: Cell::new(SimDate::default()),
            start_year: Cell::new(2000),
            paused: Cell::new(false),
            now_ms: Cell::new(0),
            values: RefCell::default(),
            lists: RefCell::default(),
            occupants: RefCell::default(),
            statuses: RefCell::default(),
            reward_subjects: RefCell::default(),
            deals: RefCell::default(),
            history: RefCell::default(),
            history_days: RefCell::default(),
            ordinances: RefCell::default(),
            tutorial: Cell::new(None),
            refs: RefCell::default(),
            log: RefCell::default(),
            tags: Cell::new((1, 2)),
        }
    }
}

/// The literal reading of a trigger text.
pub fn literal_result(text: &[u8]) -> LuaResult {
    let t = String::from_utf8_lossy(text);
    let t = t.trim();
    match t {
        "true" => LuaResult::Bool(true),
        "false" => LuaResult::Bool(false),
        "nil" => LuaResult::Other,
        "error" => LuaResult::Error,
        "0/0" => LuaResult::Number(f64::NAN),
        _ => {
            if let Some(s) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                return LuaResult::Str(s.as_bytes().to_vec());
            }
            match t.parse::<f64>() {
                Ok(n) => LuaResult::Number(n),
                Err(_) => LuaResult::Other,
            }
        }
    }
}

/// The text form of a message's object.
pub fn obj_name(o: &MsgObj) -> String {
    match o {
        MsgObj::None => "none".into(),
        MsgObj::Advice(a) => format!("advice:{:#x}", a.guid()),
        MsgObj::Advisor(a) => format!("advisor:{}", a.id()),
        MsgObj::Host(t) => format!("host:{t:#x}"),
    }
}

fn target_name(t: &Target) -> String {
    match t {
        Target::System => "system".into(),
        Target::Advisor(a) => format!("advisor:{}", a.id()),
    }
}

impl ScriptedHost {
    pub fn new() -> Rc<ScriptedHost> {
        Rc::new(ScriptedHost::default())
    }

    fn push(&self, c: HostCall) {
        self.log.borrow_mut().push(c);
    }

    /// Takes the log.
    pub fn take_log(&self) -> Vec<HostCall> {
        std::mem::take(&mut self.log.borrow_mut())
    }

    /// Sets a value source.
    pub fn set_value(&self, src: Src, v: Val) {
        self.values.borrow_mut().insert(src, v);
    }

    /// Sets a trigger text's result.
    pub fn set_trigger(&self, text: &[u8], r: LuaResult) {
        self.triggers.borrow_mut().insert(text.to_vec(), r);
    }

    /// The text a compiled reference was made from.
    pub fn compiled_text(&self, r: i32) -> Option<Vec<u8>> {
        self.compiled.borrow().get(&r).cloned()
    }

    /// The globals written so far, by name (the last value).
    pub fn global(&self, name: &str) -> Option<f64> {
        self.globals.borrow().get(name).copied()
    }

    /// The last `set_local` of `name`.
    pub fn local(&self, name: &str) -> Option<(f64, Option<SubjectState>)> {
        self.log.borrow().iter().rev().find_map(|c| match c {
            HostCall::Local {
                name: n,
                value,
                subject,
            } if n == name => Some((*value, *subject)),
            _ => None,
        })
    }

    /// Sets the date from a day number.
    pub fn set_day(&self, day: i32) {
        self.date.set(SimDate::from_day(day));
    }
}

impl AdvisorHost for ScriptedHost {
    fn advisor_count(&self) -> Option<u32> {
        self.advisors.borrow().as_ref().map(|v| v.len() as u32)
    }
    fn advisor_entry(&self, i: u32) -> Option<AdvisorEntry> {
        let a = self.advisors.borrow();
        let v = a.as_ref()?;
        v.get((i as usize).checked_sub(1)?).cloned()
    }
    fn advice_count(&self) -> u32 {
        self.advices.borrow().len() as u32
    }
    fn advice_entry(&self, i: u32) -> Option<AdviceEntry> {
        self.advices.borrow().get((i as usize).checked_sub(1)?).cloned()
    }
    fn compile_trigger(&self, text: &[u8]) -> Option<i32> {
        if self.bad_triggers.borrow().iter().any(|t| t == text) {
            return None;
        }
        let r = self.next_ref.get();
        self.next_ref.set(r + 1);
        self.compiled.borrow_mut().insert(r, text.to_vec());
        Some(r)
    }
    fn release_trigger(&self, r: i32) {
        self.compiled.borrow_mut().remove(&r);
        self.released_triggers.borrow_mut().push(r);
    }
    fn call_trigger(&self, r: i32) -> LuaResult {
        let Some(text) = self.compiled_text(r) else {
            return LuaResult::Error;
        };
        self.trigger_calls.borrow_mut().push(text.clone());
        if let Some(hook) = &*self.trigger_hook.borrow() {
            hook(&text);
        }
        let set = self.triggers.borrow().get(&text).cloned();
        set.unwrap_or_else(|| literal_result(&text))
    }
    fn call_condition(&self, table_index: i32) -> LuaResult {
        self.conditions
            .borrow()
            .get(&table_index)
            .cloned()
            .unwrap_or(LuaResult::Other)
    }
    fn detokenize(&self, text: &[u8]) -> Vec<u8> {
        let mut v = b"detok:".to_vec();
        v.extend_from_slice(text);
        v
    }
    fn reload_scripts(&self) {
        self.push(HostCall::Lua("reload_scripts".into()));
    }
    fn register_api(&self, global: bool) {
        self.push(HostCall::Lua(format!("register_api({global})")));
    }
    fn set_tutorial_lua(&self, on: bool) {
        self.push(HostCall::Lua(format!("set_tutorial_lua({on})")));
    }
    fn create_tags(&self) -> (i32, i32) {
        self.push(HostCall::Lua("create_tags".into()));
        self.tags.get()
    }
    fn register_detokenizer(&self, on: bool) {
        self.push(HostCall::Lua(format!("register_detokenizer({on})")));
    }
    fn popup_priority(&self, key: &str) -> Option<f64> {
        self.popup.borrow().as_ref()?.get(key).copied()
    }
    fn collect_garbage(&self) {
        self.push(HostCall::CollectGarbage);
    }
    fn close_lua(&self) {
        self.push(HostCall::Lua("close_lua".into()));
    }
    fn set_global(&self, name: &str, value: f64) {
        self.globals.borrow_mut().insert(name.into(), value);
        self.push(HostCall::Global {
            name: name.into(),
            value,
        });
    }
    fn set_local(&self, name: &str, value: f64, subject: Option<&Rc<AdviceSubject>>) {
        self.globals.borrow_mut().insert(name.into(), value);
        self.push(HostCall::Local {
            name: name.into(),
            value,
            subject: subject.map(|s| s.state()),
        });
    }
    fn set_global_string(&self, name: &str, value: &[u8]) {
        self.push(HostCall::GlobalString {
            name: name.into(),
            value: value.to_vec(),
        });
    }
    fn debug_enabled(&self) -> bool {
        self.debug.get()
    }
    fn register_cheat(&self, id: u32, _name: &str) {
        self.push(HostCall::Cheat { id, on: true });
    }
    fn unregister_cheat(&self, id: u32) {
        self.push(HostCall::Cheat { id, on: false });
    }
    fn advisor_window(&self, start: bool) {
        self.push(HostCall::Window { start });
    }
    fn has_city(&self) -> bool {
        self.has_city.get()
    }
    fn city_established(&self) -> bool {
        self.established.get()
    }
    fn difficulty(&self) -> Option<i32> {
        self.difficulty.get()
    }
    fn cell_counts(&self) -> (u32, u32) {
        self.cells.get()
    }
    fn sim_date(&self) -> SimDate {
        self.date.get()
    }
    fn start_year(&self) -> i32 {
        self.start_year.get()
    }
    fn is_any_paused(&self) -> bool {
        self.paused.get()
    }
    fn now_ms(&self) -> u32 {
        self.now_ms.get()
    }
    fn subscribe(&self, target: &Target, msg_type: u32) {
        self.push(HostCall::Subscribe {
            target: target_name(target),
            msg: msg_type,
        });
    }
    fn unsubscribe(&self, target: &Target, msg_type: u32) {
        self.push(HostCall::Unsubscribe {
            target: target_name(target),
            msg: msg_type,
        });
    }
    fn post(&self, m: &OutMessage) {
        self.push(HostCall::Post {
            msg: m.msg_type,
            data1: m.data1,
            obj: obj_name(&m.unknown),
        });
    }
    fn send(&self, m: &OutMessage) {
        self.push(HostCall::Send {
            msg: m.msg_type,
            data1: m.data1,
            obj: obj_name(&m.unknown),
        });
    }
    fn post_update_self(&self, advisor: &Rc<Advisor>, data1: u32, data2: u32) {
        self.push(HostCall::UpdateSelf {
            advisor: advisor.id(),
            data1,
            data2,
        });
    }
    fn post_ticker(&self) {
        self.push(HostCall::Ticker);
    }
    fn global_effect(&self, id: u32, assert: bool) {
        self.push(HostCall::Effect { id, assert });
    }
    fn rebuild_text(&self, advice: &Rc<Advice>) {
        self.push(HostCall::Rebuild { guid: advice.guid() });
    }
    fn value(&self, src: &Src) -> Option<Val> {
        self.values.borrow().get(src).copied()
    }
    fn list(&self, src: &ListSrc) -> Option<Vec<Item>> {
        self.lists.borrow().get(src).cloned()
    }
    fn record_history(&self, history_type: u32, value: i32) {
        self.push(HostCall::History { history_type, value });
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.occupant_type)
    }
    fn occupant_bool_property(&self, occupant: u64, _id: u32) -> Option<bool> {
        self.occupants.borrow().get(&occupant)?.conditional
    }
    fn building_key_instance(&self, occupant: u64) -> Option<u32> {
        self.occupants.borrow().get(&occupant)?.key_instance
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occupant)
            .is_some_and(|o| o.groups.contains(&group))
    }
    fn conditional_status(&self, guid: u32) -> Option<ConditionalStatus> {
        self.statuses.borrow().get(&guid).cloned()
    }
    fn update_conditional_status(&self, guid: u32, status: &ConditionalStatus) {
        self.statuses.borrow_mut().insert(guid, status.clone());
        self.push(HostCall::Status {
            guid,
            status: status.clone(),
        });
    }
    fn reward_subject(&self, guid: u32) -> Option<SubjectInit> {
        self.reward_subjects.borrow().get(&guid).copied()
    }
    fn deal_type(&self, object: u64) -> Option<u32> {
        self.deals.borrow().get(&object).copied()
    }
    fn cancel_deal(&self, object: u64) {
        self.push(HostCall::CancelDeal { object });
    }
    fn history_value(&self, history_type: u32, day: i32) -> Option<f64> {
        self.history_days.borrow_mut().push((history_type, day, day));
        self.history.borrow().get(&history_type).map(|h| h.0)
    }
    fn history_trend(&self, history_type: u32, from: i32, to: i32) -> Option<f32> {
        self.history_days.borrow_mut().push((history_type, from, to));
        self.history.borrow().get(&history_type).map(|h| h.1)
    }
    fn history_count(&self, history_type: u32, from: i32, to: i32) -> Option<f64> {
        self.history_days.borrow_mut().push((history_type, from, to));
        self.history.borrow().get(&history_type).map(|h| h.2)
    }
    fn ordinance_flag(&self, id: u32, on: bool) -> Option<bool> {
        self.ordinances
            .borrow()
            .get(&id)
            .map(|o| if on { o.on } else { o.available })
    }
    fn set_ordinance_on(&self, id: u32, on: bool) {
        if let Some(o) = self.ordinances.borrow_mut().get_mut(&id) {
            o.on = on;
        }
        self.push(HostCall::SetOrdinance { id, on });
    }
    fn ordinance_amount(&self, id: u32, which: u32) -> Option<i64> {
        self.ordinances
            .borrow()
            .get(&id)
            .map(|o| o.amounts[which.min(2) as usize])
    }
    fn ordinance_text(&self, id: u32, which: u32) -> Option<Vec<u8>> {
        self.ordinances.borrow().get(&id).map(|o| {
            if which == 0 {
                o.name.clone()
            } else {
                o.description.clone()
            }
        })
    }
    fn tutorial_guid(&self) -> Option<u32> {
        self.tutorial.get()
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: 0x1234_5678,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => {
                self.add_ref(*token);
                Some(*token)
            }
            _ => None,
        }
    }
}
