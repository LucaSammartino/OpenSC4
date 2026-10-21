// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! timers.md §6 conformance tests and the edge rules of §1.3 and §2.

use super::*;
use std::cell::Cell;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const F: u64 = 1_048_576;

/// Injected counters: `qpc` and `rdtsc` return their cells; `sleep0`
/// advances both by `step`.
#[derive(Default)]
struct Clock {
    qpc: Cell<u64>,
    tsc: Cell<u64>,
    step: Cell<u64>,
    sleeps: Cell<u32>,
}

impl Clock {
    fn at(t: u64) -> Self {
        let c = Clock::default();
        c.qpc.set(t);
        c.tsc.set(t);
        c
    }
    fn set(&self, t: u64) {
        self.qpc.set(t);
        self.tsc.set(t);
    }
}

impl TimerHost for Clock {
    fn qpc(&self) -> u64 {
        self.qpc.get()
    }
    fn rdtsc(&self) -> u64 {
        self.tsc.get()
    }
    fn sleep0(&self) {
        self.sleeps.set(self.sleeps.get() + 1);
        self.qpc.set(self.qpc.get() + self.step.get());
        self.tsc.set(self.tsc.get() + self.step.get());
    }
}

fn cal() -> Calibration {
    Calibration::new(F, F, 3)
}

#[test]
fn t1_units() {
    let cal = cal();
    let h = Clock::at(1000);
    let expect: [(i32, u64); 6] = [
        (1, 1_048_576),
        (2, 1_000_000_000),
        (3, 1_000_000),
        (4, 1000),
        (5, 1),
        (6, 0),
    ];
    for (unit, want) in expect {
        let mut t = RzTimer::new(unit);
        h.set(1000);
        t.start(&h);
        h.set(1_049_579);
        assert_eq!(t.elapsed_ticks(&h, &cal), 1_048_576, "unit {unit}");
        assert_eq!(t.elapsed_u64(&h, &cal), want, "unit {unit}");
        assert_eq!(u64::from(t.elapsed_u32(&h, &cal)), want, "unit {unit}");
        assert!(t.is_running());
    }
    // The float form: exact for the power-of-two factors.
    let mut t = RzTimer::new(4);
    h.set(1000);
    t.start(&h);
    h.set(1_049_579);
    assert_eq!(t.elapsed_f32(&h, &cal), 1000.0);
}

#[test]
fn t2_overhead() {
    let cal = cal();
    for (stop, want) in [(102, 1), (103, 1), (200, 97)] {
        let h = Clock::at(100);
        let mut t = RzTimer::new(1);
        t.start(&h);
        h.set(stop);
        t.stop(&h, &cal);
        assert_eq!(t.accumulated, want);
        assert!(!t.is_running());
    }
}

#[test]
fn t3_accumulation_reset_restart() {
    let cal = cal();
    let h = Clock::at(10);
    let mut t = RzTimer::new(1);
    for _ in 0..2 {
        let s = h.qpc();
        t.start(&h);
        h.set(s + 1003);
        t.stop(&h, &cal);
    }
    assert_eq!(t.accumulated, 2000);
    t.reset();
    assert_eq!((t.accumulated, t.is_running(), t.unit), (0, false, 1));
    h.set(5000);
    t.restart(&h);
    assert_eq!((t.accumulated, t.start), (0, 5000));
    // Restart on a running timer drops what it had.
    h.set(6000);
    t.stop(&h, &cal);
    t.start(&h);
    h.set(7000);
    t.restart(&h);
    assert_eq!((t.accumulated, t.start), (0, 7000));
}

