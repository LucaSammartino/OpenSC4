// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The simulator service: master clock and agent scheduler
//! (`docs/specs/simulator.md`).
//!
//! Re-entrancy: every public method takes `&self`. State lives in a `RefCell`
//! that is only borrowed for short, non-reentrant sections; it is never held
//! while calling [`AgentDelivery::deliver`], [`Broadcast::post`] or the
//! reference-count hooks. Side effects produced while the state is borrowed
//! (posts, AddRef, Release) are queued and flushed, in order, right after the
//! borrow ends. An agent may therefore call back into the simulator (add or
//! remove agents, pause, change speed, ...) from inside its delivery.
//!
//! Threads: the game calls the `cISC4Simulator` interface from more than one
//! thread — a worker thread reads the clock (for example `GetSimTime`) while
//! the main thread is inside `OnTick` (in-game run 26: `RefCell already
//! mutably borrowed`, a fail-fast). A `RefCell` is not `Sync`, so every state
//! section also takes [`Simulator::gate`], a mutex held for exactly the same
//! span as the borrow and therefore never across `deliver`, `post` or the
//! reference-count hooks. Two threads are thus serialised for microseconds
//! rather than for a whole tick, so a blocked caller cannot deadlock against a
//! game lock the delivering agent needs. The only host callback reached while a
//! section is open is [`Clock::now_ms`], which must not call back into the
//! simulator (it is a clock read).

use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

use crate::calibration::Calibration;
use crate::host::{AgentHandle, Host, Message, TimerSource};
use crate::ids::{cheat, level, msg, LEVEL_COUNT};
use crate::julian::{self, DateFields, JULIAN_2000_01_01};
use crate::save::{LoadError, SaveRecord, SAVE_VERSION};

/// Tuning values read from the "Main Simulator" exemplar (spec §7).
/// `None` (or a value of the wrong length) keeps the §3 defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tuning {
    /// Property `0x89590E6D`; used only if it has exactly 3 values.
    pub ms_per_day: Option<Vec<u32>>,
    /// Property `0x89590E6E`; [choice] used only if it has exactly 5 values.
    pub animation_dilation: Option<Vec<f32>>,
}

/// A copy of the complete observable state (for tests, logging and the C ABI).
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub initialized: bool,
    pub in_tick: bool,
    pub city_established: bool,
    pub pause_count: i32,
    pub hidden_pause_count: i32,
    pub emergency_pause_count: i32,
    pub sim_speed: i32,
    pub sim_time: i32,
    pub speed_change_enabled: bool,
    pub frame_budget_ms: u32,
    pub gate_level: u32,
    pub ms_per_day: u32,
    pub target_fps: i32,
    pub ms_per_day_table: [u32; 3],
    pub animation_dilation: [f32; 5],
    pub level_counters: [u32; LEVEL_COUNT],
    pub level_fired: [bool; LEVEL_COUNT],
    pub julian: i32,
    pub agent_counts: [usize; LEVEL_COUNT],
    /// `None` = at end.
    pub cursors: [Option<usize>; LEVEL_COUNT],
    pub idle_cursor: Option<usize>,
}

struct AgentEntry {
    handle: AgentHandle,
    name: String,
    flags: u32,
    enabled: bool,
    /// Unique per registration, so a delivery can tell whether "its" entry
    /// survived the call.
    serial: u64,
}

enum Effect {
    Post(Message),
    AddRef(AgentHandle),
    Release(AgentHandle),
}

const DEFAULT_MS_PER_DAY_TABLE: [u32; 3] = [5000, 1000, 100];
const DEFAULT_ANIMATION_DILATION: [f32; 5] = [0.0, 1.0, 1.5, 2.0, 1.0];
const TARGET_FPS_BY_SPEED: [i32; 3] = [30, 20, 15];
/// simSpeed value meaning "never set" (spec §3).
const SPEED_UNSET: i32 = 3;

struct State {
    initialized: bool,
    in_tick: bool,
    city_established: bool,
    pause_count: i32,
    hidden_pause_count: i32,
    emergency_pause_count: i32,
    sim_speed: i32,
    sim_time: i32,
    speed_change_enabled: bool,
    frame_budget_ms: u32,
    gate_level: u32,
    ms_per_day: u32,
    target_fps: i32,
    ms_per_day_table: [u32; 3],
    animation_dilation: [f32; 5],
    level_counters: [u32; LEVEL_COUNT],
    level_fired: [bool; LEVEL_COUNT],
    julian: i32,
    agents: [Vec<AgentEntry>; LEVEL_COUNT],
    cursors: [Option<usize>; LEVEL_COUNT],
    idle_cursor: Option<usize>,
    frame_timer: u32,
    interval_timer: u32,
    slice_timer: u32,
    next_serial: u64,
    effects: Vec<Effect>,
}

