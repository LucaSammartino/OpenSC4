// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The city situation manager (§2–§8).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::rng::RzRandomSuper;
use crate::stream::{InStream, OutStream, SaveObject};

use super::{
    button, can_player_drive, msg, notify, parse_cheat, status, CitySituationHost, Popup, Script, SituationMessage,
    AUTO_FLAG_DISABLED, AUTO_FLAG_LONG_END, CHEAT_ID, CLSID_CITY_SITUATION, COUNTDOWN_MS, DEFAULT_END_DELAY,
    IID_CITY_SITUATION, IID_MESSAGE_TARGET, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN, INDICATOR_KIND,
    MAX_SUCCESS_COUNT, MIN_TIME_LIMIT, MISSION_TYPES_NEXT_TARGET, MISSION_TYPE_COVERAGE, MISSION_TYPE_NO_TARGET,
    MS_TO_SECONDS, PROP_PREVIEW_OCCUPANT, RECORD_VERSION, REWARD_SLOTS, SITUATION_FLAG_MYSIM_WINDOW,
    SITUATION_FLAG_REWARD, SITUATION_FLAG_WINDOW, WARNING_SECONDS, WARNING_SOUND,
};

/// `QueryInterface` (§1): the five IIDs and their sub-object offsets.
/// *[quirk]* `cIGZUnknown` answers the serializable sub-object (+0xC).
pub fn query_interface(iid: u32) -> Option<u32> {
    match iid {
        IID_CITY_SITUATION => Some(0),
        IID_UNKNOWN | IID_SERIALIZABLE => Some(0xC),
        IID_MESSAGE_TARGET2 => Some(4),
        IID_MESSAGE_TARGET => Some(8),
        _ => None,
    }
}

/// One entry of `trackList` (§2): the advice, its tracked vehicle and the
/// indicator handle (both borrowed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackEntry {
    pub advice: u64,
    pub automaton: Option<u64>,
    pub handle: u32,
}

/// Every field of §2.
#[derive(Clone, Debug, PartialEq)]
pub struct CitySituationState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    pub indicators_visible: bool,
    pub loaded_from_save: bool,
    /// Held.
    pub active_situation: Option<u64>,
    /// Held.
    pub automata_managers: Vec<u64>,
    pub track_list: Vec<TrackEntry>,
    pub tracked: BTreeSet<u64>,
    pub success_counts: BTreeMap<u32, u16>,
    pub status: u32,
    pub active_automaton: Option<u64>,
    pub current_target: Option<u64>,
    pub num_targets: u32,
    pub target_index: u32,
    pub end_delay: f32,
    pub elapsed: f32,
    pub countdown: f32,
    pub time_limit: f32,
    pub just_activated: bool,
    pub warning_given: bool,
    /// The countdown effect (`+0xAA0`) is running: started with the
    /// countdown, released when it runs out or by `EndSituation`.
    pub countdown_effect: bool,
    /// The process-wide max CSI (here a field, [choice]).
    pub max_csi: u8,
    /// Held.
    pub window: Option<u64>,
    pub cheat_registered: bool,
    pub indicator_view_held: bool,
    pub lot_manager_held: bool,
    pub mysim_held: bool,
    pub lua_held: bool,
}

impl Default for CitySituationState {
    fn default() -> Self {
        CitySituationState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            indicators_visible: true,
            loaded_from_save: false,
            active_situation: None,
            automata_managers: Vec::new(),
            track_list: Vec::new(),
            tracked: BTreeSet::new(),
            success_counts: BTreeMap::new(),
            status: status::IDLE,
            active_automaton: None,
            current_target: None,
            num_targets: 0,
            target_index: 0,
            end_delay: 0.0,
            elapsed: 0.0,
            countdown: 0.0,
            time_limit: 0.0,
            just_activated: false,
            warning_given: false,
            countdown_effect: false,
            max_csi: 0,
            window: None,
            cheat_registered: false,
            indicator_view_held: false,
            lot_manager_held: false,
            mysim_held: false,
            lua_held: false,
        }
    }
}

/// `t × 0.001f` in `float` (§4.4, CS4: the x87 runs at 24 bits, so each
/// operation rounds to `f32`).
fn seconds(t_ms: f32) -> f32 {
    t_ms * MS_TO_SECONDS
}

/// `cSC4CitySituationManager`.
pub struct CitySituationManager {
    host: Rc<dyn CitySituationHost>,
    token: Cell<u64>,
    rng: RefCell<RzRandomSuper>,
    s: RefCell<CitySituationState>,
}

