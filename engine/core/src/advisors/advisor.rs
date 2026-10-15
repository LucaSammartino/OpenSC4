// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4Advisor`, `cSC4AdvisorBase` and the seven advisor classes (§2.4,
//! §3.2, §3.4, §4.4, §5.3–§5.5, §6.2–§6.5, §11.2, §11.5, §11.6, §12.2).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::advice::{entry_matches, Advice};
use super::subject::AdviceSubject;
use super::system::Shared;
use super::{
    event, flags, iid, msg, variables, AdviceKind, AdvisorExt, AdvisorKind, AdvisorMessage, MsgObj, OutMessage, Target,
    ADVISOR_RECORD_VERSION, CITY_PLANNING_RECORD_VERSION, FORCE_SKIP_TEXT, GROUP_LANDMARK, GROUP_PARK, NEVER,
    OCCUPANT_TYPE_BUILDING, OCCUPANT_TYPE_NETWORK, PROP_CONDITIONAL_BUILDING,
};

/// One advice info record (§2.4).
#[derive(Clone)]
pub struct AdviceInfo {
    pub advice: Rc<Advice>,
    pub next_day: i32,
    pub expiry: i32,
    pub last_month: i32,
}

impl std::fmt::Debug for AdviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdviceInfo")
            .field("guid", &format_args!("{:#x}", self.advice.guid()))
            .field("next_day", &self.next_day)
            .field("expiry", &self.expiry)
            .field("last_month", &self.last_month)
            .finish()
    }
}

/// Pending-update bits (§5.5).
pub mod pending {
    pub const OCCUPANTS: u32 = 1;
    /// Utility: networks [choice: buildings set bit 1, networks bit 2].
    pub const NETWORKS: u32 = 2;
    pub const DEALS: u32 = 4;
}

/// The advisor's fields (§3.2, §3.4).
#[derive(Clone, Default)]
pub struct AdvisorState {
    pub table_index: i32,
    /// The caption cut at the first `|`.
    pub caption: Vec<u8>,
    pub name: Vec<u8>,
    pub advice_type: i32,
    pub month_counter: i32,
    pub current_day: i32,
    pub id: i32,
    pub cursor: i32,
    pub infos: Vec<AdviceInfo>,
    /// The active map: advice serial (pointer order [choice]) → index.
    pub active: BTreeMap<u32, usize>,
    /// The event multimap: event ID → indices, in insertion order.
    pub events: BTreeMap<u32, Vec<usize>>,
    /// The event queue.
    pub queue: Vec<(u32, Option<Rc<AdviceSubject>>)>,
    pub debug_subscribed: bool,
    pub base_subscribed: bool,
    pub own_subscribed: bool,
    /// The base family's dirty byte.
    pub dirty: bool,
    pub pending: u32,
    // CityPlanning (persisted, §11.5)
    pub building_count: i32,
    pub park_count: i32,
    pub recreation_count: i32,
    /// Exemplar instance → count, ascending signed key.
    pub landmarks: BTreeMap<i32, i32>,
    // Traffic: lots with a car zot.
    pub zots: BTreeSet<u64>,
    // Safety
    pub fire_active: bool,
    pub riot_active: bool,
    pub user_disaster: bool,
}

impl std::fmt::Debug for AdvisorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvisorState")
            .field("id", &self.id)
            .field("table_index", &self.table_index)
            .field("month_counter", &self.month_counter)
            .field("current_day", &self.current_day)
            .field("cursor", &self.cursor)
            .field("infos", &self.infos)
            .field("active", &self.active)
            .finish()
    }
}

/// One advisor.
pub struct Advisor {
    pub(crate) shared: Rc<Shared>,
    serial: u32,
    kind: AdvisorKind,
    ext: Option<Rc<dyn AdvisorExt>>,
    token: Cell<u64>,
    s: RefCell<AdvisorState>,
}

impl Advisor {
    pub(crate) fn new(shared: &Rc<Shared>, kind: AdvisorKind, ext: Option<Rc<dyn AdvisorExt>>) -> Rc<Advisor> {
        let a = Rc::new(Advisor {
            shared: shared.clone(),
            serial: shared.next_serial(),
            kind,
            ext,
            token: Cell::new(0),
            s: RefCell::new(AdvisorState {
                // The constructor sets dirty (§3.4).
                dirty: kind.is_base_family(),
                ..AdvisorState::default()
            }),
        });
        a.token.set(shared.created(a.clone()));
        a
    }