impl State {
    fn new() -> Self {
        State {
            initialized: false,
            in_tick: false,
            city_established: false,
            pause_count: 0,
            hidden_pause_count: 0,
            emergency_pause_count: 0,
            sim_speed: SPEED_UNSET,
            sim_time: 0,
            speed_change_enabled: true,
            frame_budget_ms: 15,
            gate_level: level::DAY,
            ms_per_day: 100,
            target_fps: 15,
            ms_per_day_table: DEFAULT_MS_PER_DAY_TABLE,
            animation_dilation: DEFAULT_ANIMATION_DILATION,
            level_counters: [0; LEVEL_COUNT],
            level_fired: [false; LEVEL_COUNT],
            julian: JULIAN_2000_01_01,
            agents: Default::default(),
            cursors: [None; LEVEL_COUNT],
            idle_cursor: None,
            frame_timer: 0,
            interval_timer: 0,
            slice_timer: 0,
            next_serial: 1,
            effects: Vec::new(),
        }
    }

    fn post(&mut self, msg_type: u32, data1: i64, data2: i64, data3: i64) {
        self.effects
            .push(Effect::Post(Message::new(msg_type, data1, data2, data3)));
    }

    // ---- timers -----------------------------------------------------------

    fn slice_elapsed(&self, host: &dyn Host) -> u32 {
        host.now_ms().wrapping_sub(self.slice_timer)
    }

    // ---- agents (§4) -------------------------------------------------------

    fn enabled_for_flags(&self, flags: u32) -> bool {
        let mask = if self.city_established { 2 } else { 1 };
        flags & mask != 0
    }

