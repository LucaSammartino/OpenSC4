// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AdvisorSystem` (§3.1, §4.1–§4.3, §4.6, §5.1, §5.2, §6.3, §6.7,
//! §6.8, §7.1, §7.2, §11.1, §12.1).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use crate::rng::{derive_seed, FixedSeeds, RzRandom};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::x87::ftol32;

use super::advice::Advice;
use super::advisor::Advisor;
use super::subject::AdviceSubject;
use super::{
    cheat, clsid, event, iid, msg, AdviceKind, AdvisorExtFactory, AdvisorHost, AdvisorKind, AdvisorMessage, MsgObj,
    OutMessage, Target, DEFAULT_POPUP_THRESHOLD, IDLE_PERIOD_MS, MONTHLY_DAY, POPUP_KEYS, SYSTEM_RECORD_VERSION,
};

/// What every advisor object shares: the host, the serial counter and the
/// system (the original's private global, §14.2).
pub(crate) struct Shared {
    pub(crate) host: Rc<dyn AdvisorHost>,
    serial: Cell<u32>,
    sys: RefCell<Weak<AdvisorSystem>>,
}

impl Shared {
    pub(crate) fn next_serial(&self) -> u32 {
        let s = self.serial.get().wrapping_add(1);
        self.serial.set(s);
        s
    }

    pub(crate) fn sys(&self) -> Option<Rc<AdvisorSystem>> {
        self.sys.borrow().upgrade()
    }
}

/// The system's fields (§3.1).
#[derive(Clone, Default)]
pub struct SystemState {
    /// Advisor ID → advisor, ascending ID.
    pub advisors: BTreeMap<i32, Rc<Advisor>>,
    /// Event ID → the subscribed advisors, by serial (pointer order
    /// [choice]).
    pub subscriptions: BTreeMap<u32, BTreeMap<u32, Rc<Advisor>>>,
    /// The context-advice stack (§6.7), bottom first.
    pub context: Vec<Rc<Advice>>,
    pub thresholds: [i32; 3],
    /// The two Lua tags (§8.1): subject, string.
    pub tags: (i32, i32),
    pub last_day: i32,
    /// The idle timer's start (`None`: never started, reads 0 ms
    /// [choice]).
    pub idle_start: Option<u32>,
    pub loaded_from_save: bool,
    pub initialized: bool,
    pub reload_pending: bool,
    /// The `DollyLlama` byte (UI only).
    pub dolly_llama: bool,
}

/// The advisor system.
pub struct AdvisorSystem {
    shared: Rc<Shared>,
    token: Cell<u64>,
    rng: RefCell<RzRandom>,
    s: RefCell<SystemState>,
    advisor_classes: RefCell<BTreeMap<u32, AdvisorExtFactory>>,
    advice_classes: RefCell<BTreeMap<u32, AdviceKind>>,
}

impl AdvisorSystem {
    /// A constructed system. Its generator is seeded with
    /// `derive_seed(session_seed, 0x4A0DD616)` (§9, foundations §6.3
    /// *[choice]*).
    pub fn new(host: Rc<dyn AdvisorHost>, session_seed: u32) -> Rc<AdvisorSystem> {
        let seed = derive_seed(session_seed, clsid::SYSTEM);
        let shared = Rc::new(Shared {
            host,
            serial: Cell::new(0),
            sys: RefCell::new(Weak::new()),
        });
        let mut advice_classes = BTreeMap::new();
        advice_classes.insert(clsid::ADVICE, AdviceKind::Plain);
        advice_classes.insert(clsid::REWARD_ADVICE, AdviceKind::Reward);
        advice_classes.insert(clsid::NEIGHBOR_DEAL_ADVICE, AdviceKind::NeighborDeal);
        let sys = Rc::new(AdvisorSystem {
            shared: shared.clone(),
            token: Cell::new(0),
            rng: RefCell::new(RzRandom::new(
                seed,
                &FixedSeeds {
                    wall_clock: seed,
                    process_clock: seed,
                },
            )),
            s: RefCell::new(SystemState {
                thresholds: [DEFAULT_POPUP_THRESHOLD; 3],
                ..SystemState::default()
            }),
            advisor_classes: RefCell::new(BTreeMap::new()),
            advice_classes: RefCell::new(advice_classes),
        });
        *shared.sys.borrow_mut() = Rc::downgrade(&sys);
        sys
    }