#[test]
fn t4_set_elapsed() {
    let cal = cal();
    let h = Clock::at(42);
    let mut t = RzTimer::new(5);
    t.set_elapsed_u64(2, &h, &cal);
    assert_eq!(t.accumulated, 2_097_152);
    assert_eq!(t.start, 42);
    let mut u = RzTimer::new(9);
    u.accumulated = 77;
    u.set_elapsed_u64(7, &h, &cal);
    assert_eq!(u.accumulated, 77);
    assert!(u.is_running());
    u.set_elapsed_f32(7.0, &h, &cal);
    assert_eq!(u.accumulated, 77);
    // Units 0-1 store the value; the float form truncates toward zero.
    let mut r = RzTimer::new(1);
    r.set_elapsed_f32(12.9, &h, &cal);
    assert_eq!(r.accumulated, 12);
    r.set_elapsed_u64(u64::MAX, &h, &cal);
    assert_eq!(r.accumulated, u64::MAX);
    // Units 2-6 float form: 3 ms = 3 × 1048.576 ticks, truncated.
    let mut m = RzTimer::new(4);
    m.set_elapsed_f32(3.0, &h, &cal);
    assert_eq!(m.accumulated, 3145);
    // A negative signed value (u64 ≥ 2^63) truncates to a negative i64.
    let mut s = RzTimer::new(5);
    s.set_elapsed_u64(u64::MAX, &h, &cal);
    assert_eq!(s.accumulated as i64, -1_048_576);
}

#[test]
fn t5_unknown_unit() {
    let cal = cal();
    let h = Clock::at(100);
    let mut t = RzTimer::new(9);
    t.start(&h);
    h.set(200);
    assert_eq!(t.elapsed_u32(&h, &cal), 0);
    assert_eq!(t.elapsed_u64(&h, &cal), 0);
    assert_eq!(t.elapsed_f32(&h, &cal), 0.0);
    t.stop(&h, &cal);
    assert_eq!(t.accumulated, 97);
}

#[test]
fn unit0_quirks() {
    let cal = cal();
    let h = Clock::default();
    h.qpc.set(1000);
    h.tsc.set(50);
    let mut t = RzTimer::new(0);
    t.start(&h);
    assert_eq!(t.start, 50, "start reads the cycle counter");
    h.tsc.set(80);
    assert_eq!(t.elapsed_u64(&h, &cal), 27, "read: cycles minus o");
    t.stop(&h, &cal);
    assert_eq!(t.accumulated, 1000 - 50 - 3, "stop reads the performance counter");
}

#[test]
fn start_on_running_drops_interval_and_wraps() {
    let cal = cal();
    let h = Clock::at(100);
    let mut t = RzTimer::new(1);
    t.start(&h);
    h.set(500);
    t.start(&h);
    h.set(510);
    t.stop(&h, &cal);
    assert_eq!(t.accumulated, 7);
    // A counter that went backwards wraps to a huge unsigned difference.
    h.set(1000);
    t.start(&h);
    h.set(999);
    assert_eq!(t.elapsed_ticks(&h, &cal), 7u64.wrapping_add(u64::MAX - 3));
    // A counter reading of 0 leaves the timer stopped.
    h.set(0);
    let mut z = RzTimer::new(1);
    z.start(&h);
    assert!(!z.is_running());
    // The 32-bit form keeps the low half for units 0-1.
    let mut big = RzTimer::new(1);
    big.accumulated = 0x1_0000_0005;
    assert_eq!(big.elapsed_u32(&h, &cal), 5);
}

#[test]
fn set_units_and_delay() {
    let cal = cal();
    let h = Clock::at(1);
    let mut t = RzTimer::new(1);
    t.start(&h);
    t.accumulated = 9;
    t.set_units(4);
    assert_eq!((t.unit, t.start, t.accumulated), (4, 0, 0));
    h.step.set(1049);
    t.delay(3, &h, &cal);
    // 1 ms = 1048.576 ticks; each sleep adds 1049 ticks minus o = 3.
    assert_eq!(h.sleeps.get(), 4);
    assert_eq!(RzTimer::timer_cycle(&h), h.qpc());
    t.delay(0, &h, &cal);
    assert_eq!(h.sleeps.get(), 4);
}

#[test]
fn factor_getters() {
    let c = Calibration::new(F, 2_000_000, 3);
    assert_eq!(c.ns_per_tick(), 1e9f32 / 1_048_576.0);
    assert_eq!(c.us_per_tick(), 1e6f32 / 1_048_576.0);
    assert_eq!(c.ms_per_tick(), 1000.0 / 1_048_576.0);
    assert_eq!(c.s_per_tick(), 1.0 / 1_048_576.0);
    assert_eq!(c.min_per_tick(), ((1.0 / 60.0) / 1_048_576.0f64) as f32);
    assert_eq!(c.ms_per_cycle(), 0.0005);
    assert_eq!(c.reciprocal(5), Some(1_048_576.0));
    assert_eq!(c.factor(1), None);
    assert_eq!(c.factor(7), None);
}