    fn first_cursor(&self, l: usize) -> Option<usize> {
        if self.agents[l].is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn level_message(&self, l: usize) -> Message {
        Message::new(
            msg::SIM_LEVEL_BASE + l as u32,
            i64::from(self.julian),
            0,
            i64::from(self.level_counters[l]),
        )
    }

    fn add_agent(&mut self, handle: AgentHandle, lvl: u32, name: &str, flags: u32) -> bool {
        let l = lvl as usize;
        if l >= LEVEL_COUNT {
            return true;
        }
        if self.agents[l].iter().any(|e| e.handle == handle) {
            return true;
        }
        let enabled = self.enabled_for_flags(flags);
        let serial = self.next_serial;
        self.next_serial += 1;
        // [choice] registration order (spec §4).
        self.agents[l].push(AgentEntry {
            handle,
            name: name.to_owned(),
            flags,
            enabled,
            serial,
        });
        self.effects.push(Effect::AddRef(handle));
        true
    }

    /// Removes entry `i` of level `l`. A cursor on it moves to the next agent.
    fn remove_at(&mut self, l: usize, i: usize) {
        let entry = self.agents[l].remove(i);
        let len = self.agents[l].len();
        let fix = |c: &mut Option<usize>| {
            if let Some(ci) = *c {
                if ci > i {
                    *c = Some(ci - 1);
                } else if ci == i && i >= len {
                    *c = None;
                }
            }
        };
        fix(&mut self.cursors[l]);
        if l == level::IDLE as usize {
            fix(&mut self.idle_cursor);
        }
        self.effects.push(Effect::Release(entry.handle));
    }

    fn remove_agent(&mut self, handle: AgentHandle, lvl: u32) -> bool {
        let l = lvl as usize;
        if l >= LEVEL_COUNT {
            return false;
        }
        match self.agents[l].iter().position(|e| e.handle == handle) {
            Some(i) => {
                self.remove_at(l, i);
                true
            }
            None => false,
        }
    }

    fn remove_all_agents_at(&mut self, l: usize) {
        for e in self.agents[l].drain(..) {
            self.effects.push(Effect::Release(e.handle));
        }
        self.cursors[l] = None;
        if l == level::IDLE as usize {
            self.idle_cursor = None;
        }
    }

    fn advance_cursor(&mut self, l: usize) {
        if let Some(i) = self.cursors[l] {
            self.cursors[l] = if i + 1 < self.agents[l].len() {
                Some(i + 1)
            } else {
                None
            };
        }
    }

    /// After a delivery to the registration `serial` at `cursors[l]`: advance
    /// past it (or remove it). If the registration disappeared during the
    /// call, the removal already moved the cursor, so nothing is done.
    fn finish_at_cursor(&mut self, l: usize, serial: u64, remove: bool) {
        if let Some(i) = self.cursors[l] {
            if self.agents[l][i].serial == serial {
                if remove {
                    self.remove_at(l, i);
                } else {
                    self.advance_cursor(l);
                }
            }
        }
    }

    fn cursor_target(&self, l: usize) -> Option<(AgentHandle, u64, bool, Message)> {
        self.cursors[l].map(|i| {
            let e = &self.agents[l][i];
            (e.handle, e.serial, e.enabled, self.level_message(l))
        })
    }

    // ---- speed and pause (§6) -------------------------------------------

    fn pause(&mut self) -> bool {
        self.pause_count += 1;
        if self.pause_count == 1 {
            self.post(msg::PAUSE_CHANGED, 1, 0, 0);
        }
        true
    }

    fn resume(&mut self) -> bool {
        if self.speed_change_enabled {
            self.pause_count -= 1;
            if self.pause_count == 0 {
                self.post(msg::PAUSE_CHANGED, 0, 0, 0);
            }
        }
        true
    }

    fn do_set_sim_speed(&mut self, speed: i32, host: &dyn Host) {
        // §6 [compat]: any speed outside {0, 1, 2} changes nothing and posts nothing.
        if !(0..=2).contains(&speed) {
            return;
        }
        self.target_fps = TARGET_FPS_BY_SPEED[speed as usize];
        self.ms_per_day = self.ms_per_day_table[speed as usize];
        self.sim_speed = speed;
        self.interval_timer = host.now_ms();
        self.post(msg::SPEED_CHANGED, i64::from(speed), 0, 0);
    }

    fn set_sim_speed(&mut self, speed: i32, host: &dyn Host) -> bool {
        if !self.speed_change_enabled {
            return false;
        }
        if speed != self.sim_speed {
            self.do_set_sim_speed(speed, host);
        }
        true
    }

    // ---- clock (§5.2) ----------------------------------------------------

    fn simulate_time_passage(&mut self, host: &dyn Host) {
        self.level_fired = [false; LEVEL_COUNT];
        if !self.agents[level::ONE_SHOT as usize].is_empty() || !self.agents[level::EMERGENCY as usize].is_empty() {
            return;
        }
        let ready = host.now_ms().wrapping_sub(self.interval_timer) >= self.ms_per_day;
        let done = |s: &State, l: u32| s.cursors[l as usize].is_none();
        let gate = self.gate_level;

        let tomorrow = julian::date_fields(self.julian + 1);
        let starts_week = tomorrow.weekday == 1;
        let starts_month = tomorrow.day_of_month == 1;
        let starts_year = tomorrow.day_of_year == 1;

        const MIN: usize = level::MINUTE as usize;
        const HOUR: usize = level::HOUR as usize;

        // Minute.
        if done(self, level::MINUTE) && (gate != level::MINUTE || ready) {
            if gate == level::MINUTE {
                self.interval_timer = host.now_ms();
            }
            if self.agents[MIN].is_empty() {
                self.level_counters[MIN] = 60;
            } else {
                self.level_fired[MIN] = true;
                self.level_counters[MIN] = self.level_counters[MIN].wrapping_add(1);
            }
        }

        // Hour.
        if self.level_counters[MIN] >= 60 && done(self, level::HOUR) && (gate != level::HOUR || ready) {
            self.level_counters[MIN] = 0;
            if gate == level::HOUR {
                self.interval_timer = host.now_ms();
            }
            if self.agents[HOUR].is_empty() {
                self.level_counters[HOUR] = 24;
            } else {
                self.level_fired[HOUR] = true;
                self.level_counters[HOUR] = self.level_counters[HOUR].wrapping_add(1);
            }
        }

        // Day.
        let blocked = (starts_week && !done(self, level::WEEK))
            || (starts_month && !done(self, level::MONTH))
            || (starts_year && !done(self, level::YEAR));
        if self.level_counters[HOUR] >= 24 && (gate != level::DAY || ready) && done(self, level::DAY) && !blocked {
            self.level_counters[HOUR] = 0;
            if gate == level::DAY {
                self.interval_timer = host.now_ms();
            }
            self.level_fired[level::DAY as usize] = true;
            self.julian += 1;
            let d: DateFields = julian::date_fields(self.julian);
            let jd = i64::from(self.julian);

            self.level_counters[level::DAY as usize] = d.day_of_year as u32;
            self.post(msg::SIM_NEW_DAY, jd, 0, i64::from(d.day_of_year));
            if d.weekday == 1 {
                let w = julian::week_of_year(d.day_of_year);
                self.level_fired[level::WEEK as usize] = true;
                self.level_counters[level::WEEK as usize] = w as u32;
                self.post(msg::SIM_NEW_WEEK, jd, 0, i64::from(w));
            }
            if d.day_of_month == 1 {
                self.level_fired[level::MONTH as usize] = true;
                self.level_counters[level::MONTH as usize] = d.month as u32;
                self.post(msg::SIM_NEW_MONTH, jd, 0, i64::from(d.month));
            }
            if d.day_of_year == 1 {
                self.level_fired[level::YEAR as usize] = true;
                self.level_counters[level::YEAR as usize] = d.year as u32;
                self.post(msg::SIM_NEW_YEAR, jd, 0, i64::from(d.year));
            }
        }
    }

    fn snapshot(&self) -> Snapshot {
        let mut agent_counts = [0usize; LEVEL_COUNT];
        for (l, v) in self.agents.iter().enumerate() {
            agent_counts[l] = v.len();
        }
        Snapshot {
            initialized: self.initialized,
            in_tick: self.in_tick,
            city_established: self.city_established,
            pause_count: self.pause_count,
            hidden_pause_count: self.hidden_pause_count,
            emergency_pause_count: self.emergency_pause_count,
            sim_speed: self.sim_speed,
            sim_time: self.sim_time,
            speed_change_enabled: self.speed_change_enabled,
            frame_budget_ms: self.frame_budget_ms,
            gate_level: self.gate_level,
            ms_per_day: self.ms_per_day,
            target_fps: self.target_fps,
            ms_per_day_table: self.ms_per_day_table,
            animation_dilation: self.animation_dilation,
            level_counters: self.level_counters,
            level_fired: self.level_fired,
            julian: self.julian,
            agent_counts,
            cursors: self.cursors,
            idle_cursor: self.idle_cursor,
        }
    }
}

/// The simulator service. See the module documentation.
pub struct Simulator {
    /// The clock and scheduler state. A `Mutex`, not a `RefCell`: the game
    /// calls this interface from more than one thread (module documentation).
    /// It is only ever locked for the span of one [`Simulator::with`] or
    /// [`Simulator::read`] section.
    state: Mutex<State>,
    host: Box<dyn Host>,
    /// Optional limit-timer calibration (§5.5): its source/sink and state.
    /// Only the main thread reaches it (`on_tick`'s calibration step and the
    /// host's install/read of the coefficients), so a `RefCell` is enough.
    timers: RefCell<Option<(Box<dyn TimerSource>, Calibration)>>,
}

impl Simulator {
    pub fn new(host: Box<dyn Host>) -> Self {
        Simulator {
            state: Mutex::new(State::new()),
            host,
            timers: RefCell::new(None),
        }
    }

