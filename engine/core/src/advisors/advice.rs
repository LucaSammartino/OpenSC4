// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4Advice`, `cSC4RewardAdvice` and `cSC4AdviceNeghborDeal` (§2.2,
//! §2.3, §3.3, §4.5, §6.1, §6.6, §6.9, §12.3) and their records (§11.3).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::subject::AdviceSubject;
use super::system::Shared;
use super::{
    flags, iid, status_bits, AdviceEntry, AdviceKind, LuaResult, ADVICE_RECORD_VERSION, DEFAULT_TRIGGER,
    ND_RECORD_VERSION, NO_TRIGGER,
};

/// The advice's fields (§3.3).
#[derive(Clone, Default)]
pub struct AdviceState {
    pub table_index: i32,
    pub guid: u32,
    pub flags: u32,
    pub advisor_id: i32,
    pub priority: i32,
    pub frequency: i32,
    pub timeout: i32,
    pub mood: i32,
    pub effects: Vec<u32>,
    pub subject: Option<Rc<AdviceSubject>>,
    pub event_id: u32,
    pub command_id: u32,
    /// The compiled trigger reference ([`NO_TRIGGER`]: none).
    pub trigger: i32,
    /// The trigger text kept for display (debug functionality only).
    pub trigger_text: Option<Vec<u8>>,
    /// The text the trigger was compiled from (for `Clone`).
    pub trigger_source: Vec<u8>,
    /// True between `Read` and `Init`.
    pub loaded: bool,
    /// `nd_type` (neighbour-deal advices).
    pub nd_type: i32,
}

impl std::fmt::Debug for AdviceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdviceState")
            .field("guid", &format_args!("{:#x}", self.guid))
            .field("flags", &format_args!("{:#x}", self.flags))
            .field("table_index", &self.table_index)
            .field("advisor_id", &self.advisor_id)
            .field("subject", &self.subject.as_ref().map(|s| s.serial()))
            .finish()
    }
}

/// One advice.
pub struct Advice {
    pub(crate) shared: Rc<Shared>,
    serial: u32,
    class_id: u32,
    kind: AdviceKind,
    token: Cell<u64>,
    s: RefCell<AdviceState>,
}

impl Advice {
    pub(crate) fn new(shared: &Rc<Shared>, class_id: u32, kind: AdviceKind) -> Rc<Advice> {
        let a = Rc::new(Advice {
            shared: shared.clone(),
            serial: shared.next_serial(),
            class_id,
            kind,
            token: Cell::new(0),
            s: RefCell::new(AdviceState {
                trigger: NO_TRIGGER,
                ..AdviceState::default()
            }),
        });
        a.token.set(shared.created(a.clone()));
        a
    }