impl CitySituationManager {
    /// A constructed (not initialised) manager, reference count 0. The
    /// member generator is seeded with `derive_seed(session_seed,
    /// 0x0BB14381)` (§5 *[choice]*).
    pub fn new(host: Rc<dyn CitySituationHost>, token: u64, session_seed: u32) -> Rc<CitySituationManager> {
        let key = crate::seeding::keys::CITY_SITUATION;
        let rng = RzRandomSuper::new(
            crate::seeding::seed(session_seed, key),
            &crate::seeding::clocks(session_seed, key),
        );
        Rc::new(CitySituationManager {
            host,
            token: Cell::new(token),
            rng: RefCell::new(rng),
            s: RefCell::new(CitySituationState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn snapshot(&self) -> CitySituationState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut CitySituationState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- the generator (§5) ----

    /// `RandomUint32Uniform(n)` on the member generator (for the host's
    /// mission code).
    pub fn random_uniform_u32(&self, n: u32) -> u32 {
        self.rng.borrow_mut().uniform_u32(n)
    }

    /// `RandomSint32RangeUniform(lo, hi)` on the member generator.
    pub fn random_range_i32(&self, lo: i32, hi: i32) -> i32 {
        self.rng.borrow_mut().range_i32(lo, hi)
    }

    /// The next raw word of the generator, without advancing it (tests:
    /// "draw nothing").
    pub fn peek_random(&self) -> u32 {
        let mut copy = self.rng.borrow().clone();
        copy.next_u32()
    }

    // ---- identity (§1) ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// `Release` (§1): above 1 it decrements; at 1 or less it runs
    /// `Shutdown`, destroys the object (here: marks it) and returns 0.
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

    /// `Init` (§4.1): steps 1–9 when not initialised. Always true.
    pub fn init(&self) -> bool {
        if self.s.borrow().initialized {
            return true;
        }
        let h = &*self.host;
        // 1. The cheat.
        let cheat = h.debug_enabled() && h.register_cheat();
        // 2. Subscriptions, indicator view, animation tick.
        for m in msg::SUBSCRIPTIONS {
            h.subscribe(m);
        }
        let view = h.acquire_indicator_view();
        h.register_animation_tick();
        // 3. The four automata managers.
        let managers = h.acquire_automata_managers();
        // 4. Lot manager, MySim, Lua.
        let lot = h.acquire_lot_manager();
        let mysim = h.acquire_mysim();
        let lua = h.acquire_lua();
        // 5. The Lua table.
        h.create_lua_table();
        {
            let mut s = self.s.borrow_mut();
            s.cheat_registered = cheat;
            s.indicator_view_held = view;
            s.automata_managers = managers;
            s.lot_manager_held = lot;
            s.mysim_held = mysim;
            s.lua_held = lua;
        }
        // 6. HandleTuningParametersChanged.
        self.handle_tuning_parameters_changed();
        // 7. ShowAllIndicators.
        let visible = self.s.borrow().indicators_visible;
        if view {
            h.show_all_indicators(INDICATOR_KIND, visible);
        }
        // 8. Rewards from the profile (*[quirk]* overwrites with 1).
        let rewards = h.preferences_rewards();
        {
            let mut s = self.s.borrow_mut();
            for g in rewards.into_iter().take(REWARD_SLOTS) {
                if g == 0 {
                    break;
                }
                s.success_counts.insert(g, 1);
            }
        }
        // 9. Release the held window; initialised.
        let window = self.s.borrow_mut().window.take();
        if let Some(w) = window {
            h.release(w);
        }
        self.s.borrow_mut().initialized = true;
        true
    }

    /// `Shutdown` (§4.1). Always true; `successCounts` and
    /// `indicatorsVisible` are kept.
    pub fn shutdown(&self) -> bool {
        if !self.s.borrow().initialized {
            return true;
        }
        let h = &*self.host;
        let (window, mysim, lua, lot) = {
            let mut s = self.s.borrow_mut();
            (
                s.window.take(),
                std::mem::take(&mut s.mysim_held),
                std::mem::take(&mut s.lua_held),
                std::mem::take(&mut s.lot_manager_held),
            )
        };
        if let Some(w) = window {
            h.release(w);
        }
        if mysim {
            h.release_mysim();
        }
        if lua {
            h.release_lua();
        }
        if lot {
            h.release_lot_manager();
        }
        h.release_mission_resources();
        let managers = std::mem::take(&mut self.s.borrow_mut().automata_managers);
        // The countdown must not unfreeze managers already released.
        if self.s.borrow().countdown > 0.0 {
            if std::mem::take(&mut self.s.borrow_mut().countdown_effect) {
                h.end_countdown_effect();
            }
            for &m in &managers {
                h.freeze_automata(m, false);
            }
            self.s.borrow_mut().countdown = 0.0;
        }
        for m in managers {
            h.release(m);
        }
        h.unregister_animation_tick();
        if std::mem::take(&mut self.s.borrow_mut().indicator_view_held) {
            h.release_indicator_view();
        }
        for m in msg::SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        self.deactivate_situation();
        {
            let mut s = self.s.borrow_mut();
            s.track_list.clear();
            s.tracked.clear();
        }
        h.remove_lua_table();
        if std::mem::take(&mut self.s.borrow_mut().cheat_registered) {
            h.unregister_cheat();
        }
        self.s.borrow_mut().initialized = false;
        true
    }

    // ---- tracking (§4.2) ----

    /// The track-list index of `advice`.
    fn entry_of(&self, advice: u64) -> Option<usize> {
        self.s.borrow().track_list.iter().position(|e| e.advice == advice)
    }

    /// Looks for a vehicle for the entry of `advice` (when it has none and
    /// the tracked count is below max CSI).
    fn find_vehicle(&self, advice: u64) {
        let h = &*self.host;
        let Some(i) = self.entry_of(advice) else {
            return;
        };
        if self.s.borrow().track_list[i].automaton.is_some() {
            return;
        }
        for a in h.candidate_vehicles(advice) {
            {
                let s = self.s.borrow();
                if s.tracked.len() >= usize::from(s.max_csi) {
                    // CS7: the take-over scans the entries after this one for
                    // a strictly lower priority; the list is sorted ascending,
                    // so none qualifies (dead in shipped play): not done.
                    return;
                }
                if s.tracked.contains(&a) {
                    continue;
                }
            }
            if !can_player_drive(h.automaton_flags(a), h.automaton_type(a)) {
                continue;
            }
            let handle = h.track_indicator(a);
            h.add_listener(a);
            let mut s = self.s.borrow_mut();
            s.tracked.insert(a);
            if let Some(e) = s.track_list.iter_mut().find(|e| e.advice == advice) {
                e.automaton = Some(a);
                e.handle = handle;
            }
            return;
        }
    }

    /// Gives free indicator slots to the entries that have none.
    fn fill_free_slots(&self) {
        let advices: Vec<u64> = self
            .s
            .borrow()
            .track_list
            .iter()
            .filter(|e| e.automaton.is_none())
            .map(|e| e.advice)
            .collect();
        for a in advices {
            self.find_vehicle(a);
        }
    }

    /// `HandleInsertAutomaton` (`0x005291C0`, §4.2): the per-automaton side
    /// of the tracking, for an inserted occupant (`0x99EF1142`) and slot 25.
    /// [question CS13] A drivable, untracked automaton goes to the first
    /// entry (in list order) without a vehicle whose advice accepts it
    /// (advice slot 53), while fewer than max CSI entries hold a vehicle.
    pub fn handle_insert_automaton(&self, a: u64) {
        let h = &*self.host;
        {
            let s = self.s.borrow();
            let holders = s.track_list.iter().filter(|e| e.automaton.is_some()).count();
            if s.tracked.contains(&a) || holders >= usize::from(s.max_csi) {
                return;
            }
        }
        if !can_player_drive(h.automaton_flags(a), h.automaton_type(a)) {
            return;
        }
        let free: Vec<u64> = self
            .s
            .borrow()
            .track_list
            .iter()
            .filter(|e| e.automaton.is_none())
            .map(|e| e.advice)
            .collect();
        let Some(advice) = free.into_iter().find(|&adv| h.advice_accepts_automaton(adv, a)) else {
            return;
        };
        let handle = h.track_indicator(a);
        h.add_listener(a);
        let mut s = self.s.borrow_mut();
        s.tracked.insert(a);
        if let Some(e) = s.track_list.iter_mut().find(|e| e.advice == advice) {
            e.automaton = Some(a);
            e.handle = handle;
        }
    }

    /// Untracks the vehicle of entry `i` (the entry stays).
    fn untrack_entry(&self, i: usize) {
        let (a, handle) = {
            let s = self.s.borrow();
            let e = s.track_list[i];
            (e.automaton, e.handle)
        };
        if let Some(a) = a {
            self.host.untrack_indicator(handle);
            self.host.remove_listener(a);
            let mut s = self.s.borrow_mut();
            s.tracked.remove(&a);
            s.track_list[i].automaton = None;
            s.track_list[i].handle = 0;
        }
    }

    /// **Situation triggered** (§4.2): a new entry is appended and the list
    /// re-sorted ascending by advice priority (stable, CS7).
    pub fn situation_triggered(&self, advice: u64) {
        if self.entry_of(advice).is_none() {
            let mut list = std::mem::take(&mut self.s.borrow_mut().track_list);
            list.push(TrackEntry {
                advice,
                automaton: None,
                handle: 0,
            });
            // [question CS15] The priority compares unsigned.
            let keys: Vec<u32> = list.iter().map(|e| self.host.advice_priority(e.advice)).collect();
            let mut order: Vec<usize> = (0..list.len()).collect();
            order.sort_by_key(|&i| keys[i]);
            self.s.borrow_mut().track_list = order.into_iter().map(|i| list[i]).collect();
        }
        self.find_vehicle(advice);
    }

    /// **Situation untriggered** (§4.2).
    pub fn situation_untriggered(&self, advice: u64) {
        if let Some(i) = self.entry_of(advice) {
            self.untrack_entry(i);
            self.s.borrow_mut().track_list.remove(i);
        }
        self.fill_free_slots();
    }

    /// Drops an automaton from tracking; its situation looks for another
    /// vehicle (§3).
    fn drop_automaton(&self, a: u64) {
        let i = self.s.borrow().track_list.iter().position(|e| e.automaton == Some(a));
        if let Some(i) = i {
            let advice = self.s.borrow().track_list[i].advice;
            self.untrack_entry(i);
            self.find_vehicle(advice);
            self.fill_free_slots();
        }
    }

    // ---- starting a mission (§4.3) ----

    /// Slot 36 `InitiateSituation(advice, automaton, showBriefing)` (§4.3,
    /// CS5). False when nothing started (the advice already active, or
    /// null) or the mission could not start (status 5). *[quirk]* Without a
    /// briefing a started mission also gets status 5 and the post, but the
    /// call answers true.
    pub fn initiate_situation(&self, advice: Option<u64>, vehicle: Option<u64>, show_briefing: bool) -> bool {
        // 1.
        if self.s.borrow().active_situation == advice {
            return false;
        }
        // 2.
        self.deactivate_situation();
        // 3.
        let Some(advice) = advice else {
            return false;
        };
        let h = &*self.host;
        h.add_ref(advice);
        // 4.
        {
            let mut s = self.s.borrow_mut();
            s.active_situation = Some(advice);
            s.status = status::IDLE;
            s.end_delay = 0.0;
            s.warning_given = false;
        }
        let info = h.advice_info(advice);
        if info.flags & SITUATION_FLAG_MYSIM_WINDOW != 0 {
            if let Some(w) = h.acquire_mysim_window() {
                let old = self.s.borrow_mut().window.replace(w);
                if let Some(o) = old {
                    h.release(o);
                }
            }
        }
        self.s.borrow_mut().active_automaton = vehicle;
        h.bind_mission_script(advice, vehicle);
        // 5.
        let started = vehicle.is_some()
            && match info.mission_type {
                MISSION_TYPE_NO_TARGET => true,
                MISSION_TYPE_COVERAGE => h.build_coverage_cells(advice),
                _ => {
                    let n = h.target_count(advice);
                    {
                        let mut s = self.s.borrow_mut();
                        s.num_targets = n;
                        s.target_index = 0;
                    }
                    let t = h.first_target(advice);
                    self.s.borrow_mut().current_target = t;
                    t.is_some()
                }
            };
        // 6.
        if started && show_briefing {
            h.post_show_popup(Some(advice), vehicle);
            return true;
        }
        // 7. and 8.
        self.s.borrow_mut().status = status::COULD_NOT_START;
        h.post_show_popup(Some(advice), None);
        started
    }

    /// Starts `advice` with the automaton `a`: with a briefing through slot
    /// 22, else slot 36 without one and, when that answers true, slot 37
    /// (§8 slot 21).
    fn start_with(&self, advice: u64, a: u64, show_briefing: bool) -> bool {
        if show_briefing {
            self.initiate_situation_for_occupant(a, Some(advice))
        } else if self.initiate_situation(Some(advice), Some(a), false) {
            self.activate_situation();
            true
        } else {
            false
        }
    }

    /// The automata of the groups (`0x00525640`), in set (token) order.
    fn collect_group_automata(&self, groups: &[u32]) -> Vec<u64> {
        let mut set = BTreeSet::new();
        for &g in groups {
            set.extend(self.host.group_automata(g));
        }
        set.into_iter().collect()
    }

    /// Slot 21 `InitiateSituationByGuid(guid, showBriefing, group)` (§8):
    /// advisor 29's city-situation advice with that GUID; the drivable
    /// automata of `group` (non-zero) or of the advice's groups, in set
    /// order, until one starts; else a created automaton (removed when it
    /// does not start, released either way); with none, slot 36 without a
    /// vehicle (the "could not start" popup). True when a start succeeded.
    pub fn initiate_situation_by_guid(&self, guid: u32, show_briefing: bool, group: u32) -> bool {
        let h = &*self.host;
        let Some(advice) = h.find_advice(guid).filter(|&a| h.is_situation_advice(a)) else {
            return false;
        };
        let groups = if group != 0 {
            vec![group]
        } else {
            h.advice_groups(advice)
        };
        for a in self.collect_group_automata(&groups) {
            if can_player_drive(h.automaton_flags(a), h.automaton_type(a)) && self.start_with(advice, a, show_briefing)
            {
                return true;
            }
        }
        match h.create_automaton_for_player_drive(Some(advice), group) {
            Some(n) => {
                let ok = self.start_with(advice, n, show_briefing);
                if !ok {
                    h.remove_automaton(n);
                }
                h.release(n);
                ok
            }
            None => {
                self.initiate_situation(Some(advice), None, true);
                false
            }
        }
    }

    /// Slot 22 `InitiateSituationForOccupant(occupant, advice)` (§8, CS2):
    /// false unless the occupant is an automaton; with an advice,
    /// `InitiateSituation(advice, automaton, true)`; without one, the same
    /// call with the first track-list entry whose vehicle is the automaton or
    /// whose advice accepts it (advice slot 53); false when none matches.
    pub fn initiate_situation_for_occupant(&self, occupant: u64, advice: Option<u64>) -> bool {
        let h = &*self.host;
        if !h.is_automaton(occupant) {
            return false;
        }
        let advice = match advice {
            Some(a) => Some(a),
            None => {
                let entries: Vec<(u64, Option<u64>)> = self
                    .s
                    .borrow()
                    .track_list
                    .iter()
                    .map(|e| (e.advice, e.automaton))
                    .collect();
                entries
                    .into_iter()
                    .find(|&(a, v)| v == Some(occupant) || h.advice_accepts_automaton(a, occupant))
                    .map(|(a, _)| a)
            }
        };
        match advice {
            Some(a) => self.initiate_situation(Some(a), Some(occupant), true),
            None => false,
        }
    }

    /// Slot 23 `InitiateSituationByAutomataGroup(group, showBriefing,
    /// search)` (§8, CS11): the first entry whose advice accepts the group
    /// starts through slot 21; else (with `search`) a joyride in the first
    /// drivable automaton of the group, or a created one. The automaton is
    /// held for the call and released after `StartJoyride` [choice].
    pub fn initiate_situation_by_automata_group(&self, group: u32, show_briefing: bool, search: bool) -> bool {
        let h = &*self.host;
        let advices: Vec<u64> = self.s.borrow().track_list.iter().map(|e| e.advice).collect();
        if let Some(advice) = advices.into_iter().find(|&a| h.advice_accepts_group(a, group)) {
            let guid = h.advice_info(advice).guid;
            return self.initiate_situation_by_guid(guid, show_briefing, group);
        }
        if !search {
            return false;
        }
        let found = self
            .collect_group_automata(&[group])
            .into_iter()
            .find(|&a| can_player_drive(h.automaton_flags(a), h.automaton_type(a)));
        let a = match found {
            Some(a) => {
                h.add_ref(a);
                a
            }
            None => match h.create_automaton_for_player_drive(None, group) {
                Some(a) => a,
                None => return false,
            },
        };
        self.start_joyride(a, true, None);
        h.release(a);
        true
    }

    /// Slot 24 `StartJoyride(automaton, b, target)` (§8, CS11): stores `b`
    /// in `justActivated`, sends `0xABBBCA64` (data1 = 1) with no advice and
    /// enters the drive view. The status and the active advice stay.
    pub fn start_joyride(&self, automaton: u64, b: bool, target: Option<u64>) {
        self.s.borrow_mut().just_activated = b;
        self.host.send_activation(None, true);
        self.host.enter_player_drive_view(automaton, target);
    }

    /// Slot 25 `TreatAutomatonAsGroup(automaton, group)` (§8, CS11): null
    /// does nothing; else the group is set and the automaton goes through
    /// `HandleInsertAutomaton`.
    pub fn treat_automaton_as_group(&self, automaton: Option<u64>, group: u32) {
        let Some(a) = automaton else { return };
        self.host.set_automaton_group(a, group);
        self.handle_insert_automaton(a);
    }

    /// Slot 37 `ActivateSituation` (§4.3).
    pub fn activate_situation(&self) {
        let (advice, vehicle) = {
            let s = self.s.borrow();
            (s.active_situation, s.active_automaton)
        };
        let Some(advice) = advice else {
            return;
        };
        let h = &*self.host;
        self.s.borrow_mut().status = status::RUNNING;
        h.run_script(advice, Script::Start);
        let limit = h.time_limit(advice);
        {
            let mut s = self.s.borrow_mut();
            s.time_limit = limit;
            s.elapsed = 0.0;
            s.warning_given = false;
        }
        h.send_activation(Some(advice), true);
        h.player_drive_mode(vehicle, true);
        self.s.borrow_mut().just_activated = true;
    }

    /// Slot 38 `DeactivateSituation` (§4.3.1, CS6), only with a mission
    /// held. The time limit, the elapsed time, the countdown, `endDelay` and
    /// the indicators are left alone.
    pub fn deactivate_situation(&self) {
        let h = &*self.host;
        let Some(advice) = self.s.borrow().active_situation else {
            return;
        };
        // 1. and 2.
        h.clear_mission_state(advice);
        {
            let mut s = self.s.borrow_mut();
            s.active_automaton = None;
            s.current_target = None;
            s.status = status::IDLE;
            s.num_targets = 0;
        }
        // 3.
        h.send_activation(Some(advice), false);
        self.s.borrow_mut().just_activated = false;
        // 4.
        h.release(advice);
        let window = {
            let mut s = self.s.borrow_mut();
            s.active_situation = None;
            s.window.take()
        };
        if let Some(w) = window {
            h.release(w);
        }
    }

    /// Slot 39 `EndSituation` (§4.3.1, CS6).
    pub fn end_situation(&self) {
        let h = &*self.host;
        // 1.
        h.release_situation_effect();
        self.s.borrow_mut().time_limit = 0.0;
        // 2. The countdown value is kept: it still runs out on later ticks.
        if std::mem::take(&mut self.s.borrow_mut().countdown_effect) {
            h.end_countdown_effect();
        }
        // 3.
        let (advice, vehicle) = {
            let s = self.s.borrow();
            (s.active_situation, s.active_automaton)
        };
        if let Some(advice) = advice {
            if let Some(v) = vehicle {
                // The vehicle is untracked through the entry that holds it.
                let holder = self.s.borrow().track_list.iter().position(|e| e.automaton == Some(v));
                if let Some(j) = holder {
                    self.untrack_entry(j);
                }
            }
            if let Some(i) = self.entry_of(advice) {
                let mut s = self.s.borrow_mut();
                s.track_list[i].automaton = None;
                s.track_list[i].handle = 0;
            }
            self.deactivate_situation();
        }
        // 4.
        let (view, visible) = {
            let s = self.s.borrow();
            (s.indicator_view_held, s.indicators_visible)
        };
        if view {
            h.show_all_indicators(INDICATOR_KIND, visible);
        }
    }

    /// `ShowSituationPopup` (§4.3): the window by status (a null advice
    /// counts as 5), run modally, and its button.
    pub fn show_situation_popup(&self, advice: Option<u64>, vehicle: Option<u64>) {
        let st = self.s.borrow().status;
        let popup = match (advice, st) {
            (None, _) | (_, status::SUCCESS | status::COULD_NOT_START) => Popup::Success,
            (_, status::FAILURE) => Popup::Failure,
            _ => Popup::Briefing,
        };
        let b = self.host.show_popup(popup, advice, vehicle);
        match b {
            button::START => self.activate_situation(),
            button::JOYRIDE => self.host.start_joyride(vehicle),
            button::CLOSE => {
                if st == status::SUCCESS || st == status::FAILURE {
                    if let Some(a) = advice {
                        self.host.post_popup_closed(a);
                    }
                } else {
                    self.deactivate_situation();
                }
            }
            _ => {}
        }
    }

    // ---- the animation tick (§4.4) ----

    /// `HandleStatusChanged(s)` (§4.4).
    pub fn handle_status_changed(&self, new_status: u32) {
        let (old, advice, vehicle) = {
            let s = self.s.borrow();
            (s.status, s.active_situation, s.active_automaton)
        };
        if new_status == old {
            return;
        }
        self.s.borrow_mut().status = new_status;
        let Some(advice) = advice else {
            return;
        };
        if new_status == status::SUCCESS || new_status == status::FAILURE {
            let long = vehicle.is_some_and(|v| self.host.automaton_flags(v) & AUTO_FLAG_LONG_END != 0);
            let d = self.host.advice_info(advice).end_delay;
            self.s.borrow_mut().end_delay = if long && d > DEFAULT_END_DELAY {
                d
            } else {
                DEFAULT_END_DELAY
            };
        }
    }

    /// `CheckForSuccess(t)` (`0x0052C8F0`, §4.4, CS4): 4 without a mission
    /// or an active automaton; the host's checks (steps 2–4); at status 3
    /// the mission clock advances in `float` and a time limit of at least
    /// 1 s that the clock reached gives status 2.
    pub fn check_for_success(&self, t_ms: f32) -> u32 {
        let (advice, vehicle, st) = {
            let s = self.s.borrow();
            (s.active_situation, s.active_automaton, s.status)
        };
        // 1.
        let (Some(advice), Some(_)) = (advice, vehicle) else {
            return status::IDLE;
        };
        // 2.–4.
        let mut st = self.host.check_for_success(advice, t_ms, st);
        // 5.
        if st == status::RUNNING {
            let mut s = self.s.borrow_mut();
            s.elapsed += seconds(t_ms);
            if s.time_limit >= MIN_TIME_LIMIT && s.time_limit <= s.elapsed {
                st = status::FAILURE;
            }
        }
        // 6.
        st
    }

    /// The animation tick (§4.4, CS4), `t_ms` the elapsed milliseconds.
    pub fn tick(&self, t_ms: f32) {
        let h = &*self.host;
        let (just, countdown, managers) = {
            let s = self.s.borrow();
            (s.just_activated, s.countdown, s.automata_managers.clone())
        };
        if just {
            // 1.
            for &m in &managers {
                h.freeze_automata(m, true);
            }
            {
                let mut s = self.s.borrow_mut();
                s.just_activated = false;
                s.countdown = COUNTDOWN_MS;
                s.countdown_effect = true;
            }
            h.start_countdown_effect();
            return;
        }
        if countdown > 0.0 {
            // 2.
            let c = countdown - t_ms;
            self.s.borrow_mut().countdown = c;
            if c > 0.0 {
                return;
            }
            if std::mem::take(&mut self.s.borrow_mut().countdown_effect) {
                h.end_countdown_effect();
            }
            for &m in &managers {
                h.freeze_automata(m, false);
            }
            // On to step 4 in the same tick.
        } else {
            // 3.
            let (advice, end_delay, st) = {
                let s = self.s.borrow();
                (s.active_situation, s.end_delay, s.status)
            };
            if let Some(advice) = advice {
                if end_delay > 0.0 {
                    let d = end_delay - seconds(t_ms);
                    self.s.borrow_mut().end_delay = d;
                    if d <= 0.0 {
                        match st {
                            status::SUCCESS => self.success(advice),
                            status::FAILURE => self.failure(advice),
                            _ => {}
                        }
                        h.post_show_popup(Some(advice), None);
                    }
                    return;
                }
            }
        }
        // 4. With a mission held, whatever the status.
        let Some(advice) = self.s.borrow().active_situation else {
            return;
        };
        // 4.1 The warning, against the elapsed time before this tick's step.
        let warn = {
            let mut s = self.s.borrow_mut();
            let warn =
                !s.warning_given && s.time_limit >= WARNING_SECONDS && s.time_limit - WARNING_SECONDS <= s.elapsed;
            if warn {
                s.warning_given = true;
            }
            warn.then_some(s.active_automaton)
        };
        // [choice] The original has no null test; no automaton, no sound.
        if let Some(Some(v)) = warn {
            h.play_sound(v, WARNING_SOUND);
        }
        // 4.2
        let st = self.check_for_success(t_ms);
        self.handle_status_changed(st);
        // 4.3
        h.run_pursuit(advice, t_ms);
    }

    // ---- the outcome (§4.5) ----

    /// The aura (radius > 0) at (x, z).
    fn add_aura(&self, aura: (f32, f32), x: f32, z: f32) {
        if aura.0 > 0.0 {
            self.host.add_transient_effect(x, z, aura.0, aura.1);
        }
    }

    /// **Success** (§4.5), only for the active advice.
    pub fn success(&self, advice: u64) {
        if self.s.borrow().active_situation != Some(advice) {
            return;
        }
        let h = &*self.host;
        let info = h.advice_info(advice);
        // 1.
        {
            let mut s = self.s.borrow_mut();
            let c = s.success_counts.entry(info.guid).or_insert(0);
            if *c < MAX_SUCCESS_COUNT {
                *c += 1;
            }
        }
        // 2.
        if info.flags & SITUATION_FLAG_REWARD != 0 {
            let rewards = h.preferences_rewards();
            let n = rewards.len().min(REWARD_SLOTS);
            if !rewards[..n].contains(&info.guid) {
                if let Some(i) = rewards[..n].iter().position(|&g| g == 0) {
                    h.set_preferences_reward(i, info.guid);
                    h.save_preferences();
                }
            }
        }
        // 3. CS9: the active automaton's position, else (0, 0, 0) [quirk].
        let vehicle = self.s.borrow().active_automaton;
        let (x, z) = vehicle.map_or((0.0, 0.0), |v| h.position(v));
        // 4.
        h.outcome_effect(advice, true, x, z);
        // 5.
        self.add_aura(info.success_aura, x, z);
        // 6.
        if info.success_money != 0 {
            h.deposit_funds(i64::from(info.success_money));
        }
        // 7.
        h.run_script(advice, Script::Success);
        h.retire_advice(advice);
        self.situation_untriggered(advice);
    }

    /// **Failure** (§4.5), only for the active advice. CS9: the active
    /// automaton's position, else the target's; with neither, no effect, no
    /// aura and no money (CS8: money only when not 0).
    pub fn failure(&self, advice: u64) {
        if self.s.borrow().active_situation != Some(advice) {
            return;
        }
        let h = &*self.host;
        let info = h.advice_info(advice);
        let at = {
            let s = self.s.borrow();
            s.active_automaton.or(s.current_target)
        };
        if let Some(o) = at {
            let (x, z) = h.position(o);
            h.outcome_effect(advice, false, x, z);
            self.add_aura(info.failure_aura, x, z);
            if info.failure_money != 0 {
                h.deposit_funds(i64::from(info.failure_money));
            }
        }
        h.run_script(advice, Script::Failure);
    }

    // ---- entry points (§4.6, §8) ----

    /// Slot 5 / Lua `get_success_count`.
    pub fn success_count(&self, guid: u32) -> u16 {
        self.s.borrow().success_counts.get(&guid).copied().unwrap_or(0)
    }

    /// Lua `activate_situation(guid [, showBriefing])`: posts `0xAC1045EF`.
    pub fn lua_activate_situation(&self, guid: u32, show_briefing: bool) -> bool {
        self.host.post_initiate(guid, show_briefing);
        true
    }

    /// Slot 6.
    pub fn active_situation(&self) -> Option<u64> {
        self.s.borrow().active_situation
    }

    /// Slot 7.
    pub fn current_status(&self) -> u32 {
        self.s.borrow().status
    }

    /// Slot 8.
    pub fn active_auto(&self) -> Option<u64> {
        self.s.borrow().active_automaton
    }

    /// Slot 9.
    pub fn current_target(&self) -> Option<u64> {
        self.s.borrow().current_target
    }

    /// Slot 10.
    pub fn current_target_index(&self) -> u32 {
        self.s.borrow().target_index
    }

    /// Slot 11.
    pub fn num_targets(&self) -> u32 {
        self.s.borrow().num_targets
    }

    /// Slot 12: slot 11 − slot 10.
    pub fn num_targets_remaining(&self) -> u32 {
        let s = self.s.borrow();
        s.num_targets.wrapping_sub(s.target_index)
    }

    /// Slot 13.
    pub fn time_limit_seconds(&self) -> f32 {
        self.s.borrow().time_limit
    }

    /// Slot 14.
    pub fn situation_time(&self) -> f32 {
        self.s.borrow().elapsed
    }

    /// Slot 15: slot 13 − slot 14.
    pub fn time_remaining_seconds(&self) -> f32 {
        let s = self.s.borrow();
        s.time_limit - s.elapsed
    }

    /// Slot 16.
    pub fn current_target_name(&self) -> String {
        let (a, t) = {
            let s = self.s.borrow();
            (s.active_situation, s.current_target)
        };
        self.host.target_name(a, t)
    }

    /// Slot 17: the active advice's success aura (false without a mission).
    pub fn situation_aura(&self) -> Option<(f32, f32)> {
        let a = self.s.borrow().active_situation?;
        Some(self.host.advice_info(a).success_aura)
    }

    /// Slot 18: the active advice's success money.
    pub fn situation_money(&self) -> Option<i64> {
        let a = self.s.borrow().active_situation?;
        Some(i64::from(self.host.advice_info(a).success_money))
    }

    /// Slots 19, 20: the automaton carries an indicator.
    pub fn is_situation_available(&self, automaton: u64) -> bool {
        self.s.borrow().tracked.contains(&automaton)
    }

    /// Slot 26 (process-wide in the original).
    pub fn set_max_csi(&self, n: u8) {
        self.s.borrow_mut().max_csi = n;
    }

    /// Slot 27.
    pub fn max_csi(&self) -> u8 {
        self.s.borrow().max_csi
    }

    /// Slot 28.
    pub fn tracked_automata_count(&self) -> u32 {
        self.s.borrow().tracked.len() as u32
    }

    /// Slot 29 [choice: also re-shows the indicators].
    pub fn set_csi_visible(&self, on: bool) {
        let view = {
            let mut s = self.s.borrow_mut();
            s.indicators_visible = on;
            s.initialized && s.indicator_view_held
        };
        if view {
            self.host.show_all_indicators(INDICATOR_KIND, on);
        }
    }

    /// Slot 30.
    pub fn csi_visible(&self) -> bool {
        self.s.borrow().indicators_visible
    }

    /// Slot 31: `SetMaxCSI(GetMaxCSI())`. True.
    pub fn handle_tuning_parameters_changed(&self) -> bool {
        let n = self.max_csi();
        self.set_max_csi(n);
        true
    }

    /// Slot 32.
    pub fn countdown_in_progress(&self) -> bool {
        self.s.borrow().countdown > 0.0
    }

    /// Slot 33 (`ResetTutorials`, §4.6): clear the counts, zero the 64
    /// reward entries and save the preferences.
    pub fn reset_tutorials(&self) {
        self.s.borrow_mut().success_counts.clear();
        for i in 0..REWARD_SLOTS {
            self.host.set_preferences_reward(i, 0);
        }
        self.host.save_preferences();
    }

    /// Slot 35 `InitiatePlayerDriveMode(on)` (§4.3): on also sets
    /// `justActivated`.
    pub fn initiate_player_drive_mode(&self, on: bool) {
        if on {
            self.s.borrow_mut().just_activated = true;
        }
        let v = self.s.borrow().active_automaton;
        self.host.player_drive_mode(v, on);
    }

    // ---- messages (§3) ----

    /// `DoMessage` (+4). True for every message.
    pub fn do_message(&self, m: &SituationMessage) -> bool {
        let h = &*self.host;
        match m.msg_type {
            msg::OCCUPANT_INSERTED => {
                if let Some(o) = m.object {
                    if !h.occupant_has_property(o, PROP_PREVIEW_OCCUPANT) && h.is_automaton(o) {
                        self.handle_insert_automaton(o);
                    }
                }
            }
            msg::OCCUPANT_REMOVED => {
                if let Some(o) = m.object {
                    self.occupant_removed(o);
                }
            }
            msg::ADVICE_ADDED => {
                if let Some(a) = m.object.filter(|&a| h.is_situation_advice(a)) {
                    self.situation_triggered(a);
                }
            }
            msg::ADVICE_REMOVED => {
                if let Some(a) = m.object.filter(|&a| h.is_situation_advice(a)) {
                    self.situation_untriggered(a);
                }
            }
            msg::ADVICE_REMOVED_ALL => {}
            msg::POPUP_CLOSED => {
                let active = self.s.borrow().active_situation;
                if m.object.is_some() && m.object == active {
                    self.initiate_player_drive_mode(false);
                    self.end_situation();
                }
            }
            msg::SHOW_POPUP => {
                h.hide_mission_status_panel();
                h.end_mysim_dialog();
                self.show_situation_popup(m.object, m.void1);
            }
            msg::WINDOW => {
                let active = self.s.borrow().active_situation;
                if let (Some(a), Some(w)) = (active, m.object) {
                    if h.advice_info(a).flags & SITUATION_FLAG_WINDOW != 0 {
                        h.add_ref(w);
                        let old = self.s.borrow_mut().window.replace(w);
                        if let Some(o) = old {
                            h.release(o);
                        }
                    }
                }
            }
            msg::INITIATE_BY_GUID => {
                self.initiate_situation_by_guid(m.data1, m.data2 != 0, m.data3);
            }
            msg::POST_CITY_INIT => {
                if self.s.borrow().loaded_from_save {
                    for a in h.advisor_advices() {
                        if h.is_situation_advice(a) {
                            self.situation_triggered(a);
                        }
                    }
                }
            }
            msg::ANIMATION_TICK => self.tick(f32::from_bits(m.data3)),
            msg::AUTOMATON_NOTIFY => {
                if let Some(a) = m.object {
                    self.automaton_notify(a, m.data2, m.data3);
                }
            }
            msg::PLAYER_LEFT_VEHICLE => {
                let st = self.s.borrow().status;
                if st != status::SUCCESS && st != status::FAILURE {
                    self.end_situation();
                }
            }
            msg::CHEAT => {
                if m.data1 == CHEAT_ID && self.s.borrow().cheat_registered {
                    if let Some(g) = m.text.as_deref().and_then(parse_cheat) {
                        h.post_initiate(g, true);
                    }
                }
            }
            msg::POPUP_NO_ADVICE => {
                h.hide_mission_status_panel();
                h.end_mysim_dialog();
                self.show_situation_popup(None, None);
            }
            _ => {}
        }
        true
    }

    /// `0x99EF1143` (§3).
    fn occupant_removed(&self, o: u64) {
        let (active, st, vehicle, target) = {
            let s = self.s.borrow();
            (s.active_situation, s.status, s.active_automaton, s.current_target)
        };
        if active.is_some() && (vehicle == Some(o) || target == Some(o)) {
            if st == status::RUNNING {
                self.handle_status_changed(status::FAILURE);
            }
            let mut s = self.s.borrow_mut();
            if s.active_automaton == Some(o) {
                s.active_automaton = None;
            }
            if s.current_target == Some(o) {
                s.current_target = None;
            }
        }
        if self.s.borrow().tracked.contains(&o) {
            self.drop_automaton(o);
        }
    }

    /// `0x09D92FC3` (§4.7, CS10).
    fn automaton_notify(&self, a: u64, subtype: u32, data3: u32) {
        let (active, st, vehicle, target) = {
            let s = self.s.borrow();
            (s.active_situation, s.status, s.active_automaton, s.current_target)
        };
        match subtype {
            notify::MOVED => {
                if active.is_some() && (vehicle == Some(a) || target == Some(a)) {
                    // [question CS12] §4.7 names only the call: its result is
                    // not applied.
                    let _ = self.check_for_success(0.0);
                }
            }
            notify::REMOVED => {
                if active.is_some() && st == status::RUNNING && vehicle == Some(a) {
                    self.handle_status_changed(status::FAILURE);
                }
            }
            notify::DISABLED_B => self.handle_automaton_disabled(a),
            notify::DISABLED_C if data3 == notify::DISABLED_C_DATA3 => self.handle_automaton_disabled(a),
            _ => {}
        }
    }

    /// `HandleAutomatonDisabled` (`0x0052C020`, §4.7, CS10): the automaton
    /// is marked disabled (flag `0x10000000`); with a mission held, the
    /// active automaton fails it whatever the status, the current target
    /// moves on (types 1 and 3; success when none is left) or fails it; then
    /// a tracked automaton is dropped as for a removed occupant.
    fn handle_automaton_disabled(&self, a: u64) {
        let h = &*self.host;
        h.set_automaton_flag(a, AUTO_FLAG_DISABLED);
        let (active, st, vehicle, target) = {
            let s = self.s.borrow();
            (s.active_situation, s.status, s.active_automaton, s.current_target)
        };
        if let Some(advice) = active {
            let new_status = if vehicle == Some(a) {
                status::FAILURE
            } else if target == Some(a) {
                if MISSION_TYPES_NEXT_TARGET.contains(&h.advice_info(advice).mission_type) {
                    // [question CS14] The host chooses the next target; the
                    // index and the count are left alone.
                    match h.next_target(advice) {
                        Some(t) => {
                            self.s.borrow_mut().current_target = Some(t);
                            st
                        }
                        None => status::SUCCESS,
                    }
                } else {
                    status::FAILURE
                }
            } else {
                st
            };
            self.handle_status_changed(new_status);
        }
        if self.s.borrow().tracked.contains(&a) {
            self.drop_automaton(a);
        }
    }
}

impl SaveObject for CitySituationManager {
    fn class_id(&self) -> u32 {
        CLSID_CITY_SITUATION
    }

    /// §7: version 2, the counts (ascending GUID, [choice]), the visible
    /// flag. True unless the stream reports an error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let (counts, visible) = {
            let s = self.s.borrow();
            (s.success_counts.clone(), s.indicators_visible)
        };
        out.put_u16(RECORD_VERSION);
        out.put_u32(counts.len() as u32);
        for (g, c) in counts {
            out.put_u32(g);
            out.put_u16(c);
        }
        out.put_bool(visible);
        out.error() == 0
    }

    /// §7, literally: a version above 2 fails before anything changes;
    /// otherwise the map is cleared and filled entry by entry (the first of
    /// two equal GUIDs wins).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else {
            return false;
        };
        if version > RECORD_VERSION {
            return false;
        }
        {
            let mut s = self.s.borrow_mut();
            s.loaded_from_save = true;
            s.success_counts.clear();
        }
        let Some(n) = inp.get_u32() else {
            return false;
        };
        for _ in 0..n {
            let (Some(g), Some(c)) = (inp.get_u32(), inp.get_u16()) else {
                return false;
            };
            self.s.borrow_mut().success_counts.entry(g).or_insert(c);
        }
        if version >= 2 {
            let Some(v) = inp.get_u8() else {
                return false;
            };
            self.s.borrow_mut().indicators_visible = v != 0;
        }
        true
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