    fn host(&self) -> &dyn AdvisorHost {
        &*self.shared.host
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn state(&self) -> SystemState {
        self.s.borrow().clone()
    }

    /// Pokes the state (tests).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut SystemState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    /// Slot 9 `GetRandom`: the system's generator.
    pub fn with_random<R>(&self, f: impl FnOnce(&mut RzRandom) -> R) -> R {
        f(&mut self.rng.borrow_mut())
    }

    // ---- classes and objects ----

    /// Registers an advisor class implemented by an [`super::AdvisorExt`]
    /// (MySims).
    pub fn register_advisor_class(&self, class: u32, factory: AdvisorExtFactory) {
        self.advisor_classes.borrow_mut().insert(class, factory);
    }

    /// Registers an advice class that behaves as `kind` (other specs' advice
    /// classes).
    pub fn register_advice_class(&self, class: u32, kind: AdviceKind) {
        self.advice_classes.borrow_mut().insert(class, kind);
    }

    /// `CreateInstance(class, 0xCA09F544)`: `None` for an unknown class.
    pub fn create_advisor(self: &Rc<Self>, class: u32) -> Option<Rc<Advisor>> {
        let factory = self.advisor_classes.borrow().get(&class).cloned();
        if let Some(f) = factory {
            let ext = f(self);
            let adv = Advisor::new(&self.shared, AdvisorKind::Ext(class), Some(ext.clone()));
            ext.attach(&adv);
            return Some(adv);
        }
        AdvisorKind::from_class(class).map(|k| Advisor::new(&self.shared, k, None))
    }

    /// `CreateInstance(class, 0x8A09F5F5)`: `None` for an unknown class.
    pub fn create_advice(&self, class: u32) -> Option<Rc<Advice>> {
        let kind = self.advice_classes.borrow().get(&class).copied()?;
        Some(Advice::new(&self.shared, class, kind))
    }

    /// `CreateInstance(0x6A26D049, 0x6A26D050)`.
    pub fn new_subject(&self) -> Rc<AdviceSubject> {
        AdviceSubject::new(&self.shared)
    }

    /// The class store's factory: a new object of a saved class (§11): the
    /// system, an advisor (§11.6: `0xEAC937B4` and `0xCA09F543` load as
    /// plain advisors), an advice or a subject.
    pub fn create_object(self: &Rc<Self>, class: u32) -> Option<RefValue> {
        if class == clsid::SYSTEM {
            return Some(RefValue::Object(self.clone()));
        }
        if class == clsid::SUBJECT {
            return Some(RefValue::Object(self.new_subject()));
        }
        if let Some(a) = self.create_advice(class) {
            return Some(RefValue::Object(a));
        }
        if self.advisor_classes.borrow().contains_key(&class) {
            return self
                .create_advisor(class)
                .map(|a| RefValue::Object(a as Rc<dyn SaveObject>));
        }
        let kind = AdvisorKind::for_loaded_class(class)?;
        Some(RefValue::Object(Advisor::new(&self.shared, kind, None)))
    }

    // ---- the advisor map (§4.3) ----

    /// Slot 7 `GetAdvisor(id)` (no AddRef).
    pub fn get_advisor(&self, id: i32) -> Option<Rc<Advisor>> {
        self.s.borrow().advisors.get(&id).cloned()
    }

    /// Slot 8 `GetAdvisors`, ascending ID.
    pub fn advisors(&self) -> Vec<(i32, Rc<Advisor>)> {
        self.s.borrow().advisors.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// `load_advisor(i)` (§4.3).
    pub fn load_advisor(self: &Rc<Self>, i: u32) {
        let Some(e) = self.host().advisor_entry(i) else { return };
        if e.id == 0 {
            return;
        }
        let Some(adv) = self.create_advisor(e.class_id) else {
            return;
        };
        self.s.borrow_mut().advisors.insert(e.id, adv.clone());
        if !adv.init(i as i32) {
            self.remove_advisor(e.id);
        } else {
            self.host()
                .post(&OutMessage::new(msg::ADVISOR_ADDED, 0, MsgObj::Advisor(adv)));
        }
    }

    /// `load_advisors()` (§4.3): false without an `advisors` table.
    pub fn load_advisors(self: &Rc<Self>) -> bool {
        let Some(n) = self.host().advisor_count() else {
            return false;
        };
        for i in 1..=n {
            self.load_advisor(i);
        }
        true
    }

    /// `update_advisors()` (§4.3).
    pub fn update_advisors(self: &Rc<Self>) {
        let Some(n) = self.host().advisor_count() else { return };
        for i in 1..=n {
            let Some(e) = self.host().advisor_entry(i) else {
                continue;
            };
            if e.id == 0 {
                continue;
            }
            match self.get_advisor(e.id) {
                None => self.load_advisor(i),
                Some(a) => {
                    a.init(i as i32);
                }
            }
        }
    }

    /// Slot 6 `RemoveAdvisor(id)` (§4.3).
    pub fn remove_advisor(&self, id: i32) -> bool {
        let Some(adv) = self.get_advisor(id) else { return false };
        self.host()
            .send(&OutMessage::new(msg::ADVISOR_REMOVED, 0, MsgObj::Advisor(adv.clone())));
        let mut s = self.s.borrow_mut();
        let serial = adv.serial();
        s.subscriptions.retain(|_, set| {
            set.remove(&serial);
            !set.is_empty()
        });
        s.advisors.remove(&id);
        true
    }

    /// Slot 5 `AddAdvisor(a)` (§4.3): replaces; null does nothing. True.
    pub fn add_advisor(&self, a: Option<Rc<Advisor>>) -> bool {
        if let Some(a) = a {
            self.s.borrow_mut().advisors.insert(a.id(), a.clone());
            self.host()
                .post(&OutMessage::new(msg::ADVISOR_ADDED, 0, MsgObj::Advisor(a)));
        }
        true
    }

    /// `kill_advisors()` (§4.3).
    pub fn kill_advisors(&self) {
        self.host()
            .send(&OutMessage::new(msg::ADVISOR_REMOVED_ALL, 0, MsgObj::None));
        let all = self.advisors();
        for (_, a) in &all {
            a.shutdown();
        }
        self.s.borrow_mut().advisors.clear();
    }

    // ---- Init / Shutdown (§4.1, §4.2, §4.6) ----

    /// Slot 3 `Init` (§4.1) → true.
    pub fn init(self: &Rc<Self>) -> bool {
        let h = self.shared.host.clone();
        if self.s.borrow().initialized {
            h.advisor_window(true);
            return true;
        }
        // 1. The app's context is already open (Open(0x7E) is a no-op).
        h.register_api(true);
        let tags = h.create_tags();
        self.s.borrow_mut().tags = tags;
        h.register_detokenizer(true);
        self.reload_data();
        let (empty, loaded) = {
            let s = self.s.borrow();
            (s.advisors.is_empty(), s.loaded_from_save)
        };
        if !empty && !loaded {
            self.kill_advisors();
        }
        h.register_cheat(cheat::DOLLY_LLAMA, "DollyLlama");
        if h.debug_enabled() {
            h.register_cheat(cheat::ADV, "Adv");
        }
        if !loaded {
            self.load_advisors();
        }
        self.s.borrow_mut().initialized = true;
        self.update_advisors();
        self.init_popup_thresholds();
        for m in msg::SYSTEM_SUBSCRIPTIONS {
            h.subscribe(&Target::System, m);
        }
        h.advisor_window(true);
        true
    }

    /// `reload_data` (§4.2) → true.
    pub fn reload_data(&self) -> bool {
        let h = self.host();
        self.s.borrow_mut().reload_pending = false;
        h.reload_scripts();
        h.register_api(false);
        h.set_tutorial_lua(true);
        true
    }

    /// Slot 4 `Shutdown` (§4.6) → true.
    pub fn shutdown(&self) -> bool {
        let h = self.shared.host.clone();
        h.register_detokenizer(false);
        for m in msg::SYSTEM_SUBSCRIPTIONS {
            h.unsubscribe(&Target::System, m);
        }
        h.advisor_window(false);
        self.kill_advisors();
        h.unregister_cheat(cheat::DOLLY_LLAMA);
        h.unregister_cheat(cheat::ADV);
        h.close_lua();
        h.set_tutorial_lua(false);
        self.s.borrow_mut().initialized = false;
        true
    }

    /// Q2 [choice]: the host's "reload scripts" command sets reloadPending.
    pub fn set_reload_pending(&self) {
        self.s.borrow_mut().reload_pending = true;
    }

    // ---- popup thresholds (§6.8) ----

    /// `init_advice_popup_priority_thresholds`.
    pub fn init_popup_thresholds(&self) {
        let mut t = [DEFAULT_POPUP_THRESHOLD; 3];
        for (d, key) in POPUP_KEYS.iter().enumerate() {
            if let Some(v) = self.host().popup_priority(key) {
                t[d] = ftol32(v);
            }
        }
        self.s.borrow_mut().thresholds = t;
    }

    /// Slot 21 `GetAdvicePopupPriorityThreshold`: difficulty 1 or 2, else 0.
    pub fn popup_threshold(&self) -> i32 {
        let d = match self.host().difficulty() {
            Some(1) => 1,
            Some(2) => 2,
            _ => 0,
        };
        self.s.borrow().thresholds[d]
    }

    /// Slot 22 `GetDataTag(i)`: tag 0 or 1; [choice] 0 for any other index.
    pub fn data_tag(&self, i: i32) -> i32 {
        let t = self.s.borrow().tags;
        match i {
            0 => t.0,
            1 => t.1,
            _ => 0,
        }
    }

    // ---- the ticker and the system messages (§5.1, §5.2) ----

    /// `DoMessage` → true.
    pub fn do_message(self: &Rc<Self>, m: &AdvisorMessage) -> bool {
        match m.msg_type {
            msg::TICKER => self.on_ticker(),
            msg::POST_CITY_INIT => {
                if self.host().city_established() {
                    self.host().post_ticker();
                    if self.s.borrow().last_day < 1 {
                        self.monthly_step();
                    }
                }
                self.s.borrow_mut().loaded_from_save = false;
            }
            msg::CITY_ESTABLISHED => {
                self.push_event(event::NEW_CITY, None);
                self.host().post_ticker();
                if self.s.borrow().last_day < 1 {
                    self.monthly_step();
                }
            }
            _ => {}
        }
        true
    }

    /// The ticker (§5.1).
    pub fn on_ticker(self: &Rc<Self>) {
        let h = self.shared.host.clone();
        // 1.
        if h.has_city() && !h.city_established() {
            return;
        }
        // 2.
        let d = h.sim_date();
        let run_daily = if d.day_number > self.s.borrow().last_day {
            self.s.borrow_mut().last_day = d.day_number;
            if d.day == MONTHLY_DAY {
                h.collect_garbage();
                self.monthly_step();
            }
            h.set_global("g_year", f64::from(d.year as u32));
            h.set_global("g_month", f64::from(d.month as u32));
            h.set_global("g_day", f64::from(d.day as u32));
            h.set_global("g_date", f64::from(d.day_number as u32));
            h.set_global("g_year_count", f64::from(d.year.wrapping_sub(h.start_year()) as u32));
            true
        } else {
            // 3.
            let elapsed = self.s.borrow().idle_start.map_or(0, |t| h.now_ms().wrapping_sub(t));
            h.is_any_paused() && elapsed > IDLE_PERIOD_MS
        };
        // 4.
        if run_daily {
            if self.s.borrow().reload_pending {
                self.reload_data();
                self.update_advisors();
            }
            self.daily_step();
            self.s.borrow_mut().idle_start = Some(h.now_ms());
        }
        // 5.
        h.post_ticker();
    }

    /// The monthly step: `Simulate(1)` on every advisor, ascending ID.
    pub fn monthly_step(&self) {
        for (_, a) in self.advisors() {
            a.simulate(1);
        }
    }

    /// The daily step: `Simulate(0)` on every advisor, ascending ID.
    pub fn daily_step(&self) {
        for (_, a) in self.advisors() {
            a.simulate(0);
        }
    }

    /// A cheat (§5.2): `DollyLlama` toggles its byte; `Adv <args>` (debug)
    /// posts the debug messages or reloads.
    pub fn cheat(self: &Rc<Self>, id: u32, text: &str) {
        let h = self.shared.host.clone();
        match id {
            cheat::DOLLY_LLAMA => {
                let mut s = self.s.borrow_mut();
                s.dolly_llama = !s.dolly_llama;
            }
            cheat::ADV => {
                let words: Vec<&str> = text.split_whitespace().collect();
                let args: &[&str] = match words.first() {
                    Some(w) if w.eq_ignore_ascii_case("adv") => &words[1..],
                    _ => &words[..],
                };
                let post = |t: u32, d1: u32| h.post(&OutMessage::new(t, d1, MsgObj::None));
                match args {
                    ["guids", "on", ..] => post(msg::DEBUG_SHOW_GUIDS, 1),
                    ["guids", "off", ..] => post(msg::DEBUG_SHOW_GUIDS, 0),
                    ["trigger", hex, ..] => {
                        let v = u32::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0);
                        post(msg::DEBUG_FORCE_TRIGGER, v)
                    }
                    ["print", ..] => post(msg::DEBUG_PRINT, 0),
                    ["refresh", ..] => post(msg::DEBUG_REFRESH, 0),
                    ["reload", ..] => {
                        self.reload_data();
                        self.update_advisors();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // ---- events (§6.3) ----

    /// Slot 15 `PushEvent(event, subject)` → true.
    pub fn push_event(&self, event_id: u32, subject: Option<Rc<AdviceSubject>>) -> bool {
        let targets: Vec<Rc<Advisor>> = self
            .s
            .borrow()
            .subscriptions
            .get(&event_id)
            .map(|set| set.values().cloned().collect())
            .unwrap_or_default();
        for a in targets {
            a.push_event(event_id, subject.clone());
        }
        true
    }

    /// Slot 16 `SubscribeToEvent` → true.
    pub fn subscribe_to_event(&self, event_id: u32, a: &Rc<Advisor>) -> bool {
        self.s
            .borrow_mut()
            .subscriptions
            .entry(event_id)
            .or_default()
            .insert(a.serial(), a.clone());
        true
    }

    /// Slot 17 `UnsubscribeFromEvent` → true when it was subscribed.
    pub fn unsubscribe_from_event(&self, event_id: u32, a: &Rc<Advisor>) -> bool {
        let mut s = self.s.borrow_mut();
        let Some(set) = s.subscriptions.get_mut(&event_id) else {
            return false;
        };
        let found = set.remove(&a.serial()).is_some();
        if set.is_empty() {
            s.subscriptions.remove(&event_id);
        }
        found
    }

    /// The subscribers of an event, in order.
    pub fn subscribers(&self, event_id: u32) -> Vec<Rc<Advisor>> {
        self.s
            .borrow()
            .subscriptions
            .get(&event_id)
            .map(|set| set.values().cloned().collect())
            .unwrap_or_default()
    }

    // ---- the context advice (§6.7) ----

    /// Slot 18 `PushContextAdvice`.
    pub fn push_context_advice(&self, a: Rc<Advice>) {
        self.s.borrow_mut().context.push(a);
    }

    /// Slot 19 `PopContextAdvice`: the top, no longer held by the stack.
    pub fn pop_context_advice(&self) -> Option<Rc<Advice>> {
        self.s.borrow_mut().context.pop()
    }

    /// Slot 20 `GetContextAdvice`.
    pub fn context_advice(&self) -> Option<Rc<Advice>> {
        self.s.borrow().context.last().cloned()
    }

    // ---- values (§7.1) ----

    /// Slot 13 `SetGlobalValue(name, double)` → true.
    pub fn set_global_value(&self, name: &str, value: f64) -> bool {
        self.host().set_global(name, value);
        true
    }

    /// Slot 12 `SetGlobalValue(name, string)` → true.
    pub fn set_global_string(&self, name: &str, value: &[u8]) -> bool {
        self.host().set_global_string(name, value);
        true
    }

    /// Slot 14 `SetLocalValue(name, value, subject)` → true.
    pub fn set_local_value(&self, name: &str, value: f64, subject: Option<&Rc<AdviceSubject>>) -> bool {
        self.host().set_local(name, value, subject);
        true
    }

    pub(crate) fn shared(&self) -> &Rc<Shared> {
        &self.shared
    }
}

impl SaveObject for AdvisorSystem {
    fn class_id(&self) -> u32 {
        clsid::SYSTEM
    }

    /// §11.1: `u16 1`, `u32 count`, then `{ref advisor, s32 id}` by
    /// ascending ID.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let all = self.advisors();
        out.put_u16(SYSTEM_RECORD_VERSION);
        out.put_u32(all.len() as u32);
        for (id, a) in all {
            out.put_ref(&RefValue::Object(a));
            out.put_i32(id);
        }
        out.error() == 0
    }

    /// §11.1: version 1, else false. Each advisor is read with
    /// `0xCA09F544` and stored as `map[id]` ([choice] an unresolved
    /// reference stores nothing). loadedFromSave := !error.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(SYSTEM_RECORD_VERSION) {
            return false;
        }
        let Some(n) = inp.get_u32() else { return false };
        for _ in 0..n {
            let Some(r) = inp.get_ref_as(iid::ADVISOR) else { break };
            let Some(id) = inp.get_i32() else { break };
            if let Some(a) = r.object::<Advisor>() {
                self.s.borrow_mut().advisors.insert(id, a);
            }
        }
        let ok = inp.error() == 0;
        self.s.borrow_mut().loaded_from_save = ok;
        ok
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