    /// The serial (also the session ID, slot 13 [choice]).
    pub fn serial(&self) -> u32 {
        self.serial
    }
    pub fn class_id(&self) -> u32 {
        self.class_id
    }
    pub fn kind(&self) -> AdviceKind {
        self.kind
    }
    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }
    pub fn state(&self) -> AdviceState {
        self.s.borrow().clone()
    }
    /// Pokes the state (tests).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut AdviceState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    /// `QueryInterface` (§1.1).
    pub fn query_interface(&self, i: u32) -> bool {
        matches!(i, iid::UNKNOWN | iid::ADVICE | iid::SERIALIZABLE)
            || (self.kind == AdviceKind::NeighborDeal && i == iid::NEIGHBOR_DEAL_ADVICE)
    }

    // ---- getters (§12.3) ----

    pub fn mood(&self) -> i32 {
        self.s.borrow().mood
    }
    pub fn priority(&self) -> i32 {
        self.s.borrow().priority
    }
    pub fn guid(&self) -> u32 {
        self.s.borrow().guid
    }
    pub fn advisor_id(&self) -> i32 {
        self.s.borrow().advisor_id
    }
    /// Slot 12: a replacement returns `this` (§12.3).
    pub fn set_advisor_id(&self, id: i32) {
        self.s.borrow_mut().advisor_id = id;
    }
    pub fn session_id(&self) -> i32 {
        self.serial as i32
    }
    pub fn frequency(&self) -> i32 {
        self.s.borrow().frequency
    }
    pub fn timeout(&self) -> i32 {
        self.s.borrow().timeout
    }
    pub fn event_id(&self) -> u32 {
        self.s.borrow().event_id
    }
    pub fn command_id(&self) -> u32 {
        self.s.borrow().command_id
    }
    pub fn table_index(&self) -> i32 {
        self.s.borrow().table_index
    }
    pub fn flags(&self) -> u32 {
        self.s.borrow().flags
    }
    pub fn event_subject(&self) -> Option<Rc<AdviceSubject>> {
        self.s.borrow().subject.clone()
    }
    pub fn trigger_text(&self) -> Option<Vec<u8>> {
        self.s.borrow().trigger_text.clone()
    }
    pub fn nd_type(&self) -> i32 {
        self.s.borrow().nd_type
    }
    pub fn is_active(&self) -> bool {
        self.check_flags(flags::ACTIVE)
    }

    /// Slot 15 `CheckFlags(mask)`: all bits of `mask`.
    pub fn check_flags(&self, mask: u32) -> bool {
        self.s.borrow().flags & mask == mask
    }

    // ---- Init / Shutdown (§4.5) ----

    /// Slot 3 `Init(i)` (§4.5).
    pub fn init(self: &Rc<Self>, table_index: i32) -> bool {
        let h = self.shared.host.clone();
        let entry = if table_index > 0 {
            h.advice_entry(table_index as u32)
        } else {
            None
        };
        let Some(e) = entry else { return false };
        let (loaded, active, old_trigger) = {
            let s = self.s.borrow();
            (s.loaded, s.flags & flags::ACTIVE != 0, s.trigger)
        };
        // 1. A loaded advice keeps its GUID.
        if loaded && e.guid != self.guid() {
            return false;
        }
        // 3. Compile the trigger. [choice] A previous reference is released.
        if old_trigger != NO_TRIGGER {
            h.release_trigger(old_trigger);
        }
        let text = match &e.trigger {
            Some(t) if !t.is_empty() => t.clone(),
            _ => DEFAULT_TRIGGER.to_vec(),
        };
        let compiled = h.compile_trigger(&text);
        let debug = h.debug_enabled();
        {
            let mut s = self.s.borrow_mut();
            s.table_index = table_index;
            s.guid = e.guid;
            // 2.
            s.priority = e.priority;
            s.mood = e.mood;
            s.frequency = e.frequency;
            s.timeout = e.timeout;
            s.trigger = compiled.unwrap_or(NO_TRIGGER);
            s.trigger_text = debug.then(|| text.clone());
            s.trigger_source = text;
            // 4.
            let set = |f: &mut u32, bit: u32, on: bool| {
                if on {
                    *f |= bit
                } else {
                    *f &= !bit
                }
            };
            set(&mut s.flags, flags::ONCE, e.once);
            set(&mut s.flags, flags::NEWS_ONLY, e.news_only);
            set(&mut s.flags, flags::EVENT, e.event != 0);
            set(&mut s.flags, flags::PERSIST, e.persist);
            set(&mut s.flags, flags::NO_TIMEOUT, e.no_timeout);
            s.event_id = e.event;
            s.command_id = e.command;
            if e.class_id != self.class_id {
                s.flags |= flags::CLASS_MISMATCH;
            }
            // 5. Appended *[quirk]*.
            s.effects.extend_from_slice(&e.effects);
            match self.kind {
                AdviceKind::Reward => s.flags |= flags::REWARD,
                AdviceKind::NeighborDeal => s.nd_type = e.nd_type,
                AdviceKind::Plain => {}
            }
        }
        // 6.
        if active && !loaded {
            self.update_text();
        }
        self.s.borrow_mut().loaded = false;
        true
    }

    /// Slot 4 `Shutdown` → true. [choice] Releases the compiled trigger and
    /// the event subject (the spec gives no body).
    pub fn shutdown(&self) -> bool {
        let (t, subject) = {
            let mut s = self.s.borrow_mut();
            let t = std::mem::replace(&mut s.trigger, NO_TRIGGER);
            (t, s.subject.take())
        };
        if t != NO_TRIGGER {
            self.shared.host.release_trigger(t);
        }
        drop(subject);
        true
    }

    /// Slot 5 `Clone` (§6.3, Utility). [choice] A new advice of the same
    /// class with a copy of the fields, inactive, without a subject, and its
    /// own trigger compiled from the same text.
    pub fn clone_advice(&self) -> Rc<Advice> {
        let c = Advice::new(&self.shared, self.class_id, self.kind);
        let mut st = self.state();
        st.flags &= !flags::ACTIVE;
        st.subject = None;
        st.trigger = self
            .shared
            .host
            .compile_trigger(&st.trigger_source)
            .unwrap_or(NO_TRIGGER);
        *c.s.borrow_mut() = st;
        c
    }

    // ---- flags, text and effects (§6.9) ----

    /// Slot 14 `SetFlags(mask, on)` → true.
    pub fn set_flags(self: &Rc<Self>, mask: u32, on: bool) -> bool {
        let was_active = self.is_active();
        if on {
            self.s.borrow_mut().flags |= mask;
            if mask & flags::ACTIVE != 0 {
                if self.check_flags(flags::TEXT_DIRTY) {
                    self.update_text();
                }
                if !was_active {
                    self.notify_global_effects();
                }
            }
        } else {
            self.s.borrow_mut().flags &= !mask;
            if mask & flags::ACTIVE != 0 {
                self.s.borrow_mut().flags |= flags::TEXT_DIRTY;
                if was_active {
                    self.notify_global_effects();
                }
            }
        }
        if mask & flags::DEBUG_GUID != 0 {
            self.s.borrow_mut().flags |= flags::TEXT_DIRTY;
        }
        true
    }

    /// Slot 23 `NotifyGlobalEffectTriggerChanged`: assert every effect while
    /// active, else retract.
    pub fn notify_global_effects(&self) {
        let (effects, active) = {
            let s = self.s.borrow();
            (s.effects.clone(), s.flags & flags::ACTIVE != 0)
        };
        for id in effects {
            self.shared.host.global_effect(id, active);
        }
    }

    /// Slot 28 `UpdateText` (class slot 30): the host's rebuild (UI); the
    /// text is clean afterwards [inferred].
    pub fn update_text(self: &Rc<Self>) -> bool {
        self.shared.host.rebuild_text(self);
        self.s.borrow_mut().flags &= !flags::TEXT_DIRTY;
        true
    }

    /// Slot 24 `SetEventSubject` (sets text dirty) → true.
    pub fn set_event_subject(&self, subject: Option<Rc<AdviceSubject>>) -> bool {
        let old = {
            let mut s = self.s.borrow_mut();
            s.flags |= flags::TEXT_DIRTY;
            std::mem::replace(&mut s.subject, subject)
        };
        drop(old);
        true
    }

    // ---- triggers (§6.1, §6.6) ----

    /// `get_raw_trigger` (§6.1).
    pub fn raw_trigger(self: &Rc<Self>) -> bool {
        let sys = self.shared.sys();
        if let Some(sys) = &sys {
            sys.push_context_advice(self.clone());
        }
        let r = self.s.borrow().trigger;
        let v = if r == NO_TRIGGER {
            false
        } else {
            match self.shared.host.call_trigger(r) {
                LuaResult::Bool(b) => b,
                // A number is true unless it equals 0.0 (NaN → true).
                LuaResult::Number(n) => n != 0.0,
                _ => false,
            }
        };
        if let Some(sys) = &sys {
            sys.pop_context_advice();
        }
        v
    }

    /// Slot 8 `GetTrigger` (§6.1; reward advices §6.6).
    pub fn get_trigger(self: &Rc<Self>) -> bool {
        if self.kind == AdviceKind::Reward {
            return self.reward_trigger();
        }
        if self.check_flags(flags::FORCED_TRIGGER) {
            return true;
        }
        self.raw_trigger() && !self.check_flags(flags::NO_TEXT)
    }

    /// `cSC4RewardAdvice::GetTrigger` (§6.6).
    fn reward_trigger(self: &Rc<Self>) -> bool {
        let h = self.shared.host.clone();
        let sys = self.shared.sys();
        if let Some(sys) = &sys {
            sys.push_context_advice(self.clone());
        }
        let (guid, index) = {
            let s = self.s.borrow();
            (s.guid, s.table_index)
        };
        let mut st = h.conditional_status(guid).unwrap_or_default();
        let mut trigger = false;
        match h.call_condition(index) {
            LuaResult::Str(text) => {
                st.status &= !3;
                st.tooltip = h.detokenize(&text);
            }
            LuaResult::Number(n) => {
                if n == 0.0 {
                    st.status = (st.status & !status_bits::AVAILABLE) | status_bits::HIDDEN;
                } else if n == 1.0 {
                    if st.status & status_bits::ADVISOR_EVENT == 0 {
                        let init = h.reward_subject(guid).unwrap_or_default();
                        let subject = AdviceSubject::from_init(&self.shared, &init);
                        self.set_event_subject(Some(subject));
                        trigger = true;
                    }
                    st.status = status_bits::AVAILABLE | status_bits::ADVISOR_EVENT;
                }
            }
            _ => {}
        }
        h.update_conditional_status(guid, &st);
        if let Some(sys) = &sys {
            sys.pop_context_advice();
        }
        trigger && !self.check_flags(flags::NO_TEXT)
    }

    // ---- records (§11.3) ----

    fn write_advice(&self, out: &mut dyn OutStream) {
        let s = self.state();
        out.put_u16(ADVICE_RECORD_VERSION);
        out.put_i32(s.table_index);
        out.put_u32(s.guid);
        out.put_u32(s.flags);
        out.put_i32(s.advisor_id);
        out.put_i32(s.priority);
        out.put_i32(s.frequency);
        out.put_i32(s.timeout);
        out.put_i32(s.mood);
        out.put_u32(s.effects.len() as u32);
        for e in &s.effects {
            out.put_u32(*e);
        }
        let r = match &s.subject {
            Some(sub) => RefValue::Object(sub.clone()),
            None => RefValue::Null,
        };
        out.put_ref(&r);
    }

    /// The advice record's `Read`: the version is not checked; the effects
    /// are read only when the version is above 3, and appended *[quirk]*.
    fn read_advice(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else { return false };
        let mut f = [0u32; 8];
        for v in f.iter_mut() {
            match inp.get_u32() {
                Some(x) => *v = x,
                None => return false,
            }
        }
        let mut effects = Vec::new();
        if version > 3 {
            let Some(n) = inp.get_u32() else { return false };
            for _ in 0..n {
                match inp.get_u32() {
                    Some(e) => effects.push(e),
                    None => return false,
                }
            }
        }
        {
            let mut s = self.s.borrow_mut();
            s.table_index = f[0] as i32;
            s.guid = f[1];
            s.flags = f[2];
            s.advisor_id = f[3] as i32;
            s.priority = f[4] as i32;
            s.frequency = f[5] as i32;
            s.timeout = f[6] as i32;
            s.mood = f[7] as i32;
            s.effects.extend(effects);
        }
        // The subject is released first, then read.
        let old = self.s.borrow_mut().subject.take();
        drop(old);
        let subject = match inp.get_ref_as(iid::SUBJECT) {
            Some(r) => r.object::<AdviceSubject>(),
            None => None,
        };
        let ok = inp.error() == 0;
        let mut s = self.s.borrow_mut();
        s.subject = subject;
        s.flags |= flags::TEXT_DIRTY;
        s.loaded = ok;
        ok
    }
}

