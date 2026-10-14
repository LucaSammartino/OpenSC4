// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The civic building simulator (§2–§4).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::{
    default_random_factory, department, msg, prop, temporary_seed, CivicHost, CivicMessage, CivicRandomFactory,
    ConditionalBuildingStatus, StlHashMap, StlHashSet, AGENT_FLAGS, AGENT_LEVEL, AGENT_NAME, APE_ESCAPE_AUTOMATA,
    APE_ESCAPE_CHANCE, CONDITION_NORMAL, CONDITION_RUN_DOWN, DEPARTMENT_NAME_GROUP, FUNDING_THRESHOLD,
    GROUP_DISEASE_CENTER, HASH_HINT, IID_CIVIC_BUILDING_SIMULATOR, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN,
    LINE_TYPE_INCOME, LOT_STATE_OCCUPIED, MAX_FUNDING, OCCUPANT_TYPE_BUILDING, PURPOSE_INCOME, PURPOSE_MAINTENANCE,
};

/// `QueryInterface` (§1): its own IID, `1`, `cIGZMessageTarget2` and
/// `cIGZSerializable`.
pub fn query_interface(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_CIVIC_BUILDING_SIMULATOR | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// Every field of §2, plus the temporary-generator counter of §5.
#[derive(Clone, Debug)]
pub struct CivicState {
    pub ref_count: u32,
    pub initialized: bool,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    /// The Landmarks department, held.
    pub landmarks: Option<u64>,
    pub exclusions: StlHashSet,
    pub statuses: StlHashMap<ConditionalBuildingStatus>,
    /// Occupants in group `0x1917`, each held. `None`: a reference that did
    /// not resolve on load.
    pub disease_centers: Vec<Option<u64>>,
    /// Occupants whose maintenance this simulator charged, each held.
    pub maintained: Vec<Option<u64>>,
    /// `n` of §5: the temporary generators made so far (not saved).
    pub temporaries: u32,
}

impl Default for CivicState {
    /// The constructor's values (§2).
    fn default() -> CivicState {
        CivicState {
            ref_count: 0,
            initialized: false,
            destroyed: false,
            landmarks: None,
            exclusions: StlHashSet::with_hint(HASH_HINT),
            statuses: StlHashMap::with_hint(HASH_HINT),
            disease_centers: Vec::new(),
            maintained: Vec::new(),
            temporaries: 0,
        }
    }
}

/// The civic building simulator.
pub struct CivicSim {
    pub(super) host: Rc<dyn CivicHost>,
    token: Cell<u64>,
    session_seed: Cell<u32>,
    rng_factory: RefCell<CivicRandomFactory>,
    pub(super) s: RefCell<CivicState>,
}

impl CivicSim {
    /// A constructed (not initialised) simulator, reference count 0.
    /// `session_seed` is foundations §6.3's `S`.
    pub fn new(host: Rc<dyn CivicHost>, token: u64, session_seed: u32) -> Rc<CivicSim> {
        Rc::new(CivicSim {
            host,
            token: Cell::new(token),
            session_seed: Cell::new(session_seed),
            rng_factory: RefCell::new(default_random_factory()),
            s: RefCell::new(CivicState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn session_seed(&self) -> u32 {
        self.session_seed.get()
    }

    pub fn set_session_seed(&self, seed: u32) {
        self.session_seed.set(seed);
    }

    /// Replaces the factory of the ape escape's temporary generators (tests,
    /// replays).
    pub fn set_random_factory(&self, f: CivicRandomFactory) {
        *self.rng_factory.borrow_mut() = f;
    }

    pub fn snapshot(&self) -> CivicState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut CivicState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release` *[question]* (see the module documentation): above
    /// 1 it decrements; at 1 or less it runs `Shutdown`, marks the object
    /// destroyed and returns 0.
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

    // ---- lifecycle (§4.1, §4.2) ----

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
        // Step 1: the Landmarks department, held, unless one was loaded.
        if self.s.borrow().landmarks.is_none() {
            let (id, group, name) = department::LANDMARKS;
            if let Some(d) = h.create_department_budget(id, group) {
                h.add_ref(d);
                h.set_department_name(d, DEPARTMENT_NAME_GROUP, name);
                h.set_max_funding(d, MAX_FUNDING);
                self.s.borrow_mut().landmarks = Some(d);
            }
        }
        // Steps 2 and 3: created when missing, not held.
        for (id, group, name) in [department::BUILDING_MAINTENANCE, department::BUSINESS_DEALS] {
            if h.department_budget(id).is_none() {
                if let Some(d) = h.create_department_budget(id, group) {
                    h.set_department_name(d, DEPARTMENT_NAME_GROUP, name);
                    h.set_max_funding(d, MAX_FUNDING);
                }
            }
        }
        // Steps 4–6.
        for m in msg::SUBSCRIPTIONS {
            h.subscribe(m);
        }
        h.add_agent(AGENT_LEVEL, AGENT_NAME, AGENT_FLAGS);
        h.register_lua();
        true
    }

    /// Slot 4 `Shutdown` (§4.2). The hash containers are not cleared
    /// *[compat]*. Returns true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        if !self.s.borrow().initialized {
            return true;
        }
        h.unregister_lua();
        let centers = std::mem::take(&mut self.s.borrow_mut().disease_centers);
        for t in centers.into_iter().flatten() {
            h.release(t);
        }
        let maintained = std::mem::take(&mut self.s.borrow_mut().maintained);
        for t in maintained.into_iter().flatten() {
            h.release(t);
        }
        h.remove_agent();
        let landmarks = self.s.borrow_mut().landmarks.take();
        if let Some(d) = landmarks {
            h.release(d);
        }
        for m in msg::SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        self.s.borrow_mut().initialized = false;
        true
    }

    // ---- messages (§3) ----

    /// `DoMessage` (§3). Returns true.
    pub fn do_message(&self, m: &CivicMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_MONTH => self.do_monthly_simulation(),
            msg::OCCUPANT_INSERTED => {
                if let Some(occ) = m.void1 {
                    self.occupant_changed(occ, true);
                }
            }
            msg::OCCUPANT_REMOVED => {
                if let Some(occ) = m.void1 {
                    self.occupant_changed(occ, false);
                }
            }
            msg::LOT_STATE_CHANGED => {
                if let Some(lot) = m.unknown.and_then(|u| self.host.message_lot(u)) {
                    if m.data2 == LOT_STATE_OCCUPIED {
                        self.do_lot_activated(lot);
                    }
                    self.host.release(lot);
                }
            }
            _ => {}
        }
        true
    }

    /// §4.3: an occupant inserted (`inserted`) or removed. No preview test
    /// *[quirk]*.
    pub fn occupant_changed(&self, occ: u64, inserted: bool) {
        let h = &*self.host;
        if h.occupant_type(occ) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        // 1. Count.
        if h.occupant_bool_property(occ, prop::CONDITIONAL_BUILDING) == Some(true) && h.is_building_occupant(occ) {
            if let Some(key) = h.building_key_instance(occ) {
                let mut s = self.s.borrow_mut();
                if let Some(st) = s.statuses.get_mut(key) {
                    st.building_count = if inserted {
                        st.building_count.wrapping_add(1)
                    } else {
                        st.building_count.wrapping_sub(1)
                    };
                }
            }
        }
        // 2. Exclusion.
        if let Some(id) = h.occupant_u32_property(occ, prop::EXCLUSION_ID) {
            if inserted {
                self.s.borrow_mut().exclusions.insert(id);
                return;
            }
            self.s.borrow_mut().exclusions.remove(id);
        }
        // 3. Removal.
        if !inserted && h.is_building_occupant(occ) {
            self.do_building_removed(occ);
        }
    }

    /// `DoLotActivated(lot)` (§4.4).
    pub fn do_lot_activated(&self, lot: u64) {
        let h = &*self.host;
        let Some(b) = h.lot_building(lot) else {
            return;
        };
        // 2. Maintenance.
        if let Some(item) = h.budget_item(b, PURPOSE_MAINTENANCE) {
            let mut line = item.line_item;
            if line == 0 {
                line = h.building_type(b);
                h.change_budget_item_line(b, item.index, line);
            }
            if let Some(dept) = h.department_budget(item.department_id) {
                let li = h
                    .line_item(dept, line)
                    .or_else(|| h.create_line_item(dept, line, false));
                if let Some(li) = li {
                    h.add_to_full_expenses(li, item.cost);
                }
            }
            // [quirk] No duplicate check.
            h.add_ref(b);
            self.s.borrow_mut().maintained.push(Some(b));
        }
        // 3. Income, keyed by the building type.
        if let Some(item) = h.budget_item(b, PURPOSE_INCOME) {
            if let Some(dept) = h.department_budget(item.department_id) {
                let ty = h.building_type(b);
                let li = match h.line_item(dept, ty) {
                    Some(li) => Some(li),
                    None => {
                        let li = h.create_line_item(dept, ty, false);
                        if let Some(li) = li {
                            h.set_line_type(li, LINE_TYPE_INCOME);
                        }
                        li
                    }
                };
                // [choice] (CB5) No line: nothing is credited.
                if let Some(li) = li {
                    h.add_to_income(li, item.cost);
                }
            }
        }
        // 4. Disease Research Center.
        if h.is_occupant_group(b, GROUP_DISEASE_CENTER) {
            let listed = self.s.borrow().disease_centers.contains(&Some(b));
            if !listed {
                self.s.borrow_mut().disease_centers.push(Some(b));
                h.add_ref(b);
            }
        }
    }

    /// `DoBuildingRemoved(b)` (§4.5).
    pub fn do_building_removed(&self, b: u64) {
        let h = &*self.host;
        // 1. Maintenance.
        if let Some(item) = h.budget_item(b, PURPOSE_MAINTENANCE) {
            h.remove_budget_item(b, item.index);
            let found = {
                let mut s = self.s.borrow_mut();
                let pos = s.maintained.iter().position(|e| *e == Some(b));
                pos.map(|p| s.maintained.remove(p))
            };
            if found.is_some() {
                h.release(b);
            }
        }
        // 2. Income.
        if let Some(item) = h.budget_item(b, PURPOSE_INCOME) {
            if let Some(dept) = h.department_budget(item.department_id) {
                let ty = h.building_type(b);
                if let Some(li) = h.line_item(dept, ty) {
                    h.set_line_income(li, h.line_income(li).wrapping_sub(item.cost));
                    if h.line_income(li) < 1 {
                        h.remove_line_item(dept, ty);
                    }
                }
            }
        }
        // 3. Disease Research Center: the first matching entry.
        if h.is_occupant_group(b, GROUP_DISEASE_CENTER) {
            let found = {
                let mut s = self.s.borrow_mut();
                let pos = s.disease_centers.iter().position(|e| *e == Some(b));
                pos.map(|p| s.disease_centers.remove(p))
            };
            if found.is_some() {
                h.release(b);
            }
        }
    }

    /// `DoMonthlySimulation()` (§4.6).
    pub fn do_monthly_simulation(&self) {
        let h = &*self.host;
        // 1. Ape escape.
        let n = self.s.borrow().disease_centers.len();
        if n > 0 {
            let k = {
                let mut s = self.s.borrow_mut();
                let k = s.temporaries;
                s.temporaries = s.temporaries.wrapping_add(1);
                k
            };
            let seed = temporary_seed(self.session_seed.get(), k);
            let mut rng = (self.rng_factory.borrow())(seed);
            if rng.uniform_f64() <= APE_ESCAPE_CHANCE {
                let i = rng.uniform_u32(n as u32) as usize;
                let occ = self.s.borrow().disease_centers.get(i).copied().flatten();
                // [choice] A null entry escapes nothing.
                if let Some(occ) = occ {
                    h.add_controllers(APE_ESCAPE_AUTOMATA, occ);
                }
            }
        }
        // 2. Run-down look, in list order (a snapshot: the host cannot
        // change the list).
        let maintained = self.s.borrow().maintained.clone();
        for occ in maintained.into_iter().flatten() {
            let Some(item) = h.budget_item(occ, PURPOSE_MAINTENANCE) else {
                continue;
            };
            let Some(dept) = h.department_budget(item.department_id) else {
                continue;
            };
            let Some(lot) = h.occupant_lot(occ) else {
                continue;
            };
            let Some(f) = h.department_funding(dept) else {
                continue;
            };
            if f < FUNDING_THRESHOLD {
                if h.lot_condition(lot) == CONDITION_NORMAL {
                    h.set_lot_condition(lot, CONDITION_RUN_DOWN);
                }
            } else if h.lot_condition(lot) == CONDITION_RUN_DOWN {
                h.set_lot_condition(lot, CONDITION_NORMAL);
            }
        }
    }

    // ---- the interface (§4.7) ----

    /// Slot 5 `IsBuildingExcluded(id)`.
    pub fn is_building_excluded(&self, id: u32) -> bool {
        self.s.borrow().exclusions.contains(id)
    }

    /// Slot 6 `GetConditionalBuildingStatus(key)`: a copy of the stored
    /// status, or `None`. The original returns a pointer that callers write
    /// through; that is [`with_status_mut`](Self::with_status_mut).
    pub fn conditional_building_status(&self, key: u32) -> Option<ConditionalBuildingStatus> {
        self.s.borrow().statuses.get(key).cloned()
    }

    /// Writes through the stored status of `key` (the advisors set bits
    /// through slot 6's pointer). `None` when there is no entry.
    pub fn with_status_mut<R>(&self, key: u32, f: impl FnOnce(&mut ConditionalBuildingStatus) -> R) -> Option<R> {
        self.s.borrow_mut().statuses.get_mut(key).map(f)
    }

    /// Slot 7 `UpdateConditionalBuildingStatus(key, status)`: find or insert
    /// `key` (a new entry starts as `{0, 0, ""}`), then copy the status,
    /// count and text. Returns true (the spec's *[choice]*).
    pub fn update_conditional_building_status(&self, key: u32, status: &ConditionalBuildingStatus) -> bool {
        let mut s = self.s.borrow_mut();
        let e = s.statuses.entry(key, ConditionalBuildingStatus::default);
        e.status = status.status;
        e.building_count = status.building_count;
        e.tooltip = status.tooltip.clone();
        true
    }

    /// Slot 8 `GetLandmarkCount()`: the length of `maintained` *[quirk: the
    /// name]*.
    pub fn landmark_count(&self) -> u32 {
        self.s.borrow().maintained.len() as u32
    }
}
