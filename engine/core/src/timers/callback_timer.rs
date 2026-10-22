// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZCallbackTimer` (timers.md §2): calls a function after a delay, once or
//! repeatedly, through a timer service. The service (the scheduler of §2.5,
//! TM1) is a separate owner, injected as a [`TimerService`]; the framework
//! lookup that finds the framework's scheduler is injected as a
//! [`ServiceLocator`] returning a [`FrameworkTimerService`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

/// The framework system service a callback timer looks up (§2.2 slot 11).
pub const FRAMEWORK_TIMER_SERVICE_ID: u32 = 0xA76C_78E7;
/// Its interface.
pub const FRAMEWORK_TIMER_SERVICE_IID: u32 = 0x276C_784B;

/// The handle a service passes back to [`CallbackTimerSet::fire`] (the
/// original passes `this`).
pub type TimerKey = u64;

/// The callback `fn(arg)` (cdecl, one argument).
pub type CallbackFn = Arc<dyn Fn(usize) + Send + Sync>;

/// A counted object (`AddRef` / `Release`), kept alive while a timer runs
/// (`+0x24`). The timer calls these explicitly, as the original does.
pub trait Counted: Send + Sync {
    fn add_ref(&self);
    fn release(&self);
}

/// The timer scheduler a callback timer keeps in `+0x20` (§2.5): either the
/// built-in default instance or the one the framework service owns. It is a
/// counted reference; the timer calls `add_ref` / `release` explicitly where
/// the original does (§2.2 slot 11, the destructor), so a test can count them.
/// The scheduler itself (worker thread, due times) is a separate owner.
pub trait TimerService: Send + Sync {
    /// The scheduler's `AddRef`.
    fn add_ref(&self);
    /// The scheduler's `Release`.
    fn release(&self);
    /// Scheduler slot 3: `AddTimer(handler, this, ms, one_shot, fixed_rate)`;
    /// the returned ID 0 means failure. The service later calls
    /// [`CallbackTimerSet::fire`] with `timer`.
    fn add_timer(&self, timer: TimerKey, period: u32, one_shot: u32, fifth: u32) -> u32;
    /// Scheduler slot 4: `RemoveTimer(ID)`: true when an entry was erased.
    /// `cRZCallbackTimer` ignores the result.
    fn remove_timer(&self, id: u32) -> bool;
}

/// The framework system service `0xA76C78E7` (interface `0x276C784B`) as the
/// framework's lookup hands it out: the caller owns one reference to it and
/// gives it back with [`FrameworkTimerService::release`].
pub trait FrameworkTimerService: Send + Sync {
    /// Its slot 3 (`0x009D8564`, a plain getter): the scheduler it owns. The
    /// result is not AddRef'd for the caller; `UseTimerService` AddRefs it
    /// when it keeps it (§2.2 slot 11).
    fn scheduler(&self) -> Arc<dyn TimerService>;
    /// Gives back the reference the lookup returned.
    fn release(&self);
}

/// The framework's system-service lookup (`GetSystemService(clsid, iid)`).
/// `None` stands for both "no framework" and "service not found" (§2.2 slot
/// 11 treats them alike).
pub trait ServiceLocator: Send + Sync {
    /// Asked with [`FRAMEWORK_TIMER_SERVICE_ID`] /
    /// [`FRAMEWORK_TIMER_SERVICE_IID`]; a found service comes with one
    /// reference for the caller.
    fn system_service(&self, clsid: u32, iid: u32) -> Option<Arc<dyn FrameworkTimerService>>;
}

/// Pointer identity of two schedulers (the original compares `+0x20` with
/// the slot-3 result and with the default's address).
fn same_service(a: &Arc<dyn TimerService>, b: &Arc<dyn TimerService>) -> bool {
    std::ptr::eq(Arc::as_ptr(a) as *const (), Arc::as_ptr(b) as *const ())
}

struct State {
    /// `+0x04`: uses the framework service.
    uses_service: bool,
    /// `+0x08`, `+0x0C`.
    callback: Option<CallbackFn>,
    arg: usize,
    /// `+0x10` (stored and reported only, §4 quirk 6).
    delay: u32,
    /// `+0x14`.
    period: u32,
    /// `+0x18`, `+0x19`.
    running: bool,
    one_shot: bool,
    /// `+0x1C`.
    id: u32,
    /// `+0x20`.
    service: Arc<dyn TimerService>,
    /// `+0x24`.
    object: Option<Arc<dyn Counted>>,
}

type Shared = Arc<Mutex<State>>;

/// The last key handed out and the live timers by key.
type Members = (TimerKey, HashMap<TimerKey, Weak<Mutex<State>>>);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The global set of live callback timers and its critical section
/// (`0x00B62A9C` / `0x00B62AA8`), with the built-in default service and the
/// framework lookup.
pub struct CallbackTimerSet {
    timers: Mutex<Members>,
    default_service: Arc<dyn TimerService>,
    locator: Option<Arc<dyn ServiceLocator>>,
}