    pub fn serial(&self) -> u32 {
        self.serial
    }
    pub fn kind(&self) -> AdvisorKind {
        self.kind
    }
    /// The class extension (MySims).
    pub fn ext(&self) -> Option<Rc<dyn AdvisorExt>> {
        self.ext.clone()
    }
    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }
    pub fn state(&self) -> AdvisorState {
        self.s.borrow().clone()
    }
    /// Pokes the state (tests, the variables).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut AdvisorState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }
    pub(crate) fn host(&self) -> Rc<dyn super::AdvisorHost> {
        self.shared.host.clone()
    }

    /// Slot 5 `GetID`.
    pub fn id(&self) -> i32 {
        self.s.borrow().id
    }
    pub fn caption(&self) -> Vec<u8> {
        self.s.borrow().caption.clone()
    }
    /// Slot 7 `SetName`.
    pub fn set_caption(&self, v: &[u8]) {
        self.s.borrow_mut().caption = v.to_vec();
    }
    pub fn personal_name(&self) -> Vec<u8> {
        self.s.borrow().name.clone()
    }
    pub fn set_personal_name(&self, v: &[u8]) {
        self.s.borrow_mut().name = v.to_vec();
    }
    pub fn month_counter(&self) -> i32 {
        self.s.borrow().month_counter
    }
    pub fn current_day(&self) -> i32 {
        self.s.borrow().current_day
    }
    /// The advices, in vector order.
    pub fn advices(&self) -> Vec<Rc<Advice>> {
        self.s.borrow().infos.iter().map(|i| i.advice.clone()).collect()
    }
    pub fn info(&self, idx: usize) -> Option<AdviceInfo> {
        self.s.borrow().infos.get(idx).cloned()
    }
    fn index_of(&self, a: &Rc<Advice>) -> Option<usize> {
        self.s.borrow().infos.iter().position(|i| Rc::ptr_eq(&i.advice, a))
    }

    /// Slots 10/11 `GetCurrentAdvices`: the active map (advice, index), in
    /// map order.
    pub fn current_advices(&self) -> Vec<(Rc<Advice>, usize)> {
        let s = self.s.borrow();
        s.active
            .values()
            .filter_map(|&i| s.infos.get(i).map(|inf| (inf.advice.clone(), i)))
            .collect()
    }

    /// Slot 12 `GetAdviceByGUID`.
    pub fn advice_by_guid(&self, guid: u32) -> Option<Rc<Advice>> {
        self.advices().into_iter().find(|a| a.guid() == guid)
    }

    /// Slot 13 `GetAdviceBySessionID`.
    pub fn advice_by_session(&self, session: i32) -> Option<Rc<Advice>> {
        self.advices().into_iter().find(|a| a.session_id() == session)
    }

    // ---- Init / Shutdown (§4.4) ----

    /// Slot 3 `Init(i)`.
    pub fn init(self: &Rc<Self>, table_index: i32) -> bool {
        if let Some(ext) = &self.ext {
            return ext.init(self, table_index);
        }
        if !self.kind.is_base_family() {
            return self.base_init(table_index);
        }
        let h = self.host();
        // The subclass's own messages (once).
        let own: &[u32] = match self.kind {
            AdvisorKind::CityPlanning => &msg::CP_UNHANDLED,
            AdvisorKind::Safety => &msg::SAFETY_SUBSCRIPTIONS,
            AdvisorKind::Traffic => &msg::TRAFFIC_SUBSCRIPTIONS,
            AdvisorKind::Utility => &msg::UTILITY_SUBSCRIPTIONS,
            _ => &[],
        };
        if !own.is_empty() && !self.s.borrow().own_subscribed {
            for &m in own {
                h.subscribe(&Target::Advisor(self.clone()), m);
            }
            self.s.borrow_mut().own_subscribed = true;
        }
        self.s.borrow_mut().dirty = true;
        // cSC4AdvisorBase::Init: the base subscriptions, once.
        if !self.s.borrow().base_subscribed {
            for m in msg::BASE_SUBSCRIPTIONS {
                h.subscribe(&Target::Advisor(self.clone()), m);
            }
            self.s.borrow_mut().base_subscribed = true;
        }
        self.base_init(table_index)
    }

    /// `cSC4Advisor::Init(i)` (§4.4).
    pub fn base_init(self: &Rc<Self>, table_index: i32) -> bool {
        let h = self.host();
        let sys = self.shared.sys();
        // 1.
        let entry = if table_index > 0 {
            h.advisor_entry(table_index as u32)
        } else {
            None
        };
        let Some(e) = entry else { return false };
        {
            let mut s = self.s.borrow_mut();
            if s.id != 0 && e.id != s.id {
                return false;
            }
            s.table_index = table_index;
            s.id = e.id;
            let cut = e.caption.iter().position(|&c| c == b'|').unwrap_or(e.caption.len());
            s.caption = e.caption[..cut].to_vec();
            s.name = e.name.clone();
            s.advice_type = e.advice_type;
        }
        h.post(&OutMessage::new(msg::ADVISOR_CHANGED, 0, MsgObj::Advisor(self.clone())));
        // 2. GUID → (index, class); the first entry of a GUID wins [choice].
        let mut to_create: BTreeMap<u32, (i32, u32)> = BTreeMap::new();
        for j in 1..=h.advice_count() {
            if let Some(a) = h.advice_entry(j) {
                if entry_matches(&a, e.advice_type) {
                    to_create.entry(a.guid).or_insert((j as i32, a.class_id));
                }
            }
        }
        // 3.
        for a in self.advices() {
            a.set_flags(flags::ACTIVE, false);
        }
        self.s.borrow_mut().active.clear();
        h.send(&OutMessage::new(
            msg::ADVICE_REMOVED_ALL,
            0,
            MsgObj::Advisor(self.clone()),
        ));
        // 4. Back to front.
        let n = self.s.borrow().infos.len();
        for idx in (0..n).rev() {
            let a = self.s.borrow().infos[idx].advice.clone();
            let guid = a.guid();
            let remove = match to_create.get(&guid).copied() {
                None => true,
                Some((index, _)) => {
                    if !a.init(index) {
                        a.set_flags(flags::CLASS_MISMATCH, false);
                        to_create.remove(&guid);
                        true
                    } else if a.check_flags(flags::CLASS_MISMATCH) {
                        true
                    } else {
                        to_create.remove(&guid);
                        false
                    }
                }
            };
            if remove {
                self.retire_advice(Some(&a));
                a.shutdown();
                self.s.borrow_mut().infos.remove(idx);
            }
        }
        // 5. Ascending GUID.
        for (_, (index, class)) in to_create {
            let Some(sys) = &sys else { break };
            let Some(a) = sys.create_advice(class) else { continue };
            a.init(index);
            a.set_advisor_id(e.id);
            let mut s = self.s.borrow_mut();
            let (day, month) = (s.current_day, s.month_counter);
            s.infos.push(AdviceInfo {
                advice: a,
                next_day: day,
                expiry: 0,
                last_month: month.wrapping_sub(1),
            });
        }
        // 6.
        self.rebuild_events();
        if let Some(sys) = &sys {
            let ids: Vec<u32> = self.s.borrow().events.keys().copied().collect();
            for ev in ids {
                sys.subscribe_to_event(ev, self);
            }
        }
        // 7.
        if !self.s.borrow().debug_subscribed {
            for m in msg::DEBUG_SUBSCRIPTIONS {
                h.subscribe(&Target::Advisor(self.clone()), m);
            }
            self.s.borrow_mut().debug_subscribed = true;
        }
        true
    }

    /// The event multimap from the advices' event IDs.
    fn rebuild_events(&self) {
        let mut s = self.s.borrow_mut();
        let mut m: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, inf) in s.infos.iter().enumerate() {
            let ev = inf.advice.event_id();
            if ev != 0 {
                m.entry(ev).or_default().push(i);
            }
        }
        s.events = m;
    }

    /// Slot 4 `Shutdown` → true.
    pub fn shutdown(self: &Rc<Self>) -> bool {
        if let Some(ext) = &self.ext {
            return ext.shutdown(self);
        }
        if self.kind.is_base_family() {
            let h = self.host();
            let (own, base) = {
                let s = self.s.borrow();
                (s.own_subscribed, s.base_subscribed)
            };
            let own_msgs: &[u32] = match self.kind {
                AdvisorKind::CityPlanning => &msg::CP_UNHANDLED,
                AdvisorKind::Safety => &msg::SAFETY_SUBSCRIPTIONS,
                AdvisorKind::Traffic => &msg::TRAFFIC_SUBSCRIPTIONS,
                AdvisorKind::Utility => &msg::UTILITY_SUBSCRIPTIONS,
                _ => &[],
            };
            if own {
                for &m in own_msgs {
                    h.unsubscribe(&Target::Advisor(self.clone()), m);
                }
            }
            if base {
                for m in msg::BASE_SUBSCRIPTIONS {
                    h.unsubscribe(&Target::Advisor(self.clone()), m);
                }
            }
            let mut s = self.s.borrow_mut();
            s.own_subscribed = false;
            s.base_subscribed = false;
            s.dirty = false;
        }
        self.base_shutdown()
    }

    /// `cSC4Advisor::Shutdown` → true.
    pub fn base_shutdown(self: &Rc<Self>) -> bool {
        let h = self.host();
        if self.s.borrow().debug_subscribed {
            for m in msg::DEBUG_SUBSCRIPTIONS {
                h.unsubscribe(&Target::Advisor(self.clone()), m);
            }
        }
        {
            let mut s = self.s.borrow_mut();
            s.debug_subscribed = false;
            s.month_counter = 0;
            s.current_day = 0;
            s.active.clear();
        }
        h.send(&OutMessage::new(
            msg::ADVICE_REMOVED_ALL,
            0,
            MsgObj::Advisor(self.clone()),
        ));
        let events: Vec<u32> = std::mem::take(&mut self.s.borrow_mut().events).into_keys().collect();
        if let Some(sys) = self.shared.sys() {
            for ev in events {
                sys.unsubscribe_from_event(ev, self);
            }
        }
        let infos = std::mem::take(&mut self.s.borrow_mut().infos);
        for inf in &infos {
            inf.advice.shutdown();
        }
        true
    }

    // ---- Simulate (§5.3) ----

    /// Slot 14 `Simulate(kind)` → true.
    pub fn simulate(self: &Rc<Self>, kind: i32) -> bool {
        match kind {
            0 => {
                let day = self.host().sim_date().day_number;
                self.s.borrow_mut().current_day = day;
                self.simulate_day();
            }
            1 => {
                {
                    let mut s = self.s.borrow_mut();
                    s.month_counter = s.month_counter.wrapping_add(1);
                }
                self.monthly();
            }
            _ => {}
        }
        true
    }

    /// Class slot 6.
    pub fn simulate_day(self: &Rc<Self>) {
        if let Some(ext) = &self.ext {
            return ext.simulate_day(self);
        }
        if self.kind.is_base_family() && self.s.borrow().dirty {
            self.update_variables();
            self.s.borrow_mut().dirty = false;
        }
        self.base_day_pass();
    }

    /// Class slot 8, the monthly hook.
    pub fn monthly(self: &Rc<Self>) {
        if let Some(ext) = &self.ext {
            return ext.monthly(self);
        }
        if self.kind.is_base_family() {
            self.update_variables();
        }
    }

    /// Class slot 9 `update_variables` (§7).
    pub fn update_variables(self: &Rc<Self>) {
        variables::update_variables(self);
    }

    // ---- the day pass (§6.2) ----

    /// `cSC4Advisor::simulate_day`: the batch, the active pass, then the
    /// events (class slot 7).
    pub fn base_day_pass(self: &Rc<Self>) {
        // 1–2.
        let n = self.s.borrow().infos.len();
        if n > 0 {
            let b = super::batch_size(n);
            for _ in 0..b {
                let idx = {
                    let mut s = self.s.borrow_mut();
                    let len = s.infos.len();
                    if len == 0 {
                        break;
                    }
                    if s.cursor < 0 || s.cursor as usize >= len {
                        s.cursor = 0;
                    }
                    let i = s.cursor as usize;
                    s.cursor += 1;
                    i
                };
                self.visit(idx);
            }
        }
        // 3. A snapshot of the active map; an advice retired meanwhile is
        // skipped [choice].
        let snapshot: Vec<(u32, usize)> = self.s.borrow().active.iter().map(|(k, v)| (*k, *v)).collect();
        for (serial, idx) in snapshot {
            if self.s.borrow().active.get(&serial) != Some(&idx) {
                continue;
            }
            let Some(inf) = self.info(idx) else { continue };
            let a = inf.advice;
            let day = self.current_day();
            let expired = day >= inf.expiry && !a.check_flags(flags::NO_TIMEOUT);
            let off =
                expired || a.check_flags(flags::EXPIRE_REQUEST) || (!a.check_flags(flags::PERSIST) && !a.get_trigger());
            if off {
                if self.s.borrow().active.get(&serial) == Some(&idx) {
                    self.s.borrow_mut().active.remove(&serial);
                    self.deactivate(idx);
                }
            } else if a.check_flags(flags::TEXT_DIRTY) {
                a.update_text();
                self.host()
                    .send(&OutMessage::new(msg::ADVICE_CHANGED, 0, MsgObj::Advice(a.clone())));
            }
        }
        // 4.
        self.simulate_day_events();
    }

    /// The batch visit of one advice (§6.2 step 2).
    fn visit(self: &Rc<Self>, idx: usize) {
        let Some(inf) = self.info(idx) else { return };
        let a = inf.advice;
        if a.is_active() || a.check_flags(flags::EVENT) {
            return;
        }
        let (day, month) = {
            let s = self.s.borrow();
            (s.current_day, s.month_counter)
        };
        if inf.next_day <= day && inf.last_month < month {
            if let Some(i) = self.s.borrow_mut().infos.get_mut(idx) {
                i.last_month = month;
            }
            if a.get_trigger() && self.index_of(&a) == Some(idx) {
                self.activate(idx);
            }
        }
    }

    /// Class slot 7 `simulate_day_events`.
    pub fn simulate_day_events(self: &Rc<Self>) {
        if let Some(ext) = &self.ext {
            return ext.simulate_day_events(self);
        }
        if self.kind == AdvisorKind::Utility {
            return self.utility_day_events();
        }
        self.base_day_events();
    }

    /// `cSC4Advisor::simulate_day_events` (§6.3): the events queued when it
    /// starts, in order.
    pub fn base_day_events(self: &Rc<Self>) {
        let queue = std::mem::take(&mut self.s.borrow_mut().queue);
        for (ev, subject) in queue {
            for idx in self.event_indices(ev) {
                let Some(inf) = self.info(idx) else { continue };
                let a = inf.advice;
                if !a.is_active() {
                    self.set_last_month(idx);
                    a.set_event_subject(subject.clone());
                    if a.get_trigger() {
                        self.activate(idx);
                    }
                } else {
                    a.set_flags(flags::TEXT_DIRTY, true);
                    if a.get_trigger() {
                        a.set_event_subject(subject.clone());
                    }
                }
            }
        }
    }

    fn event_indices(&self, ev: u32) -> Vec<usize> {
        self.s.borrow().events.get(&ev).cloned().unwrap_or_default()
    }

    fn set_last_month(&self, idx: usize) {
        let mut s = self.s.borrow_mut();
        let m = s.month_counter;
        if let Some(i) = s.infos.get_mut(idx) {
            i.last_month = m;
        }
    }

    /// The Utility override (§6.3): the deal events match the deal type,
    /// refresh an advice for the same deal and clone one for another deal;
    /// other events test the trigger before setting the subject.
    fn utility_day_events(self: &Rc<Self>) {
        let h = self.host();
        let queue = std::mem::take(&mut self.s.borrow_mut().queue);
        for (ev, subject) in queue {
            let is_deal = event::DEAL_EVENTS.contains(&ev);
            let deal = subject.as_ref().and_then(|s| s.object());
            for idx in self.event_indices(ev) {
                let Some(inf) = self.info(idx) else { continue };
                let a = inf.advice;
                if is_deal {
                    // [choice] No deal or no type: the advice is skipped.
                    let Some(obj) = deal else { continue };
                    let Some(t) = h.deal_type(obj) else { continue };
                    if t != a.nd_type() as u32 {
                        continue;
                    }
                    if !a.is_active() {
                        self.set_last_month(idx);
                        if a.get_trigger() {
                            a.set_event_subject(subject.clone());
                            self.activate(idx);
                        }
                    } else if a.event_subject().and_then(|s| s.object()) == Some(obj) {
                        a.set_flags(flags::TEXT_DIRTY, true);
                        if a.get_trigger() {
                            a.set_event_subject(subject.clone());
                        }
                    } else {
                        let c = a.clone_advice();
                        c.set_advisor_id(self.id());
                        let ci = {
                            let mut s = self.s.borrow_mut();
                            let (day, month) = (s.current_day, s.month_counter);
                            s.infos.push(AdviceInfo {
                                advice: c.clone(),
                                next_day: day,
                                expiry: 0,
                                last_month: month,
                            });
                            let ci = s.infos.len() - 1;
                            s.events.entry(ev).or_default().push(ci);
                            ci
                        };
                        if c.get_trigger() {
                            c.set_event_subject(subject.clone());
                            self.activate(ci);
                        }
                    }
                } else if !a.is_active() {
                    self.set_last_month(idx);
                    if a.get_trigger() {
                        a.set_event_subject(subject.clone());
                        self.activate(idx);
                    }
                } else {
                    a.set_flags(flags::TEXT_DIRTY, true);
                    if a.get_trigger() {
                        a.set_event_subject(subject.clone());
                    }
                }
            }
        }
    }

    /// Slot 18 `PushEvent` → true: queued (held).
    pub fn push_event(&self, ev: u32, subject: Option<Rc<AdviceSubject>>) -> bool {
        self.s.borrow_mut().queue.push((ev, subject));
        true
    }

    // ---- activation (§6.4) ----

    /// Activates the advice at `idx`.
    pub fn activate(self: &Rc<Self>, idx: usize) {
        let h = self.host();
        let a = {
            let mut s = self.s.borrow_mut();
            let day = s.current_day;
            let Some(inf) = s.infos.get_mut(idx) else { return };
            inf.next_day = 0;
            inf.expiry = day.wrapping_add(inf.advice.timeout());
            let a = inf.advice.clone();
            s.active.insert(a.serial(), idx);
            a
        };
        a.set_flags(flags::ACTIVE, true);
        a.set_flags(flags::EXPIRE_REQUEST, false);
        h.post(&OutMessage::new(msg::ADVICE_ADDED, 0, MsgObj::Advice(a.clone())));
        let cmd = a.command_id();
        if cmd != 0 {
            h.post(&OutMessage::new(
                msg::ADVICE_COMMAND,
                cmd,
                MsgObj::Advisor(self.clone()),
            ));
        }
    }

    /// Deactivates the advice at `idx` (the caller erased it from the
    /// active map).
    pub fn deactivate(self: &Rc<Self>, idx: usize) {
        let Some(inf) = self.info(idx) else { return };
        let a = inf.advice;
        a.set_flags(flags::ACTIVE | flags::EXPIRE_REQUEST | flags::FORCED_TRIGGER, false);
        {
            let mut s = self.s.borrow_mut();
            let day = s.current_day;
            if let Some(i) = s.infos.get_mut(idx) {
                i.next_day = if a.check_flags(flags::ONCE) {
                    NEVER
                } else {
                    day.wrapping_add(a.frequency())
                };
                i.expiry = 0;
            }
        }
        a.set_event_subject(None);
        self.host()
            .send(&OutMessage::new(msg::ADVICE_REMOVED, 0, MsgObj::Advice(a)));
    }

    // ---- retire, command test (§6.5) ----

    /// Slot 20 `RetireAdvice`. The base looks the advice up in the active
    /// map (an advice missing from it is looked up in the vector instead of
    /// using a garbage index [choice]); CitySituations returns true even when
    /// it is not found.
    pub fn retire_advice(self: &Rc<Self>, a: Option<&Rc<Advice>>) -> bool {
        let Some(a) = a else { return false };
        if !a.is_active() {
            return false;
        }
        let in_map = self.s.borrow_mut().active.remove(&a.serial());
        let idx = match in_map {
            Some(i) if self.info(i).is_some_and(|inf| Rc::ptr_eq(&inf.advice, a)) => Some(i),
            _ => self.index_of(a),
        };
        match idx {
            Some(i) => {
                self.deactivate(i);
                true
            }
            None => self.kind == AdvisorKind::CitySituations,
        }
    }

    /// Slot 19 `CommandWouldTrigger`.
    pub fn command_would_trigger(self: &Rc<Self>, cmd: u32) -> bool {
        if let Some(ext) = &self.ext {
            return ext.command_would_trigger(self, cmd);
        }
        self.base_command_would_trigger(cmd)
    }

    /// `cSC4Advisor::CommandWouldTrigger`: from the last advice.
    pub fn base_command_would_trigger(self: &Rc<Self>, cmd: u32) -> bool {
        for a in self.advices().into_iter().rev() {
            if !a.check_flags(flags::EVENT) && a.command_id() == cmd && a.get_trigger() {
                return true;
            }
        }
        false
    }

    // ---- messages (§5.4, §5.5) ----

    /// `DoMessage`.
    pub fn do_message(self: &Rc<Self>, m: &AdvisorMessage) -> bool {
        if let Some(ext) = &self.ext {
            return ext.do_message(self, m);
        }
        let h = self.host();
        let occupant = matches!(m.msg_type, msg::INSERT_OCCUPANT | msg::REMOVE_OCCUPANT);
        let occ_type = if occupant {
            m.void1.map(|o| h.occupant_type(o)).unwrap_or(0)
        } else {
            0
        };
        let building = occ_type == OCCUPANT_TYPE_BUILDING;
        let network = occ_type == OCCUPANT_TYPE_NETWORK;
        match self.kind {
            AdvisorKind::Plain | AdvisorKind::Ext(_) => return self.base_do_message(m),
            AdvisorKind::Base | AdvisorKind::CitySituations => {}
            AdvisorKind::Finance => {
                if occupant && building {
                    self.mark_pending(pending::OCCUPANTS);
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.s.borrow_mut().pending = 0;
                }
            }
            AdvisorKind::CityPlanning => {
                if occupant {
                    if !building {
                        return true;
                    }
                    let o = m.void1.unwrap_or(0);
                    let d = if m.msg_type == msg::INSERT_OCCUPANT { 1 } else { -1 };
                    let park = h.is_occupant_group(o, GROUP_PARK);
                    let landmark = h.is_occupant_group(o, GROUP_LANDMARK);
                    let key = if landmark { h.building_key_instance(o) } else { None };
                    {
                        let mut s = self.s.borrow_mut();
                        s.building_count = s.building_count.wrapping_add(d);
                        if park {
                            s.park_count = s.park_count.wrapping_add(d);
                            s.recreation_count = s.recreation_count.wrapping_add(d);
                        }
                        if let Some(k) = key {
                            let k = k as i32;
                            if d > 0 {
                                s.landmarks.insert(k, 1);
                            } else if let Some(c) = s.landmarks.get_mut(&k) {
                                *c = c.wrapping_sub(1);
                            }
                        }
                    }
                    self.mark_pending(pending::OCCUPANTS);
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.run_pending();
                }
            }
            AdvisorKind::Hq => {
                if occupant && building {
                    self.mark_pending(pending::OCCUPANTS);
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.run_pending();
                }
            }
            AdvisorKind::Safety => {
                if msg::SAFETY_SUBSCRIPTIONS.contains(&m.msg_type) {
                    variables::safety_message(self, m);
                    return true;
                }
                if occupant && building {
                    self.mark_pending(pending::OCCUPANTS);
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.run_pending();
                }
            }
            AdvisorKind::Traffic => {
                if m.msg_type == msg::LOT_DEVELOPER_NOTIFICATION {
                    if let Some(lot) = m.unknown {
                        let mut s = self.s.borrow_mut();
                        match m.data1 {
                            1 => {
                                s.zots.insert(lot);
                            }
                            2 => {
                                s.zots.remove(&lot);
                            }
                            _ => {}
                        }
                    }
                    let n = self.s.borrow().zots.len();
                    h.set_global("g_car_zot_count", n as f64);
                    return true;
                }
                if occupant && (building || network) {
                    self.mark_pending(pending::OCCUPANTS);
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.run_pending();
                }
            }
            AdvisorKind::Utility => {
                if occupant && (building || network) {
                    self.mark_pending(if building {
                        pending::OCCUPANTS
                    } else {
                        pending::NETWORKS
                    });
                } else if m.msg_type == msg::UPDATE_SELF {
                    self.run_pending();
                } else if m.msg_type == msg::DEAL_STATUS {
                    self.mark_pending(pending::DEALS);
                    if (3..=6).contains(&m.data1) {
                        let ev = event::DEAL_EVENTS[(m.data1 - 3) as usize];
                        let subj = AdviceSubject::new(&self.shared);
                        if let Some(d) = m.unknown {
                            subj.set_object(Some(d));
                        }
                        self.push_event(ev, Some(subj));
                    }
                    return true;
                } else if matches!(m.msg_type, msg::POWER_STRIKE_BEGIN | msg::POWER_STRIKE_END) {
                    let begin = m.msg_type == msg::POWER_STRIKE_BEGIN;
                    h.set_global("g_power_strike", if begin { 1.0 } else { 0.0 });
                    if let Some(sys) = self.shared.sys() {
                        sys.push_event(
                            if begin {
                                event::POWER_STRIKE_BEGIN
                            } else {
                                event::POWER_STRIKE_END
                            },
                            None,
                        );
                    }
                    return true;
                }
            }
        }
        self.advisor_base_do_message(m)
    }

    /// Posts UpdateSelf(0, 0) when no bit was set, then sets `bits`.
    fn mark_pending(self: &Rc<Self>, bits: u32) {
        let was = self.s.borrow().pending;
        if was == 0 {
            self.host().post_update_self(self, 0, 0);
        }
        self.s.borrow_mut().pending |= bits;
    }

    /// UpdateSelf: the matching updates, then the bits are cleared.
    fn run_pending(self: &Rc<Self>) {
        let p = std::mem::take(&mut self.s.borrow_mut().pending);
        if p & (pending::OCCUPANTS | pending::NETWORKS) != 0 {
            variables::occupant_variables(self);
        }
        if p & pending::DEALS != 0 {
            variables::deal_variables(self);
        }
    }

    /// `cSC4AdvisorBase::DoMessage` (§5.4).
    pub fn advisor_base_do_message(self: &Rc<Self>, m: &AdvisorMessage) -> bool {
        let h = self.host();
        match m.msg_type {
            msg::INSERT_OCCUPANT | msg::REMOVE_OCCUPANT => {
                let Some(o) = m.void1 else { return true };
                if h.occupant_type(o) == OCCUPANT_TYPE_BUILDING
                    && h.occupant_bool_property(o, PROP_CONDITIONAL_BUILDING) == Some(true)
                {
                    if let Some(k) = h.building_key_instance(o) {
                        h.post_update_self(self, 1, k);
                    }
                }
                true
            }
            msg::UPDATE_SELF => {
                match m.data1 {
                    1 => {
                        if let Some(a) = self.advice_by_guid(m.data2) {
                            a.get_trigger();
                            let st = h.conditional_status(m.data2);
                            if st.map_or(true, |s| s.status & super::status_bits::AVAILABLE == 0) {
                                a.set_flags(flags::EXPIRE_REQUEST, true);
                                self.s.borrow_mut().dirty = true;
                            }
                        }
                    }
                    2 => {
                        for a in self.advices() {
                            if a.check_flags(flags::REWARD) {
                                a.get_trigger();
                            }
                        }
                    }
                    _ => {}
                }
                true
            }
            msg::ORDINANCE_UPDATED => {
                h.post_update_self(self, 2, 0);
                true
            }
            msg::POST_CITY_INIT_COMPLETE => {
                for a in self.advices() {
                    if a.kind() == AdviceKind::Reward {
                        a.get_trigger();
                    }
                }
                true
            }
            msg::POST_CITY_INIT => {
                self.update_variables();
                true
            }
            _ => self.base_do_message(m),
        }
    }

    /// `cSC4Advisor::DoMessage`: the debug messages (§5.4).
    pub fn base_do_message(self: &Rc<Self>, m: &AdvisorMessage) -> bool {
        let h = self.host();
        match m.msg_type {
            msg::DEBUG_FORCE_TRIGGER => {
                let id = self.id() as u32;
                for a in self.advices() {
                    if a.is_active() || a.state().trigger_source == FORCE_SKIP_TEXT {
                        continue;
                    }
                    if m.data1 == a.guid() {
                        a.set_flags(flags::FORCED_TRIGGER | flags::DEBUG_GUID, true);
                    } else if m.data1 == id {
                        a.set_flags(flags::FORCED_TRIGGER, true);
                    }
                }
                // [choice] data1 = 1 (show the GUIDs).
                h.post(&OutMessage::new(msg::DEBUG_SHOW_GUIDS, 1, MsgObj::None));
                true
            }
            msg::DEBUG_PRINT => true,
            msg::DEBUG_REFRESH => {
                for (a, _) in self.current_advices() {
                    a.update_text();
                }
                true
            }
            msg::DEBUG_SHOW_GUIDS => {
                for (a, _) in self.current_advices() {
                    a.set_flags(flags::DEBUG_GUID, m.data1 != 0);
                }
                true
            }
            _ => true,
        }
    }

    // ---- records (§11.2, §11.5) ----

    /// The advisor record (§11.2).
    pub fn write_record(&self, out: &mut dyn OutStream) {
        let s = self.state();
        out.put_u16(ADVISOR_RECORD_VERSION);
        out.put_i32(s.table_index);
        out.put_str(&s.caption);
        out.put_str(&s.name);
        out.put_i32(s.month_counter);
        out.put_i32(s.current_day);
        out.put_i32(s.id);
        out.put_i32(s.cursor);
        out.put_u32(s.infos.len() as u32);
        for inf in &s.infos {
            out.put_ref(&RefValue::Object(inf.advice.clone()));
            out.put_i32(inf.expiry);
            out.put_i32(inf.last_month);
            out.put_i32(inf.next_day);
            out.put_i8(inf.advice.is_active() as i8);
            out.put_i32(inf.advice.event_id() as i32);
        }
        out.put_u32(s.queue.len() as u32);
        for (ev, subj) in &s.queue {
            out.put_i32(*ev as i32);
            let r = match subj {
                Some(x) => RefValue::Object(x.clone()),
                None => RefValue::Null,
            };
            out.put_ref(&r);
        }
    }

    /// Reads the advisor record: version 3. An advice reference that does
    /// not resolve to an advice is skipped [choice].
    pub fn read_record(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(ADVISOR_RECORD_VERSION) {
            return false;
        }
        let (Some(ti), Some(cap), Some(name)) = (inp.get_i32(), inp.get_str(), inp.get_str()) else {
            return false;
        };
        let (Some(mc), Some(cd), Some(id), Some(cur)) = (inp.get_i32(), inp.get_i32(), inp.get_i32(), inp.get_i32())
        else {
            return false;
        };
        let Some(n) = inp.get_u32() else { return false };
        let mut infos = Vec::new();
        let mut active = BTreeMap::new();
        let mut events: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for _ in 0..n {
            let Some(r) = inp.get_ref_as(iid::ADVICE) else {
                return false;
            };
            let (Some(expiry), Some(last_month), Some(next_day), Some(act), Some(ev)) =
                (inp.get_i32(), inp.get_i32(), inp.get_i32(), inp.get_i8(), inp.get_i32())
            else {
                return false;
            };
            let Some(a) = r.object::<Advice>() else { continue };
            let i = infos.len();
            if act != 0 {
                active.insert(a.serial(), i);
            }
            if ev != 0 {
                events.entry(ev as u32).or_default().push(i);
            }
            infos.push(AdviceInfo {
                advice: a,
                next_day,
                expiry,
                last_month,
            });
        }
        let Some(m) = inp.get_u32() else { return false };
        let mut queue = Vec::new();
        for _ in 0..m {
            let Some(ev) = inp.get_i32() else { return false };
            let Some(r) = inp.get_ref_as(iid::SUBJECT) else {
                return false;
            };
            queue.push((ev as u32, r.object::<AdviceSubject>()));
        }
        let mut s = self.s.borrow_mut();
        s.table_index = ti;
        s.caption = cap;
        s.name = name;
        s.month_counter = mc;
        s.current_day = cd;
        s.id = id;
        s.cursor = cur;
        s.infos = infos;
        s.active = active;
        s.events = events;
        s.queue = queue;
        inp.error() == 0
    }
}