    pub fn host(&self) -> &dyn Host {
        &*self.host
    }

    /// Locks the state for one section. A poisoned lock is used anyway: the
    /// state is still readable, and a release build aborts on the panic that
    /// poisoned it, so no caller can observe the difference.
    fn gate(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run `f` with the state borrowed, then flush the side effects it queued.
    /// The lock is released before the effects run, because `post` and the
    /// reference-count hooks call into the game (module documentation).
    fn with<R>(&self, f: impl FnOnce(&mut State, &dyn Host) -> R) -> R {
        let (result, effects) = {
            let mut s = self.gate();
            let result = f(&mut s, &*self.host);
            (result, std::mem::take(&mut s.effects))
        };
        for e in effects {
            match e {
                Effect::Post(m) => self.host.post(&m),
                Effect::AddRef(h) => self.host.add_ref(h),
                Effect::Release(h) => self.host.release(h),
            }
        }
        result
    }

    fn read<R>(&self, f: impl FnOnce(&State) -> R) -> R {
        f(&self.gate())
    }

    // ---- lifecycle (§9) --------------------------------------------------

    /// `Init()` steps 1, 2, 5, 6, 7 (the GZCOM registrations of steps 3, 4, 8
    /// and 9 are the shim's job). Does nothing if already initialized.
    pub fn init(&self, tuning: &Tuning) -> bool {
        self.with(|s, host| {
            if s.initialized {
                return true; // [choice] report success
            }
            s.initialized = true;
            s.hidden_pause_count = 1;

            if let Some(t) = &tuning.ms_per_day {
                if t.len() == 3 {
                    s.ms_per_day_table.copy_from_slice(t);
                }
            }
            if let Some(d) = &tuning.animation_dilation {
                if d.len() == 5 {
                    s.animation_dilation.copy_from_slice(d);
                }
            }

            for l in 0..LEVEL_COUNT {
                s.cursors[l] = s.first_cursor(l);
            }
            s.idle_cursor = s.first_cursor(level::IDLE as usize);

            if s.sim_speed == SPEED_UNSET {
                s.set_sim_speed(0, host);
            } else {
                let speed = s.sim_speed;
                s.do_set_sim_speed(speed, host);
            }

            let now = host.now_ms();
            s.frame_timer = now;
            s.interval_timer = now;
            true
        })
    }

    /// `Shutdown()` steps 2, 3 and 7. Does nothing if not initialized.
    pub fn shutdown(&self) -> bool {
        self.with(|s, _| {
            if !s.initialized {
                return true; // [choice] report success
            }
            s.julian = JULIAN_2000_01_01;
            s.sim_speed = SPEED_UNSET;
            for l in 0..LEVEL_COUNT {
                s.remove_all_agents_at(l);
            }
            s.initialized = false;
            true
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.read(|s| s.initialized)
    }

    // ---- per-frame behaviour (§5) ----------------------------------------

    /// `cIGZSystemService::OnTick` (§5.1). Always returns true.
    pub fn on_tick(&self) -> bool {
        // §5.5: calibrate the limit timers before anything else.
        self.calibrate_tick();
        let run = self.with(|s, host| {
            let now = host.now_ms();
            let elapsed = now.wrapping_sub(s.frame_timer);
            s.frame_timer = now;
            let target: i64 = if s.pause_count <= 0 && s.hidden_pause_count <= 0 {
                // [choice] guard against a non-positive fps (FPS cheat).
                1000 / i64::from(s.target_fps.max(1))
            } else {
                33
            };
            s.frame_budget_ms = (target - i64::from(elapsed)).max(15) as u32;
            if s.initialized && !s.in_tick {
                s.in_tick = true;
                s.slice_timer = host.now_ms();
                true
            } else {
                false
            }
        });
        if !run {
            return true;
        }
        loop {
            let (hidden, paused) = self.read(|s| (s.hidden_pause_count > 0, s.pause_count > 0));
            if hidden {
                break;
            }
            if !paused {
                self.simulate_time_passage();
                self.simulate_agents();
            }
            self.simulate_idle_time();
            if self.read(|s| s.slice_elapsed(&*self.host) >= s.frame_budget_ms) {
                break;
            }
        }
        self.with(|s, _| s.in_tick = false);
        true
    }

    /// §5.2
    pub fn simulate_time_passage(&self) {
        self.with(|s, host| s.simulate_time_passage(host));
    }

    /// §5.3
    pub fn simulate_agents(&self) {
        let fired = self.read(|s| s.level_fired);
        for l in level::MINUTE..=level::YEAR {
            if fired[l as usize] {
                self.begin_new_interval(l);
            }
        }

        enum Next {
            Advance,
            Again,
            Deliver(AgentHandle, u64, bool, Message),
        }

        let mut l = 0usize;
        while l < level::IDLE as usize {
            let next = self.with(|s, _| match s.cursor_target(l) {
                None => {
                    if l <= level::EMERGENCY as usize && !s.agents[l].is_empty() {
                        // [choice] continuous levels wrap to their first agent.
                        s.cursors[l] = Some(0);
                        Next::Again
                    } else {
                        Next::Advance
                    }
                }
                Some((h, serial, enabled, m)) => Next::Deliver(h, serial, enabled, m),
            });
            match next {
                Next::Advance => l += 1,
                Next::Again => {}
                Next::Deliver(handle, serial, enabled, m) => {
                    let finished = !enabled || self.host.deliver(self, handle, &m);
                    let stop = self.with(|s, host| {
                        if finished {
                            // A finished one-shot agent is removed; a disabled one is
                            // only skipped ([choice], see report).
                            let remove = enabled && l == level::ONE_SHOT as usize;
                            s.finish_at_cursor(l, serial, remove);
                        }
                        s.slice_elapsed(host) >= s.frame_budget_ms
                    });
                    if stop {
                        return;
                    }
                }
            }
        }
    }

    /// `BeginNewInterval(L)` (§5.3 step 1): flush the level's unfinished work
    /// synchronously, then rewind its cursor.
    pub fn begin_new_interval(&self, lvl: u32) {
        let l = lvl as usize;
        if l >= LEVEL_COUNT || self.read(|s| s.agents[l].is_empty()) {
            return;
        }
        while let Some((handle, serial, enabled, m)) = self.read(|s| s.cursor_target(l)) {
            let finished = !enabled || self.host.deliver(self, handle, &m);
            if finished {
                self.with(|s, _| s.finish_at_cursor(l, serial, false));
            }
            // Not finished: deliver again (if the agent was removed meanwhile,
            // the cursor already points at the next one).
        }
        self.with(|s, _| s.cursors[l] = s.first_cursor(l));
    }

    /// §5.4
    pub fn simulate_idle_time(&self) {
        const IDLE: usize = level::IDLE as usize;
        let share = self.with(|s, _| {
            if s.hidden_pause_count > 0 || s.agents[IDLE].is_empty() {
                return None;
            }
            if s.idle_cursor.is_none() {
                s.idle_cursor = Some(0);
            }
            Some(s.frame_budget_ms / 3)
        });
        let Some(share) = share else { return };
        loop {
            let step = self.with(|s, host| {
                let i = s.idle_cursor?;
                let t = s.slice_elapsed(host);
                if t >= s.frame_budget_ms {
                    return None;
                }
                let (handle, enabled) = (s.agents[IDLE][i].handle, s.agents[IDLE][i].enabled);
                s.idle_cursor = if i + 1 < s.agents[IDLE].len() {
                    Some(i + 1)
                } else {
                    None
                };
                let paused = i64::from(s.pause_count > 0);
                let budget_left = (s.frame_budget_ms - t).min(share);
                Some((
                    handle,
                    enabled,
                    Message::new(msg::SIM_IDLE, i64::from(s.julian), paused, i64::from(budget_left)),
                ))
            });
            let Some((handle, enabled, m)) = step else { break };
            if enabled {
                let _ = self.host.deliver(self, handle, &m); // result ignored
            }
        }
    }

    // ---- agents (§4) -----------------------------------------------------

    pub fn add_agent(&self, handle: AgentHandle, lvl: u32, name: &str, flags: u32) -> bool {
        self.with(|s, _| s.add_agent(handle, lvl, name, flags))
    }

    pub fn remove_agent(&self, handle: AgentHandle, lvl: u32) -> bool {
        self.with(|s, _| s.remove_agent(handle, lvl))
    }

    /// `RemoveAgent(target)`: every level.
    pub fn remove_agent_all_levels(&self, handle: AgentHandle) -> bool {
        self.with(|s, _| {
            let mut any = false;
            for l in 0..LEVEL_COUNT as u32 {
                any |= s.remove_agent(handle, l);
            }
            any
        })
    }

    /// `RemoveAllAgents(level)`. [choice] an invalid level does nothing and
    /// still returns true.
    pub fn remove_all_agents_at(&self, lvl: u32) -> bool {
        self.with(|s, _| {
            if (lvl as usize) < LEVEL_COUNT {
                s.remove_all_agents_at(lvl as usize);
            }
            true
        })
    }

    pub fn remove_all_agents(&self) -> bool {
        self.with(|s, _| {
            for l in 0..LEVEL_COUNT {
                s.remove_all_agents_at(l);
            }
            true
        })
    }

    pub fn set_city_established(&self, established: bool) -> bool {
        self.with(|s, _| {
            if s.city_established != established {
                s.city_established = established;
                let mask = if established { 2 } else { 1 };
                for e in s.agents.iter_mut().flatten() {
                    e.enabled = e.flags & mask != 0;
                }
            }
            true
        })
    }

    pub fn is_city_established(&self) -> bool {
        self.read(|s| s.city_established)
    }

    pub fn get_agent_enabled(&self, name: &str) -> bool {
        self.read(|s| s.agents.iter().flatten().any(|e| e.enabled && e.name == name))
    }

    pub fn set_agent_enabled(&self, name: &str, enabled: bool) -> bool {
        self.with(|s, _| {
            for e in s.agents.iter_mut().flatten().filter(|e| e.name == name) {
                e.enabled = enabled;
            }
            true
        })
    }

    /// Names of every registration, level by level.
    pub fn enumerate_agent_names(&self) -> Vec<String> {
        self.read(|s| s.agents.iter().flatten().map(|e| e.name.clone()).collect())
    }

    // ---- speed and pause (§6) ------------------------------------------------

    pub fn pause(&self) -> bool {
        self.with(|s, _| s.pause())
    }

    pub fn resume(&self) -> bool {
        self.with(|s, _| s.resume())
    }

    pub fn hidden_pause(&self) -> bool {
        self.with(|s, _| {
            s.hidden_pause_count += 1;
            if s.hidden_pause_count == 1 {
                s.post(msg::HIDDEN_PAUSE_CHANGED, 1, 0, 0);
            }
            true
        })
    }

    pub fn hidden_resume(&self) -> bool {
        self.with(|s, _| {
            s.hidden_pause_count -= 1;
            if s.hidden_pause_count == 0 {
                s.post(msg::HIDDEN_PAUSE_CHANGED, 0, 0, 0);
            }
            true
        })
    }

    /// Notification `0x26D31EC1` (PostCityInit, spec §9): the city finished
    /// initialising, so release the hidden pause that `init` set. Unsubscribing
    /// from the notification is the host's job.
    pub fn on_post_city_init(&self) -> bool {
        self.hidden_resume()
    }

    pub fn emergency_pause(&self) -> bool {
        self.with(|s, _| {
            s.pause();
            s.speed_change_enabled = false;
            s.emergency_pause_count += 1;
            if s.emergency_pause_count == 1 {
                s.post(msg::EMERGENCY_PAUSE_CHANGED, 1, 0, 0);
            }
            true
        })
    }

    pub fn emergency_resume(&self) -> bool {
        self.with(|s, _| {
            s.speed_change_enabled = true;
            s.resume();
            s.emergency_pause_count -= 1;
            if s.emergency_pause_count == 0 {
                s.post(msg::EMERGENCY_PAUSE_CHANGED, 0, 0, 0);
            }
            true
        })
    }

    pub fn is_paused(&self) -> bool {
        self.read(|s| s.pause_count > 0)
    }

    pub fn is_hidden_paused(&self) -> bool {
        self.read(|s| s.hidden_pause_count > 0)
    }

    pub fn is_emergency_paused(&self) -> bool {
        self.read(|s| s.emergency_pause_count > 0)
    }

    pub fn is_any_paused(&self) -> bool {
        self.read(|s| s.pause_count > 0 || s.hidden_pause_count > 0 || s.emergency_pause_count > 0)
    }

    pub fn sim_speed(&self) -> i32 {
        self.read(|s| s.sim_speed)
    }

    pub fn set_sim_speed(&self, speed: i32) -> bool {
        self.with(|s, host| s.set_sim_speed(speed, host))
    }

    pub fn sim_time(&self) -> i32 {
        self.read(|s| s.sim_time)
    }

    pub fn set_sim_time(&self, t: i32) -> bool {
        self.with(|s, _| {
            s.sim_time = t;
            true
        })
    }

    /// Accepted and ignored ([compat]).
    pub fn set_max_milliseconds_per_tick(&self, _ms: u32) -> bool {
        true
    }

    pub fn animation_time_dilation(&self) -> f32 {
        self.read(|s| {
            let d = &s.animation_dilation;
            if s.emergency_pause_count > 0 {
                d[4]
            } else if s.pause_count > 0 {
                d[0]
            } else {
                match s.sim_speed {
                    1 => d[2],
                    2 => d[3],
                    _ => d[1],
                }
            }
        })
    }

    // ---- date --------------------------------------------------------------

    pub fn julian_day(&self) -> i32 {
        self.read(|s| s.julian)
    }

    pub fn date_fields(&self) -> DateFields {
        julian::date_fields(self.julian_day())
    }

    // ---- persistence (§8) ------------------------------------------------

    pub fn save_record(&self) -> SaveRecord {
        self.read(|s| SaveRecord {
            version: SAVE_VERSION,
            in_tick: s.in_tick,
            city_established: s.city_established,
            pause_count: s.pause_count,
            sim_speed: s.sim_speed,
            sim_time: s.sim_time,
            frame_budget_ms: s.frame_budget_ms,
            level_counters: s.level_counters,
            level_fired: s.level_fired,
            julian_day: s.julian as u32,
        })
    }

    pub fn save(&self) -> Vec<u8> {
        self.save_record().encode()
    }

    /// Load a record. On error nothing is changed ([choice]).
    /// `inTick` is ignored; `cityEstablished` is stored without recomputing
    /// agent enabled bits ([choice]); speed is re-applied by the next `init`.
    pub fn load(&self, bytes: &[u8]) -> Result<(), LoadError> {
        let rec = SaveRecord::decode(bytes)?;
        self.with(|s, _| {
            s.city_established = rec.city_established;
            s.pause_count = rec.pause_count;
            s.sim_speed = rec.sim_speed;
            s.sim_time = rec.sim_time;
            s.frame_budget_ms = rec.frame_budget_ms;
            s.level_counters = rec.level_counters;
            s.level_fired = rec.level_fired;
            s.julian = rec.julian_day as i32;
        });
        Ok(())
    }

    // ---- cheats (§9) -----------------------------------------------------

    /// Handle a cheat notification. `text` is the full cheat line as typed,
    /// e.g. `"FPS 30"` or `"SimDate 3 15 2001"`. Returns true if `cheat_id` is
    /// one of ours.
    pub fn handle_cheat(&self, cheat_id: u32, text: &str) -> bool {
        let args = |name: &str| -> Vec<i64> {
            let mut toks = text.split_whitespace().peekable();
            // [choice] skip the leading cheat name if present.
            if toks.peek().is_some_and(|t| t.eq_ignore_ascii_case(name)) {
                toks.next();
            }
            // [choice] stop at the first token that is not an integer.
            toks.map_while(|t| t.parse::<i64>().ok()).collect()
        };
        match cheat_id {
            cheat::FPS => {
                let a = args(cheat::FPS_TEXT);
                // [choice] a missing, non-positive or oversized value is ignored.
                if let Some(&n) = a.first() {
                    if n > 0 && n <= i64::from(i32::MAX) {
                        self.with(|s, _| s.target_fps = n as i32);
                    }
                }
                true
            }
            cheat::SIM_DATE => {
                let a = args(cheat::SIM_DATE_TEXT);
                if a.len() >= 2 {
                    let current = self.date_fields();
                    let year = a.get(2).copied().unwrap_or(i64::from(current.year));
                    let fits = |v: i64| i32::try_from(v).ok();
                    // [choice] an invalid date is ignored (and time does not pass).
                    if let (Some(m), Some(d), Some(y)) = (fits(a[0]), fits(a[1]), fits(year)) {
                        if julian::is_valid_civil_date(y, m, d) {
                            let jd = julian::julian_from_civil(y, m, d);
                            self.with(|s, _| s.julian = jd);
                            self.simulate_time_passage();
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    // ---- limit-timer calibration (§5.5) -----------------------------------

    /// Installs the calibration source/sink. `initial_ms_per_cycle` is the
    /// host's estimate (milliseconds per TSC cycle). Reads the wall clock and
    /// the TSC as the first sample point and publishes both coefficients.
    /// Returns false (and changes nothing) for an invalid estimate or a zero
    /// wall-clock frequency.
    pub fn set_timer_source(&self, source: Box<dyn TimerSource>, initial_ms_per_cycle: f32) -> bool {
        if source.qpc_frequency() == 0 {
            return false;
        }
        let Some(cal) = Calibration::new(initial_ms_per_cycle, source.qpc(), source.tsc()) else {
            return false;
        };
        source.publish(cal.cycles_to_ms(), cal.ms_to_cycles());
        *self.timers.borrow_mut() = Some((source, cal));
        true
    }

    /// One calibration step (§5.5), then publish. `on_tick` calls this first;
    /// it is public for tests. Does nothing without a timer source.
    pub fn calibrate_tick(&self) {
        let mut timers = self.timers.borrow_mut();
        if let Some((source, cal)) = timers.as_mut() {
            let q = source.qpc();
            let c = source.tsc();
            cal.tick(q, c, source.qpc_frequency());
            // [choice] publish on every tick (cheap; keeps the globals right even
            // if something else overwrote them).
            source.publish(cal.cycles_to_ms(), cal.ms_to_cycles());
        }
    }

    /// The current calibration, if a timer source is installed.
    pub fn calibration(&self) -> Option<Calibration> {
        self.timers.borrow().as_ref().map(|(_, c)| *c)
    }

    // ---- inspection --------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        self.read(|s| s.snapshot())
    }

    /// Scheduler/barrier state for diagnostics: why the next day can or
    /// cannot open (§5.2 step 8). Reads the clock once; changes nothing.
    pub fn debug_info(&self) -> DebugInfo {
        self.read(|s| {
            let now = self.host.now_ms();
            let interval_elapsed_ms = now.wrapping_sub(s.interval_timer);
            let ready = interval_elapsed_ms >= s.ms_per_day;
            let done = |l: u32| s.cursors[l as usize].is_none();
            let tomorrow = julian::date_fields(s.julian + 1);
            let starts_week = tomorrow.weekday == 1;
            let starts_month = tomorrow.day_of_month == 1;
            let starts_year = tomorrow.day_of_year == 1;
            let blocked_by_week = starts_week && !done(level::WEEK);
            let blocked_by_month = starts_month && !done(level::MONTH);
            let blocked_by_year = starts_year && !done(level::YEAR);
            let one_shot_or_emergency_pending =
                !s.agents[level::ONE_SHOT as usize].is_empty() || !s.agents[level::EMERGENCY as usize].is_empty();
            let hours_complete = s.level_counters[level::HOUR as usize] >= 24;
            let gate_ready = s.gate_level != level::DAY || ready;
            DebugInfo {
                interval_elapsed_ms,
                slice_elapsed_ms: now.wrapping_sub(s.slice_timer),
                ms_per_day: s.ms_per_day,
                ready,
                one_shot_or_emergency_pending,
                hours_complete,
                day_level_done: done(level::DAY),
                tomorrow_starts_week: starts_week,
                tomorrow_starts_month: starts_month,
                tomorrow_starts_year: starts_year,
                blocked_by_week,
                blocked_by_month,
                blocked_by_year,
                day_gate_open: !one_shot_or_emergency_pending
                    && hours_complete
                    && gate_ready
                    && done(level::DAY)
                    && !blocked_by_week
                    && !blocked_by_month
                    && !blocked_by_year,
                cursors: s.cursors,
                idle_cursor: s.idle_cursor,
            }
        })
    }

    /// The registration at `index` of `lvl` (registration order), for diagnostics.
    pub fn agent_info(&self, lvl: u32, index: usize) -> Option<AgentInfo> {
        self.read(|s| {
            s.agents.get(lvl as usize)?.get(index).map(|e| AgentInfo {
                handle: e.handle,
                name: e.name.clone(),
                flags: e.flags,
                enabled: e.enabled,
            })
        })
    }
}

/// See [`Simulator::debug_info`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugInfo {
    pub interval_elapsed_ms: u32,
    pub slice_elapsed_ms: u32,
    pub ms_per_day: u32,
    /// interval timer >= msPerDay
    pub ready: bool,
    /// level 0 or 1 has agents: calendar time is suspended (§5.2 step 2)
    pub one_shot_or_emergency_pending: bool,
    /// levelCounter[Hour] >= 24
    pub hours_complete: bool,
    /// cursor[Day] at end
    pub day_level_done: bool,
    pub tomorrow_starts_week: bool,
    pub tomorrow_starts_month: bool,
    pub tomorrow_starts_year: bool,
    pub blocked_by_week: bool,
    pub blocked_by_month: bool,
    pub blocked_by_year: bool,
    /// all §5.2 step 8 conditions hold right now
    pub day_gate_open: bool,
    pub cursors: [Option<usize>; LEVEL_COUNT],
    pub idle_cursor: Option<usize>,
}

/// See [`Simulator::agent_info`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentInfo {
    pub handle: AgentHandle,
    pub name: String,
    pub flags: u32,
    pub enabled: bool,
}

impl Drop for Simulator {
    /// Release every remaining registration so hosts can balance references.
    fn drop(&mut self) {
        let handles: Vec<AgentHandle> = self
            .state
            // A poisoned lock still gives the state back; see `gate`.
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .agents
            .iter_mut()
            .flat_map(|v| v.drain(..))
            .map(|e| e.handle)
            .collect();
        for h in handles {
            self.host.release(h);
        }
    }
}