impl CallbackTimerSet {
    pub fn new(default_service: Arc<dyn TimerService>, locator: Option<Arc<dyn ServiceLocator>>) -> Arc<Self> {
        Arc::new(CallbackTimerSet {
            timers: Mutex::new((0, HashMap::new())),
            default_service,
            locator,
        })
    }

    /// The number of live timers.
    pub fn len(&self) -> usize {
        lock(&self.timers).1.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The handler the service calls (§2.4). A timer no longer in the set
    /// (destroyed) is ignored.
    pub fn fire(&self, key: TimerKey) {
        let (callback, arg, taken) = {
            let set = lock(&self.timers);
            let Some(state) = set.1.get(&key).and_then(Weak::upgrade) else {
                return;
            };
            let mut s = lock(&state);
            let callback = s.callback.clone();
            let arg = s.arg;
            let taken = s.object.clone();
            if let Some(o) = &taken {
                o.add_ref();
            }
            if s.one_shot {
                if let Some(o) = s.object.take() {
                    o.release();
                }
                s.id = 0;
                s.running = false;
                s.one_shot = false;
            }
            (callback, arg, taken)
        };
        if let Some(f) = callback {
            f(arg);
        }
        if let Some(o) = taken {
            o.release();
        }
    }
}

/// `cRZCallbackTimer`. Dropping it is the destructor (§2.2 slot 0).
pub struct CallbackTimer {
    key: TimerKey,
    set: Arc<CallbackTimerSet>,
    state: Shared,
}

impl CallbackTimer {
    /// Construction (§2.3): the initial values, joins the set, the default
    /// service, then `UseTimerService(true)`.
    pub fn new(set: &Arc<CallbackTimerSet>) -> Self {
        // `+0x04` starts false so that slot 11's "on a change" does the
        // lookup (§2.3, TM2); it is true afterwards only with a framework
        // service. The default is stored without an AddRef: §2.3 says only
        // "takes the default service" (literal reading, question TM3).
        let state = Arc::new(Mutex::new(State {
            uses_service: false,
            callback: None,
            arg: 0,
            delay: 100,
            period: 1000,
            running: false,
            one_shot: false,
            id: 0,
            service: set.default_service.clone(),
            object: None,
        }));
        let key = {
            let mut t = lock(&set.timers);
            t.0 += 1;
            let key = t.0;
            t.1.insert(key, Arc::downgrade(&state));
            key
        };
        let timer = CallbackTimer {
            key,
            set: set.clone(),
            state,
        };
        timer.use_timer_service(true);
        timer
    }

    /// The handle the service is given.
    pub fn key(&self) -> TimerKey {
        self.key
    }