impl SaveObject for Advisor {
    fn class_id(&self) -> u32 {
        self.kind.class_id()
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        if let Some(ext) = &self.ext {
            ext.write_prefix(out);
            self.write_record(out);
            return out.error() == 0;
        }
        if self.kind == AdvisorKind::CityPlanning {
            out.put_u16(CITY_PLANNING_RECORD_VERSION);
            self.write_record(out);
            let s = self.state();
            out.put_i32(s.building_count);
            out.put_i32(s.park_count);
            out.put_i32(s.recreation_count);
            out.put_u32(s.landmarks.len() as u32);
            for (k, v) in &s.landmarks {
                out.put_i32(*k);
                out.put_i32(*v);
            }
            return out.error() == 0;
        }
        self.write_record(out);
        out.error() == 0
    }

    /// §11.2; CityPlanning §11.5 (any version, a failed base read does not
    /// stop it, the landmarks only above version 2).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if let Some(ext) = &self.ext {
            if !ext.read_prefix(inp) {
                return false;
            }
            return self.read_record(inp);
        }
        if self.kind == AdvisorKind::CityPlanning {
            let Some(v) = inp.get_u16() else { return false };
            self.read_record(inp);
            let (b, p, r) = (inp.get_i32(), inp.get_i32(), inp.get_i32());
            if let (Some(b), Some(p), Some(r)) = (b, p, r) {
                let mut s = self.s.borrow_mut();
                s.building_count = b;
                s.park_count = p;
                s.recreation_count = r;
            }
            if v > 2 {
                let mut lm = BTreeMap::new();
                let n = inp.get_u32().unwrap_or(0);
                for _ in 0..n {
                    let (Some(k), Some(c)) = (inp.get_i32(), inp.get_i32()) else {
                        break;
                    };
                    lm.insert(k, c);
                }
                self.s.borrow_mut().landmarks = lm;
            }
            return inp.error() == 0;
        }
        self.read_record(inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
