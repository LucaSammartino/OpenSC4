// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The park manager (§2–§5, §7).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::rng::{derive_seed, FixedSeeds, RzRandomSuper};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::{
    escape_probability, msg, tuning_from, ParkHost, ParkMessage, ParkRandom, AGENT_FLAGS, AGENT_LEVEL, AGENT_NAME,
    CLSID_PARK_MANAGER, CONDITION_NORMAL, CONDITION_RUN_DOWN, DEFAULT_THRESHOLD, DEPARTMENT_GROUP, DEPARTMENT_ID,
    DEPARTMENT_NAME, DISPLAY_FLAG_SECONDARY, GROUP_PARK, GROUP_ZOO, IID_MESSAGE_TARGET2, IID_OCCUPANT,
    IID_PARK_MANAGER, IID_SERIALIZABLE, IID_UNKNOWN, MAX_FUNDING, MAX_LIST_COUNT, OCCUPANT_TYPE_BUILDING,
    PROP_PREVIEW_OCCUPANT, PURPOSE_PARK, RECORD_VERSION, ZOO_ESCAPE_AUTOMATA,
};

/// `QueryInterface` (§1): `0xAA3FE6BC`, `1`, `cIGZMessageTarget2` and
/// `cIGZSerializable`.
pub fn query_interface(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_PARK_MANAGER | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// Every field of §2 (the generator lives beside it).
#[derive(Clone, Debug)]
pub struct ParkState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    pub tuning_read: bool,
    /// The `underfunded` latch (not saved).
    pub underfunded: bool,
    /// Building occupants in group `0x1006`, each held (`None`: a reference
    /// that did not resolve on load).
    pub parks: Vec<Option<u64>>,
    /// Building occupants in group `0x1702`, each held.
    pub zoos: Vec<Option<u64>>,
    pub city_held: bool,
    pub budget_held: bool,
    pub message_server_held: bool,
    pub lot_manager_held: bool,
    /// The Parks and Recreation department, held.
    pub department: Option<u64>,
    pub distress_threshold: f32,
    pub escape_curve: ResponseCurve,
}

impl Default for ParkState {
    /// The constructor's values (§2).
    fn default() -> ParkState {
        ParkState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            tuning_read: false,
            underfunded: false,
            parks: Vec::new(),
            zoos: Vec::new(),
            city_held: false,
            budget_held: false,
            message_server_held: false,
            lot_manager_held: false,
            department: None,
            distress_threshold: DEFAULT_THRESHOLD,
            escape_curve: ResponseCurve::default(),
        }
    }
}

/// The park manager.
pub struct ParkManager {
    host: Rc<dyn ParkHost>,
    token: Cell<u64>,
    rng: RefCell<Box<dyn ParkRandom>>,
    s: RefCell<ParkState>,
}

