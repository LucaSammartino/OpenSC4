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

use crate::rng::{derive_seed, FixedSeeds, RzRandomSuper};
use crate::stream::{InStream, OutStream, SaveObject};

use super::{
    button, can_player_drive, msg, notify, parse_cheat, status, CitySituationHost, Popup, Script, SituationMessage,
    AUTO_FLAG_LONG_END, CHEAT_ID, CLSID_CITY_SITUATION, COUNTDOWN_MS, DEFAULT_END_DELAY, IID_CITY_SITUATION,
    IID_MESSAGE_TARGET, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN, INDICATOR_KIND, MAX_SUCCESS_COUNT,
    MISSION_TYPE_COVERAGE, MISSION_TYPE_NO_TARGET, PROP_PREVIEW_OCCUPANT, RECORD_VERSION, REWARD_SLOTS,
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

/// `x − t × 0.001` in double, stored to float (§4.4).
fn minus_seconds(x: f32, t_ms: f32) -> f32 {
    (f64::from(x) - f64::from(t_ms) * 0.001) as f32
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
        let seed = derive_seed(session_seed, CLSID_CITY_SITUATION);
        let rng = RzRandomSuper::new(
            seed,
            &FixedSeeds {
                wall_clock: seed,
                process_clock: seed,
            },
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
            h.end_countdown_effect();
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
                    // [question CS7] no take-over.
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

    /// **Situation triggered** (§4.2).
    pub fn situation_triggered(&self, advice: u64) {
        if self.entry_of(advice).is_none() {
            self.s.borrow_mut().track_list.push(TrackEntry {
                advice,
                automaton: None,
                handle: 0,
            });
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

    /// Slot 36 `InitiateSituation(advice, automaton, showBriefing)`.
    pub fn initiate_situation(&self, advice: u64, vehicle: Option<u64>, show_briefing: bool) -> bool {
        if self.s.borrow().active_situation == Some(advice) {
            return false;
        }
        self.deactivate_situation();
        let h = &*self.host;
        h.add_ref(advice);
        {
            let mut s = self.s.borrow_mut();
            s.active_situation = Some(advice);
            s.status = status::IDLE;
            s.end_delay = 0.0;
            s.active_automaton = vehicle;
        }
        h.bind_mission_script(advice, vehicle);
        let info = h.advice_info(advice);
        if info.mission_type == MISSION_TYPE_COVERAGE {
            h.build_coverage_cells(advice);
        } else if info.mission_type != MISSION_TYPE_NO_TARGET {
            match h.first_target(advice) {
                Some(t) => {
                    let n = h.target_count(advice);
                    let mut s = self.s.borrow_mut();
                    s.current_target = Some(t);
                    s.num_targets = n;
                    s.target_index = 0;
                }
                None => {
                    self.s.borrow_mut().status = status::COULD_NOT_START;
                    h.post_show_popup(Some(advice), None);
                    // [question CS5]
                    return false;
                }
            }
        }
        if show_briefing {
            h.post_show_popup(Some(advice), vehicle);
        }
        true
    }

    /// Slot 21 `InitiateSituationByGuid(guid, showBriefing, data3)`.
    pub fn initiate_situation_by_guid(&self, guid: u32, show_briefing: bool, _data3: u32) -> bool {
        let Some(advice) = self.host.find_advice(guid) else {
            return false;
        };
        let vehicle = self
            .entry_of(advice)
            .and_then(|i| self.s.borrow().track_list[i].automaton);
        let ok = self.initiate_situation(advice, vehicle, show_briefing);
        if ok && !show_briefing {
            self.activate_situation();
        }
        ok
    }

    /// Slot 22 `InitiateSituationForOccupant(occupant, …)`: the situation
    /// whose entry tracks the occupant, with a briefing (CS2).
    pub fn initiate_situation_for_occupant(&self, occupant: u64) -> bool {
        let advice = self
            .s
            .borrow()
            .track_list
            .iter()
            .find(|e| e.automaton == Some(occupant))
            .map(|e| e.advice);
        match advice {
            Some(a) => self.initiate_situation(a, Some(occupant), true),
            None => false,
        }
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
        h.send_activation(advice, true);
        h.player_drive_mode(vehicle, true);
        self.s.borrow_mut().just_activated = true;
    }

    /// Slot 38 `DeactivateSituation` [question CS6].
    pub fn deactivate_situation(&self) {
        let h = &*self.host;
        let (advice, countdown, managers) = {
            let s = self.s.borrow();
            (s.active_situation, s.countdown, s.automata_managers.clone())
        };
        let Some(advice) = advice else {
            return;
        };
        if countdown > 0.0 {
            h.end_countdown_effect();
            for m in managers {
                h.freeze_automata(m, false);
            }
        }
        h.send_activation(advice, false);
        h.release(advice);
        let mut s = self.s.borrow_mut();
        s.active_situation = None;
        s.status = status::IDLE;
        s.active_automaton = None;
        s.current_target = None;
        s.num_targets = 0;
        s.target_index = 0;
        s.end_delay = 0.0;
        s.elapsed = 0.0;
        s.countdown = 0.0;
        s.time_limit = 0.0;
        s.just_activated = false;
        s.warning_given = false;
    }

    /// Slot 39 `EndSituation` [question CS6].
    pub fn end_situation(&self) {
        self.deactivate_situation();
    }

    /// `ShowSituationPopup` (§4.3): the window by status, run modally, and
    /// its button.
    pub fn show_situation_popup(&self, advice: Option<u64>, vehicle: Option<u64>) {
        let st = self.s.borrow().status;
        let popup = match st {
            status::SUCCESS | status::COULD_NOT_START => Popup::Success,
            status::FAILURE => Popup::Failure,
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

    /// The animation tick (§4.4), `t_ms` the elapsed milliseconds.
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
            }
            h.start_countdown_effect();
        } else if countdown > 0.0 {
            // 2.
            let c = countdown - t_ms;
            self.s.borrow_mut().countdown = c;
            if c <= 0.0 {
                h.end_countdown_effect();
                for &m in &managers {
                    h.freeze_automata(m, false);
                }
            }
        } else {
            // 3.
            let (advice, end_delay, st) = {
                let s = self.s.borrow();
                (s.active_situation, s.end_delay, s.status)
            };
            if let Some(advice) = advice {
                if end_delay > 0.0 {
                    let d = minus_seconds(end_delay, t_ms);
                    self.s.borrow_mut().end_delay = d;
                    if d <= 0.0 {
                        match st {
                            status::SUCCESS => self.success(advice),
                            status::FAILURE => self.failure(advice),
                            _ => {}
                        }
                        let vehicle = self.s.borrow().active_automaton;
                        h.post_show_popup(Some(advice), vehicle);
                    }
                }
            }
        }
        // 4. [question CS4]: only while running and not counting down.
        let (advice, st, countdown) = {
            let s = self.s.borrow();
            (s.active_situation, s.status, s.countdown)
        };
        let Some(advice) = advice else {
            return;
        };
        if countdown > 0.0 || st != status::RUNNING || self.s.borrow().just_activated {
            return;
        }
        let (warn, vehicle) = {
            let mut s = self.s.borrow_mut();
            s.elapsed = (f64::from(s.elapsed) + f64::from(t_ms) * 0.001) as f32;
            let warn = s.time_limit > 0.0 && !s.warning_given && s.time_limit - s.elapsed <= WARNING_SECONDS;
            if warn {
                s.warning_given = true;
            }
            (warn, s.active_automaton)
        };
        if warn {
            if let Some(v) = vehicle {
                h.play_sound(v, WARNING_SOUND);
            }
        }
        let new_status = h.check_for_success(advice, t_ms);
        self.handle_status_changed(new_status);
        let (st, limit, elapsed) = {
            let s = self.s.borrow();
            (s.status, s.time_limit, s.elapsed)
        };
        if st == status::RUNNING && limit > 0.0 && elapsed >= limit {
            self.handle_status_changed(status::FAILURE);
        }
        h.run_pursuit(advice, t_ms);
    }

    // ---- the outcome (§4.5) ----

    /// The aura at the vehicle's (or target's) position [question CS9].
    fn add_aura(&self, aura: (f32, f32)) {
        if aura.0 <= 0.0 {
            return;
        }
        let pos = {
            let s = self.s.borrow();
            s.active_automaton.or(s.current_target)
        };
        if let Some(o) = pos {
            let (x, z) = self.host.position(o);
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
        // 3.
        h.outcome_effect(advice, true);
        // 4.
        self.add_aura(info.success_aura);
        // 5.
        if info.success_money != 0 {
            h.deposit_funds(i64::from(info.success_money));
        }
        // 6.
        h.run_script(advice, Script::Success);
        h.retire_advice(advice);
        self.situation_untriggered(advice);
    }

    /// **Failure** (§4.5).
    pub fn failure(&self, advice: u64) {
        let h = &*self.host;
        let info = h.advice_info(advice);
        h.outcome_effect(advice, false);
        self.add_aura(info.failure_aura);
        // [question CS8]
        if info.failure_money != 0 {
            h.deposit_funds(i64::from(info.failure_money));
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

    /// Slot 35 `InitiatePlayerDriveMode(on)`.
    pub fn initiate_player_drive_mode(&self, on: bool) {
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
                        self.fill_free_slots();
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

    /// `0x09D92FC3` (§3) [question CS10].
    fn automaton_notify(&self, a: u64, subtype: u32, data3: u32) {
        let (active, st, vehicle) = {
            let s = self.s.borrow();
            (s.active_situation, s.status, s.active_automaton)
        };
        match subtype {
            notify::MOVED => {
                if let (Some(advice), true, true) = (active, st == status::RUNNING, vehicle == Some(a)) {
                    let s = self.host.check_for_success(advice, 0.0);
                    self.handle_status_changed(s);
                }
            }
            notify::REMOVED => {
                if active.is_some() && st == status::RUNNING && vehicle == Some(a) {
                    self.handle_status_changed(status::FAILURE);
                }
                if self.s.borrow().tracked.contains(&a) {
                    self.drop_automaton(a);
                }
            }
            notify::DISABLED_B => self.drop_if_tracked(a),
            notify::DISABLED_C if data3 == notify::DISABLED_C_DATA3 => self.drop_if_tracked(a),
            _ => {}
        }
    }

    fn drop_if_tracked(&self, a: u64) {
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