struct CalHost {
    qpc: u64,
    tsc: u64,
    qpc_step: u64,
    tsc_step: u64,
    sleeps: u32,
    prio: Vec<&'static str>,
}

impl CalibrationHost for CalHost {
    fn qpf(&mut self) -> u64 {
        1000
    }
    fn qpc(&mut self) -> u64 {
        self.qpc += self.qpc_step;
        self.qpc
    }
    fn rdtsc(&mut self) -> u64 {
        self.tsc += self.tsc_step;
        self.tsc
    }
    fn sleep_ex_100(&mut self) {
        self.sleeps += 1;
        self.qpc += 100;
        self.tsc += 300_000;
    }
    fn raise_priority(&mut self) {
        self.prio.push("raise");
    }
    fn restore_priority(&mut self) {
        self.prio.push("restore");
    }
}

#[test]
fn calibration_measure() {
    // Raw values far apart: C is measured over >= 0.33 s (4 sleeps of 0.1 s).
    let mut h = CalHost {
        qpc: 0,
        tsc: 10_000_000,
        qpc_step: 2,
        tsc_step: 5,
        sleeps: 0,
        prio: vec![],
    };
    let c = Calibration::measure(&mut h);
    assert_eq!(h.sleeps, 4);
    assert_eq!(h.prio, ["raise", "restore"]);
    assert_eq!(c.freq, 1000);
    // Δcycles = 4 × 300000 + 5 (one rdtsc), Δseconds = (4 × 102) / 1000.
    assert_eq!(c.cycle_freq, (1_200_005f64 / 0.408) as u64);
    assert_eq!((c.overhead, c.cycle_overhead), (2, 5));
    // Raw values within 2 %: C := F, no sleep.
    let mut h = CalHost {
        qpc: 1000,
        tsc: 1005,
        qpc_step: 1,
        tsc_step: 1,
        sleeps: 0,
        prio: vec![],
    };
    let c = Calibration::measure(&mut h);
    assert_eq!((h.sleeps, c.cycle_freq), (0, 1000));
}

// ---- cRZCallbackTimer ----

#[derive(Default)]
struct Service {
    calls: Mutex<Vec<(TimerKey, u32, u32, u32)>>,
    removed: Mutex<Vec<u32>>,
    next: AtomicU32,
    fail: std::sync::atomic::AtomicBool,
    /// Net AddRef - Release calls made on it.
    refs: AtomicI32,
}

impl Service {
    fn refs(&self) -> i32 {
        self.refs.load(Ordering::SeqCst)
    }
}

impl TimerService for Service {
    fn add_ref(&self) {
        self.refs.fetch_add(1, Ordering::SeqCst);
    }
    fn release(&self) {
        self.refs.fetch_sub(1, Ordering::SeqCst);
    }
    fn add_timer(&self, timer: TimerKey, period: u32, one_shot: u32, fifth: u32) -> u32 {
        self.calls.lock().unwrap().push((timer, period, one_shot, fifth));
        if self.fail.load(Ordering::SeqCst) {
            0
        } else {
            self.next.fetch_add(1, Ordering::SeqCst) + 1
        }
    }
    fn remove_timer(&self, id: u32) -> bool {
        self.removed.lock().unwrap().push(id);
        true
    }
}

/// The framework system service: owns `scheduler`, counts the lookups it
/// handed out minus the releases.
struct Framework {
    scheduler: Arc<Service>,
    refs: AtomicI32,
    lookups: AtomicU32,
}