impl ParkManager {
    /// A constructed (not initialised) manager, reference count 0. The
    /// member generator is seeded with `derive_seed(session_seed,
    /// 0xAA3FE6BC)` (§5 *[choice]*).
    pub fn new(host: Rc<dyn ParkHost>, token: u64, session_seed: u32) -> Rc<ParkManager> {
        let seed = derive_seed(session_seed, CLSID_PARK_MANAGER);
        let rng = RzRandomSuper::new(
            seed,
            &FixedSeeds {
                wall_clock: seed,
                process_clock: seed,
            },
        );
        Rc::new(ParkManager {
            host,
            token: Cell::new(token),
            rng: RefCell::new(Box::new(rng)),
            s: RefCell::new(ParkState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// Replaces the member generator (tests, replays).
    pub fn set_random(&self, rng: Box<dyn ParkRandom>) {
        *self.rng.borrow_mut() = rng;
    }

    pub fn snapshot(&self) -> ParkState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut ParkState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity (§8) ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release` (§8): above 1 it decrements; at 1 or less it runs
    /// `Shutdown`, destroys the object (here: marks it) and returns 0
    /// without decrementing first.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        self.s.borrow_mut().destroyed = true;
        0
    }

    // ---- lifecycle (§4.1) ----

    /// Reads the tuning (§6) into the state.
    fn read_tuning(&self) {
        let ex = self.host.tuning();
        let (threshold, curve) = tuning_from(ex.as_deref());
        let mut s = self.s.borrow_mut();
        if ex.is_some() {
            s.distress_threshold = threshold;
            s.escape_curve = curve;
        }
        // Without the exemplar nothing is read: the fields keep their values.
    }

    /// Slot 3 `Init` (§4.1). Returns true.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            s.initialized = true;
        }
        // 1. The tuning, once per object.
        if !self.s.borrow().tuning_read {
            self.read_tuning();
            self.s.borrow_mut().tuning_read = true;
        }
        // 2.
        if !self.s.borrow().city_held && h.acquire_city() {
            self.s.borrow_mut().city_held = true;
        }
        if !self.s.borrow().budget_held && h.acquire_budget() {
            self.s.borrow_mut().budget_held = true;
        }
        // 3. Found or created, then held.
        if self.s.borrow().department.is_none() {
            let dept = h.department_budget(DEPARTMENT_ID).or_else(|| {
                let d = h.create_department_budget(DEPARTMENT_ID, DEPARTMENT_GROUP)?;
                h.set_department_name(d, DEPARTMENT_NAME.0, DEPARTMENT_NAME.1);
                h.set_max_funding(d, MAX_FUNDING);
                Some(d)
            });
            if let Some(d) = dept {
                h.add_ref(d);
                self.s.borrow_mut().department = Some(d);
            }
        }
        // 4.
        if !self.s.borrow().lot_manager_held && h.acquire_lot_manager() {
            self.s.borrow_mut().lot_manager_held = true;
        }
        // 5.
        if !self.s.borrow().message_server_held && h.acquire_message_server() {
            self.s.borrow_mut().message_server_held = true;
            for m in msg::SUBSCRIPTIONS {
                h.subscribe(m);
            }
        }
        // 6.
        h.add_agent(AGENT_LEVEL, AGENT_NAME, AGENT_FLAGS);
        true
    }

    /// Slot 4 `Shutdown` (§4.1). The latch and the tuning are kept. Returns
    /// true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        let s = {
            let mut s = self.s.borrow_mut();
            if !s.initialized {
                return true;
            }
            s.initialized = false;
            let taken = (
                std::mem::take(&mut s.parks),
                std::mem::take(&mut s.zoos),
                s.message_server_held,
                s.department.take(),
                s.lot_manager_held,
                s.budget_held,
                s.city_held,
            );
            s.message_server_held = false;
            s.lot_manager_held = false;
            s.budget_held = false;
            s.city_held = false;
            taken
        };
        let (parks, zoos, ms, dept, lm, budget, city) = s;
        for t in parks.into_iter().chain(zoos).flatten() {
            h.release(t);
        }
        h.remove_agent();
        if ms {
            for m in msg::SUBSCRIPTIONS {
                h.unsubscribe(m);
            }
            h.release_message_server();
        }
        if let Some(d) = dept {
            h.release(d);
        }
        if lm {
            h.release_lot_manager();
        }
        if budget {
            h.release_budget();
        }
        if city {
            h.release_city();
        }
        true
    }

    // ---- messages (§3) ----

    /// `DoMessage` (§3). Returns true.
    pub fn do_message(&self, m: &ParkMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_MONTH => self.monthly(),
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if let Some(occ) = m.void1 {
                    if !self.host.occupant_has_property(occ, PROP_PREVIEW_OCCUPANT) {
                        if m.msg_type == msg::OCCUPANT_INSERTED {
                            self.occupant_inserted(occ);
                        } else {
                            self.occupant_removed(occ);
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }

    /// §4.2 (after the preview test).
    pub fn occupant_inserted(&self, occ: u64) {
        let h = &*self.host;
        if h.occupant_type(occ) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        // 1. Park.
        if h.is_occupant_group(occ, GROUP_PARK) {
            h.add_ref(occ);
            self.s.borrow_mut().parks.push(Some(occ));
            if let Some(item) = h.budget_item(occ, PURPOSE_PARK) {
                // 3. [compat] A failed building QueryInterface skips the zoo
                // test too.
                if !h.is_building_occupant(occ) {
                    return;
                }
                // 4. [choice] No department: steps 5–7 are skipped.
                if let Some(dept) = h.department_budget(item.department_id) {
                    self.charge_park(occ, dept, &item);
                }
                // 8. The building interface is the host's (released there).
            }
        }
        // 2. Zoo.
        if h.is_occupant_group(occ, GROUP_ZOO) {
            h.add_ref(occ);
            self.s.borrow_mut().zoos.push(Some(occ));
        }
    }

    /// §4.2 steps 5–7.
    fn charge_park(&self, occ: u64, dept: u64, item: &crate::budget::BudgetItem) {
        let h = &*self.host;
        let mut line = item.line_item;
        if line == 0 {
            line = h.building_type(occ);
            h.change_budget_item_line(occ, item.index, line);
            if line == 0 {
                return;
            }
        }
        let li = match h.line_item(dept, line) {
            Some(li) => Some(li),
            None => {
                let li = h.create_line_item(dept, line, false);
                if let Some(li) = li {
                    h.set_line_display_flag(li, DISPLAY_FLAG_SECONDARY, true);
                }
                li
            }
        };
        // [choice] A line that cannot be created is skipped.
        if let Some(li) = li {
            h.add_to_full_expenses(li, item.cost);
            h.set_line_secondary(li, h.line_secondary(li).wrapping_add(1));
        }
    }

    /// §4.3 (after the preview test).
    pub fn occupant_removed(&self, occ: u64) {
        let h = &*self.host;
        if h.occupant_type(occ) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        // 1. Park: every entry removed, one release [quirk].
        if h.is_occupant_group(occ, GROUP_PARK) {
            self.s.borrow_mut().parks.retain(|e| *e != Some(occ));
            h.release(occ);
            if let Some(item) = h.budget_item(occ, PURPOSE_PARK) {
                if let Some(li) = h.line_item_from_budget_item(&item) {
                    h.add_to_full_expenses(li, item.cost.wrapping_neg());
                    h.set_line_secondary(li, h.line_secondary(li).wrapping_sub(1));
                }
            }
        }
        // 2. Zoo.
        if h.is_occupant_group(occ, GROUP_ZOO) {
            self.s.borrow_mut().zoos.retain(|e| *e != Some(occ));
            h.release(occ);
        }
    }

    /// The monthly pass (§4.4).
    pub fn monthly(&self) {
        let h = &*self.host;
        // 1.
        if self.s.borrow().parks.is_empty() {
            return;
        }
        // 2. [choice] No department or no readable funding: stop.
        let Some(dept) = self.s.borrow().department else {
            return;
        };
        let Some(f) = h.department_funding(dept) else {
            return;
        };
        // 3. Distress (NaN is neither ≤ nor >).
        let c = {
            let mut s = self.s.borrow_mut();
            let t = s.distress_threshold;
            if f <= t && !s.underfunded {
                s.underfunded = true;
                Some(CONDITION_RUN_DOWN)
            } else if f > t && s.underfunded {
                s.underfunded = false;
                Some(CONDITION_NORMAL)
            } else {
                None
            }
        };
        // 4.
        if let Some(c) = c {
            let parks = self.s.borrow().parks.clone();
            for occ in parks.into_iter().flatten() {
                if let Some(lot) = h.occupant_position_lot(occ) {
                    h.set_lot_condition(lot, c);
                }
            }
        }
        // 5. Zoo escape.
        if !self.s.borrow().zoos.is_empty() {
            let p = escape_probability(&self.s.borrow().escape_curve, f);
            let r = self.rng.borrow_mut().uniform_f64();
            if r < p {
                self.zoo_escape();
            }
        }
    }

    /// The zoo escape (§4.5).
    pub fn zoo_escape(&self) {
        let h = &*self.host;
        let n = self.s.borrow().zoos.len() as u32;
        if n == 0 {
            return;
        }
        let i = self.rng.borrow_mut().uniform_u32(n) as usize;
        let Some(zoo) = self.s.borrow().zoos.get(i).copied().flatten() else {
            return;
        };
        h.post_zoo_escape(self.token.get(), zoo);
        h.add_controllers(ZOO_ESCAPE_AUTOMATA, zoo);
    }

    // ---- the record (§7) ----

    fn read_list(&self, inp: &mut dyn InStream, acquired: &mut Vec<u64>) -> Option<Vec<Option<u64>>> {
        let n = inp.get_u32()?;
        let mut v = Vec::new();
        if n >= MAX_LIST_COUNT {
            return Some(v);
        }
        for _ in 0..n {
            let r = inp.get_ref_as(IID_OCCUPANT)?;
            let t = if r.is_null() {
                None
            } else {
                self.host.resolve_ref(&r, IID_OCCUPANT)
            };
            if let Some(t) = t {
                acquired.push(t);
            }
            v.push(t);
        }
        Some(v)
    }

    fn parse(&self, inp: &mut dyn InStream, acquired: &mut Vec<u64>) -> Option<(OccupantList, OccupantList)> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let parks = self.read_list(inp, acquired)?;
        let zoos = self.read_list(inp, acquired)?;
        if inp.error() != 0 {
            return None;
        }
        Some((parks, zoos))
    }
}

/// A list of §7 (`None`: a reference that did not resolve).
type OccupantList = Vec<Option<u64>>;

fn write_list(h: &dyn ParkHost, out: &mut dyn OutStream, list: &[Option<u64>]) {
    out.put_u32(list.len() as u32);
    for e in list {
        out.put_ref(&e.map_or(RefValue::Null, |t| h.object_ref(t)));
    }
}

impl SaveObject for ParkManager {
    fn class_id(&self) -> u32 {
        CLSID_PARK_MANAGER
    }

    /// §7: version 2, the two lists. True unless the stream reports an
    /// error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let (parks, zoos) = {
            let s = self.s.borrow();
            (s.parks.clone(), s.zoos.clone())
        };
        out.put_u16(RECORD_VERSION);
        write_list(&*self.host, out, &parks);
        write_list(&*self.host, out, &zoos);
        out.error() == 0
    }

    /// §7: false unless the version is exactly 2. [choice] Atomic.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let mut acquired = Vec::new();
        match self.parse(inp, &mut acquired) {
            Some((parks, zoos)) => {
                let mut s = self.s.borrow_mut();
                // Replaced without releasing the old entries (§7).
                s.parks = parks;
                s.zoos = zoos;
                true
            }
            None => {
                for t in acquired {
                    self.host.release(t);
                }
                false
            }
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