    fn st(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    /// Slot 1 `SetDelay`: a running timer is restarted.
    pub fn set_delay(&self, v: u32) -> bool {
        self.st().delay = v;
        self.restart_if_running();
        true
    }

    /// Slot 2 `GetDelay`.
    pub fn delay(&self) -> u32 {
        self.st().delay
    }

    /// Slot 3 `SetPeriod`: a running timer is restarted.
    pub fn set_period(&self, v: u32) -> bool {
        self.st().period = v;
        self.restart_if_running();
        true
    }

    /// Slot 4 `GetPeriod`.
    pub fn period(&self) -> u32 {
        self.st().period
    }

    /// The restart of slots 1 and 3 (§2.2): `Stop` clears the one-shot flag
    /// first, so it is always slot 5 *[quirk]* (§4 quirk 7).
    fn restart_if_running(&self) {
        let kept = {
            let s = self.st();
            if !s.running {
                return;
            }
            s.object.clone()
        };
        if let Some(o) = &kept {
            o.add_ref();
        }
        self.stop();
        if self.st().one_shot {
            self.start_once(0, None);
        } else {
            self.start(0, None);
        }
        // The kept reference becomes the timer's again (also when the start
        // failed: then a stopped timer keeps the object, as written).
        self.st().object = kept;
    }

    /// Slot 5 `Start(period, object)`.
    pub fn start(&self, period: u32, object: Option<Arc<dyn Counted>>) -> bool {
        self.start_impl(period, object, false)
    }

    /// Slot 6 `StartOnce(delay, object)`: the delay is the argument, or
    /// `period` when it is 0. Literal reading: the argument is not stored in
    /// `period` or `delay` (question TM2).
    pub fn start_once(&self, delay: u32, object: Option<Arc<dyn Counted>>) -> bool {
        self.start_impl(delay, object, true)
    }

    fn start_impl(&self, arg: u32, object: Option<Arc<dyn Counted>>, once: bool) -> bool {
        let (service, time) = {
            let mut s = self.st();
            if s.running || s.callback.is_none() {
                return false;
            }
            if let Some(o) = &object {
                o.add_ref();
            }
            // Overwrites `+0x24` without releasing an old value (only a
            // failed restart leaves one on a stopped timer). Slot 5 always
            // stores the argument; slot 6 only a non-null one.
            if !once || object.is_some() {
                s.object = object;
            }
            let time = if once {
                s.one_shot = true;
                if arg != 0 {
                    arg
                } else {
                    s.period
                }
            } else {
                if arg != 0 {
                    s.period = arg;
                }
                s.period
            };
            s.running = true;
            (s.service.clone(), time)
        };
        let id = service.add_timer(self.key, time, u32::from(once), 0);
        let mut s = self.st();
        if id == 0 {
            // "Undoes it": not running, `+0x24` released and cleared; slot 6
            // clears the one-shot flag too (TM2 (d)). Slot 5 never set it.
            s.running = false;
            if once {
                s.one_shot = false;
            }
            if let Some(o) = s.object.take() {
                o.release();
            }
            return false;
        }
        s.id = id;
        true
    }

    /// Slot 7 `Stop()`.
    pub fn stop(&self) -> bool {
        let (service, id, object) = {
            let mut s = self.st();
            let id = s.id;
            let object = if id != 0 { s.object.take() } else { None };
            s.id = 0;
            s.running = false;
            s.one_shot = false;
            (s.service.clone(), id, object)
        };
        if id != 0 {
            service.remove_timer(id);
            if let Some(o) = object {
                o.release();
            }
        }
        true
    }

    /// Slot 8 `IsRunning()`.
    pub fn is_running(&self) -> bool {
        self.st().running
    }

    /// Slots 9, 10 `SetCallback(fn, arg)`.
    pub fn set_callback(&self, callback: Option<CallbackFn>, arg: usize) -> bool {
        let mut s = self.st();
        s.callback = callback;
        s.arg = arg;
        true
    }

    /// Slot 11 `UseTimerService(bool)` (§2.2, TM2: no change returns true).
    ///
    /// A running timer stays registered with its old service: the spec names
    /// no restart, so a later `Stop` sends the old ID to the new service
    /// (literal reading).
    pub fn use_timer_service(&self, on: bool) -> bool {
        {
            let mut s = self.st();
            if s.uses_service == on {
                return true;
            }
            s.uses_service = on;
        }
        if !on {
            // Quirk 8: the default replaces `+0x20` without releasing the
            // old service (and without an AddRef on the default).
            self.st().service = self.set.default_service.clone();
            return true;
        }
        let found = self
            .set
            .locator
            .as_ref()
            .and_then(|l| l.system_service(FRAMEWORK_TIMER_SERVICE_ID, FRAMEWORK_TIMER_SERVICE_IID));
        match found {
            Some(system) => {
                let scheduler = system.scheduler();
                let old = {
                    let mut s = self.st();
                    if same_service(&s.service, &scheduler) {
                        None
                    } else {
                        Some(std::mem::replace(&mut s.service, scheduler.clone()))
                    }
                };
                if let Some(old) = old {
                    old.release();
                    scheduler.add_ref();
                }
                system.release();
                true
            }
            None => {
                let old = {
                    let mut s = self.st();
                    s.uses_service = false;
                    if same_service(&s.service, &self.set.default_service) {
                        None
                    } else {
                        Some(std::mem::replace(&mut s.service, self.set.default_service.clone()))
                    }
                };
                if let Some(old) = old {
                    old.release();
                }
                false
            }
        }
    }

    /// `+0x20` is the built-in default scheduler.
    pub fn uses_default_service(&self) -> bool {
        same_service(&self.st().service, &self.set.default_service)
    }

    /// `+0x04`.
    pub fn uses_timer_service(&self) -> bool {
        self.st().uses_service
    }

    /// Slot 12 `GetTimerService()`.
    pub fn timer_service(&self) -> Arc<dyn TimerService> {
        self.st().service.clone()
    }

    /// `+0x19`.
    pub fn is_one_shot(&self) -> bool {
        self.st().one_shot
    }

    /// `+0x1C`.
    pub fn id(&self) -> u32 {
        self.st().id
    }

    /// `+0x24` is set.
    pub fn has_object(&self) -> bool {
        self.st().object.is_some()
    }
}

impl Drop for CallbackTimer {
    /// The destructor: leave the set, `Stop` if running, release `+0x24`
    /// and the service.
    fn drop(&mut self) {
        lock(&self.set.timers).1.remove(&self.key);
        if self.is_running() {
            self.stop();
        }
        let object = self.st().object.take();
        if let Some(o) = object {
            o.release();
        }
        // The service is released whichever it is, the default too (the
        // constructor took the default without an AddRef, question TM3).
        let service = self.st().service.clone();
        service.release();
    }
}