impl FrameworkTimerService for Framework {
    fn scheduler(&self) -> Arc<dyn TimerService> {
        self.scheduler.clone()
    }
    fn release(&self) {
        self.refs.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Locator(Option<Arc<Framework>>, Mutex<Vec<(u32, u32)>>);

impl Locator {
    fn new(scheduler: Option<Arc<Service>>) -> Arc<Self> {
        let fw = scheduler.map(|scheduler| {
            Arc::new(Framework {
                scheduler,
                refs: AtomicI32::new(0),
                lookups: AtomicU32::new(0),
            })
        });
        Arc::new(Locator(fw, Mutex::new(Vec::new())))
    }
}

impl ServiceLocator for Locator {
    fn system_service(&self, clsid: u32, iid: u32) -> Option<Arc<dyn FrameworkTimerService>> {
        self.1.lock().unwrap().push((clsid, iid));
        let fw = self.0.clone()?;
        fw.refs.fetch_add(1, Ordering::SeqCst);
        fw.lookups.fetch_add(1, Ordering::SeqCst);
        Some(fw as Arc<dyn FrameworkTimerService>)
    }
}

#[derive(Default)]
struct Obj(AtomicI32);

impl Counted for Obj {
    fn add_ref(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn release(&self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn setup() -> (Arc<Service>, Arc<Service>, Arc<CallbackTimerSet>) {
    let fw = Arc::new(Service::default());
    let def = Arc::new(Service::default());
    let set = CallbackTimerSet::new(def.clone(), Some(Locator::new(Some(fw.clone()))));
    (fw, def, set)
}

fn counter() -> (Arc<AtomicU32>, CallbackFn) {
    let n = Arc::new(AtomicU32::new(0));
    let m = n.clone();
    (
        n,
        Arc::new(move |arg| {
            m.fetch_add(arg as u32, Ordering::SeqCst);
        }),
    )
}

#[test]
fn t6_callback_timer() {
    let (fw, _def, set) = setup();
    let t = CallbackTimer::new(&set);
    assert!(t.uses_timer_service());
    assert_eq!((t.delay(), t.period(), t.is_running()), (100, 1000, false));
    assert!(!t.start(0, None), "no callback");
    assert!(fw.calls.lock().unwrap().is_empty());
    let (n, cb) = counter();
    t.set_callback(Some(cb), 1);
    assert!(t.start(0, None));
    assert_eq!(fw.calls.lock().unwrap()[0], (t.key(), 1000, 0, 0));
    assert!(!t.start(0, None), "already running");
    assert!(t.stop());
    assert_eq!(*fw.removed.lock().unwrap(), [1]);
    assert_eq!((t.id(), t.is_running()), (0, false));

    let obj = Arc::new(Obj::default());
    assert!(t.start_once(250, Some(obj.clone())));
    assert_eq!(fw.calls.lock().unwrap()[1], (t.key(), 250, 1, 0));
    assert_eq!(obj.0.load(Ordering::SeqCst), 1);
    set.fire(t.key());
    assert_eq!(n.load(Ordering::SeqCst), 1);
    assert!(!t.is_running() && !t.is_one_shot() && t.id() == 0);
    assert_eq!(obj.0.load(Ordering::SeqCst), 0);
    assert_eq!(t.period(), 1000);

    // SetPeriod on a running periodic timer re-registers it.
    assert!(t.start(0, Some(obj.clone())));
    assert!(t.set_period(50));
    assert_eq!(fw.calls.lock().unwrap()[3], (t.key(), 50, 0, 0));
    assert_eq!(obj.0.load(Ordering::SeqCst), 1);
    assert!(t.is_running() && t.has_object());
    // A periodic timer keeps firing.
    set.fire(t.key());
    set.fire(t.key());
    assert_eq!(n.load(Ordering::SeqCst), 3);
    assert!(t.is_running());
    assert_eq!(obj.0.load(Ordering::SeqCst), 1);
    t.stop();

    // ... and a running one-shot timer becomes periodic.
    assert!(t.start_once(0, Some(obj.clone())));
    assert_eq!(fw.calls.lock().unwrap()[4], (t.key(), 50, 1, 0));
    assert!(t.set_period(60));
    assert_eq!(fw.calls.lock().unwrap()[5], (t.key(), 60, 0, 0));
    assert!(t.is_running() && !t.is_one_shot());
    assert_eq!(obj.0.load(Ordering::SeqCst), 1, "one reference survives");
    // SetDelay stores and restarts; the delay is never passed.
    assert!(t.set_delay(7));
    assert_eq!(fw.calls.lock().unwrap()[6], (t.key(), 60, 0, 0));
    assert_eq!(t.delay(), 7);
    drop(t);
    assert_eq!(obj.0.load(Ordering::SeqCst), 0);
    assert!(set.is_empty());
}

#[test]
fn failed_add_and_destroyed_fire() {
    let (fw, _def, set) = setup();
    let t = CallbackTimer::new(&set);
    let (n, cb) = counter();
    t.set_callback(Some(cb), 1);
    let obj = Arc::new(Obj::default());
    fw.fail.store(true, Ordering::SeqCst);
    assert!(!t.start_once(5, Some(obj.clone())));
    assert!(!t.is_running() && !t.is_one_shot() && !t.has_object());
    assert_eq!(obj.0.load(Ordering::SeqCst), 0);
    fw.fail.store(false, Ordering::SeqCst);
    assert!(t.start(0, None));
    let key = t.key();
    drop(t);
    assert_eq!(*fw.removed.lock().unwrap(), [1]);
    set.fire(key);
    assert_eq!(n.load(Ordering::SeqCst), 0, "destroyed: nothing happens");
}

#[test]
fn use_timer_service() {
    // No framework service: the flag stays false with the default.
    let def = Arc::new(Service::default());
    let set = CallbackTimerSet::new(def.clone(), Some(Locator::new(None)));
    let t = CallbackTimer::new(&set);
    assert!(!t.uses_timer_service());
    assert!(!t.use_timer_service(true));
    assert!(t.use_timer_service(false));
    let (_n, cb) = counter();
    t.set_callback(Some(cb), 0);
    assert!(t.start(0, None));
    assert_eq!(def.calls.lock().unwrap().len(), 1);
    // With one: switching off goes to the default object.
    let (fw, def, set) = setup();
    let t = CallbackTimer::new(&set);
    assert!(t.use_timer_service(true));
    assert!(t.use_timer_service(false));
    assert!(!t.uses_timer_service());
    let (_n, cb) = counter();
    t.set_callback(Some(cb), 0);
    t.start(0, None);
    assert_eq!(def.calls.lock().unwrap().len(), 1);
    assert!(fw.calls.lock().unwrap().is_empty());
    assert!(t.use_timer_service(true));
    let svc = t.timer_service();
    assert_eq!(svc.add_timer(0, 1, 0, 0), 1);
    assert_eq!(fw.calls.lock().unwrap().len(), 1);
}

#[test]
fn use_timer_service_ownership() {
    let fw = Arc::new(Service::default());
    let def = Arc::new(Service::default());
    let loc = Locator::new(Some(fw.clone()));
    let framework = loc.0.clone().unwrap();
    let set = CallbackTimerSet::new(def.clone(), Some(loc.clone()));
    // Construction: one lookup by ID/IID; the slot-3 scheduler differs from
    // the default, so the default is released and the scheduler AddRef'd;
    // the system service's own reference is given back.
    let t = CallbackTimer::new(&set);
    assert_eq!(
        *loc.1.lock().unwrap(),
        [(FRAMEWORK_TIMER_SERVICE_ID, FRAMEWORK_TIMER_SERVICE_IID)]
    );
    assert!(t.uses_timer_service() && !t.uses_default_service());
    assert_eq!((fw.refs(), def.refs()), (1, -1));
    assert_eq!(framework.refs.load(Ordering::SeqCst), 0);
    // No change: true, no lookup, no reference traffic.
    assert!(t.use_timer_service(true));
    assert_eq!(framework.lookups.load(Ordering::SeqCst), 1);
    assert_eq!((fw.refs(), def.refs()), (1, -1));
    // Quirk 8: false drops the scheduler without releasing it.
    assert!(t.use_timer_service(false));
    assert!(!t.uses_timer_service() && t.uses_default_service());
    assert_eq!((fw.refs(), def.refs()), (1, -1));
    assert!(t.use_timer_service(false), "no change");
    // Back on: a second lookup, the default released again, scheduler +1.
    assert!(t.use_timer_service(true));
    assert_eq!(framework.lookups.load(Ordering::SeqCst), 2);
    assert_eq!((fw.refs(), def.refs()), (2, -2));
    assert_eq!(framework.refs.load(Ordering::SeqCst), 0);
    // The destructor releases the scheduler once: quirk 8's reference stays.
    drop(t);
    assert_eq!((fw.refs(), def.refs()), (1, -2));
}

#[test]
fn use_timer_service_same_and_missing() {
    // The framework's scheduler is the default itself: slot 3 equals
    // `+0x20`, so no release and no AddRef, and the result is true.
    let def = Arc::new(Service::default());
    let set = CallbackTimerSet::new(def.clone(), Some(Locator::new(Some(def.clone()))));
    let t = CallbackTimer::new(&set);
    assert!(t.uses_timer_service() && t.uses_default_service());
    assert_eq!(def.refs(), 0);
    drop(t);
    assert_eq!(def.refs(), -1, "the destructor releases the default too (TM3)");

    // Not found, and no framework at all: false, flag false, the default
    // (already in `+0x20`) is not released.
    for loc in [Some(Locator::new(None) as Arc<dyn ServiceLocator>), None] {
        let def = Arc::new(Service::default());
        let set = CallbackTimerSet::new(def.clone(), loc);
        let t = CallbackTimer::new(&set);
        assert!(!t.uses_timer_service() && t.uses_default_service());
        assert!(!t.use_timer_service(true));
        assert!(!t.uses_timer_service());
        assert_eq!(def.refs(), 0);
        assert!(t.use_timer_service(false), "no change");
    }
}

#[test]
fn start_failure_and_object_refcounts() {
    let (fw, _def, set) = setup();
    let t = CallbackTimer::new(&set);
    let (n, cb) = counter();
    t.set_callback(Some(cb), 1);
    let obj = Arc::new(Obj::default());
    // Slot 5 failure: the period stays replaced, the object is released.
    fw.fail.store(true, Ordering::SeqCst);
    assert!(!t.start(77, Some(obj.clone())));
    assert_eq!(t.period(), 77);
    assert!(!t.is_running() && !t.has_object() && t.id() == 0);
    assert_eq!(obj.0.load(Ordering::SeqCst), 0);
    // Stop without an ID: no RemoveTimer, true.
    assert!(t.stop());
    assert!(fw.removed.lock().unwrap().is_empty());

    // A restart whose re-add fails leaves the kept object on a stopped timer.
    fw.fail.store(false, Ordering::SeqCst);
    assert!(t.start(0, Some(obj.clone())));
    fw.fail.store(true, Ordering::SeqCst);
    assert!(t.set_period(5));
    assert!(!t.is_running() && t.has_object());
    assert_eq!(obj.0.load(Ordering::SeqCst), 1);
    // Slot 6 with a null object keeps `+0x24`; its firing releases it.
    fw.fail.store(false, Ordering::SeqCst);
    assert!(t.start_once(0, None));
    assert_eq!(*fw.calls.lock().unwrap().last().unwrap(), (t.key(), 5, 1, 0));
    assert!(t.has_object());
    set.fire(t.key());
    assert_eq!(n.load(Ordering::SeqCst), 1);
    assert!(!t.has_object() && !t.is_running());
    assert_eq!(obj.0.load(Ordering::SeqCst), 0);

    // Slot 5 with a null object overwrites a stale `+0x24` without a release.
    assert!(t.start(0, Some(obj.clone())));
    fw.fail.store(true, Ordering::SeqCst);
    assert!(t.set_period(6));
    assert_eq!(obj.0.load(Ordering::SeqCst), 1);
    fw.fail.store(false, Ordering::SeqCst);
    assert!(t.start(0, None));
    assert!(!t.has_object());
    assert_eq!(obj.0.load(Ordering::SeqCst), 1, "the stale reference leaks");

    // The destructor of a running timer removes its ID and releases.
    let id = t.id();
    let refs = fw.refs();
    drop(t);
    assert_eq!(*fw.removed.lock().unwrap().last().unwrap(), id);
    assert_eq!(fw.refs(), refs - 1);
}