impl Drop for Advice {
    fn drop(&mut self) {
        let t = self.s.get_mut().trigger;
        if t != NO_TRIGGER {
            self.shared.host.release_trigger(t);
        }
    }
}

impl SaveObject for Advice {
    fn class_id(&self) -> u32 {
        self.class_id
    }

    /// §11.3. The neighbour-deal advice writes `u16 2`, the advice record
    /// and `s32 dealType`.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        if self.kind == AdviceKind::NeighborDeal {
            out.put_u16(ND_RECORD_VERSION);
            self.write_advice(out);
            out.put_i32(self.nd_type());
        } else {
            self.write_advice(out);
        }
        out.error() == 0
    }

    /// §11.3. The neighbour-deal advice requires version 2.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if self.kind == AdviceKind::NeighborDeal {
            if inp.get_u16() != Some(ND_RECORD_VERSION) {
                return false;
            }
            if !self.read_advice(inp) {
                return false;
            }
            let Some(t) = inp.get_i32() else { return false };
            self.s.borrow_mut().nd_type = t;
            inp.error() == 0
        } else {
            self.read_advice(inp)
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

/// An advice's entry fields as `Init` reads them (tests and the host).
pub fn entry_matches(e: &AdviceEntry, advice_type: i32) -> bool {
    e.advice_type == advice_type && e.guid != 0 && e.class_id != 0
}
