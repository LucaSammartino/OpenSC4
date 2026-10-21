// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZTimer` (timers.md §1): a stopwatch of raw counter ticks read in one
//! of seven units, and the program-start calibration it reads (§1.4, §1.5).

use crate::x87::{ftol, mul_i64_f32};

/// The host's two counters (§1.2) *[compat-limit]*.
pub trait TimerHost {
    /// `QueryPerformanceCounter`.
    fn qpc(&self) -> u64;
    /// The CPU time-stamp counter (`cpuid` then `rdtsc`).
    fn rdtsc(&self) -> u64;
    /// `Sleep(0)` (`Delay`, §1.3).
    fn sleep0(&self) {}
}

/// The units of §1.2. `unit` is a plain `int32`: any other value is valid
/// and behaves as described for "other".
pub struct TimerUnit;

impl TimerUnit {
    pub const CYCLES: i32 = 0;
    pub const TICKS: i32 = 1;
    pub const NANOSECONDS: i32 = 2;
    pub const MICROSECONDS: i32 = 3;
    pub const MILLISECONDS: i32 = 4;
    pub const SECONDS: i32 = 5;
    pub const MINUTES: i32 = 6;
}

/// What §1.5 calibration needs from the host *[compat-limit]*.
pub trait CalibrationHost {
    /// `QueryPerformanceFrequency`.
    fn qpf(&mut self) -> u64;
    /// `QueryPerformanceCounter`.
    fn qpc(&mut self) -> u64;
    /// `cpuid` + `rdtsc`.
    fn rdtsc(&mut self) -> u64;
    /// `SleepEx(100, alertable)`.
    fn sleep_ex_100(&mut self);
    /// Real-time priority class and a time-critical thread (§1.5 step 1).
    fn raise_priority(&mut self) {}
    /// Restore both priorities.
    fn restore_priority(&mut self) {}
}

/// The calibrated statics (§1.4, §1.5): the performance-counter frequency
/// `F`, the cycle frequency `C`, the call overhead `o` and the float factors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration {
    /// `F`, ticks per second.
    pub freq: u64,
    /// `C`, cycles per second.
    pub cycle_freq: u64,
    /// `o`, the smallest back-to-back `QueryPerformanceCounter` difference.
    pub overhead: u64,
    /// The cycle overhead (stored; no `cRZTimer` method reads it, §1.5 step 4).
    pub cycle_overhead: u64,
    /// `f[2..=6]` per counter tick: ns, µs, ms, s, min.
    counter: [f32; 5],
    /// `g[2..=6] = 1 / f[unit]`.
    counter_inv: [f32; 5],
    /// The same five factors per CPU cycle.
    cycle: [f32; 5],
    cycle_inv: [f32; 5],
}

/// The five factors `1e9/x, 1e6/x, 1000/x, 1/x, (1/60)/x` and their
/// reciprocals, stored as floats (§1.5 step 3). *[choice]* Each is computed
/// in double and rounded once to float; the reciprocal is taken of the
/// stored float (the spec calls `g` "the stored reciprocal `1 / f[unit]`").
fn factors(x: u64) -> ([f32; 5], [f32; 5]) {
    let x = x as f64;
    let num = [1e9, 1e6, 1000.0, 1.0, 1.0 / 60.0];
    let mut f = [0f32; 5];
    let mut g = [0f32; 5];
    for i in 0..5 {
        f[i] = (num[i] / x) as f32;
        g[i] = (1.0 / f64::from(f[i])) as f32;
    }
    (f, g)
}

impl Calibration {
    /// Injected values (§1.5 *[choice]*: tests must inject `F`, `C`, `o`).
    pub fn new(freq: u64, cycle_freq: u64, overhead: u64) -> Self {
        let (counter, counter_inv) = factors(freq);
        let (cycle, cycle_inv) = factors(cycle_freq);
        Calibration {
            freq,
            cycle_freq,
            overhead,
            cycle_overhead: 0,
            counter,
            counter_inv,
            cycle,
            cycle_inv,
        }
    }

    /// The program-start measurement (§1.5).
    pub fn measure<H: CalibrationHost + ?Sized>(host: &mut H) -> Self {
        host.raise_priority();
        let freq = host.qpf();
        let cycle_freq = measure_cycle_freq(host, freq);
        host.restore_priority();
        // Step 4 (after the priorities are restored: the spec lists it after
        // step 1's restore).
        let mut overhead = u64::MAX;
        for _ in 0..8 {
            let a = host.qpc();
            let b = host.qpc();
            overhead = overhead.min(b.wrapping_sub(a));
        }
        let mut cycle_overhead = u64::MAX;
        for _ in 0..8 {
            let a = host.rdtsc();
            let b = host.rdtsc();
            cycle_overhead = cycle_overhead.min(b.wrapping_sub(a));
        }
        let mut cal = Calibration::new(freq, cycle_freq, overhead);
        cal.cycle_overhead = cycle_overhead;
        cal
    }

    /// `f[unit]` for units 2..=6.
    pub fn factor(&self, unit: i32) -> Option<f32> {
        (2..=6).contains(&unit).then(|| self.counter[(unit - 2) as usize])
    }

    /// `g[unit] = 1 / f[unit]` for units 2..=6.
    pub fn reciprocal(&self, unit: i32) -> Option<f32> {
        (2..=6).contains(&unit).then(|| self.counter_inv[(unit - 2) as usize])
    }

    /// The cycle factors (`1e9 / C` … `(1/60) / C`) and their reciprocals,
    /// index 0 = ns … 4 = min.
    pub fn cycle_factors(&self) -> ([f32; 5], [f32; 5]) {
        (self.cycle, self.cycle_inv)
    }

    /// §1.4: nanoseconds per counter tick.
    pub fn ns_per_tick(&self) -> f32 {
        self.counter[0]
    }
    /// §1.4: microseconds per counter tick.
    pub fn us_per_tick(&self) -> f32 {
        self.counter[1]
    }
    /// §1.4: milliseconds per counter tick.
    pub fn ms_per_tick(&self) -> f32 {
        self.counter[2]
    }
    /// §1.4: seconds per counter tick.
    pub fn s_per_tick(&self) -> f32 {
        self.counter[3]
    }
    /// §1.4: minutes per counter tick.
    pub fn min_per_tick(&self) -> f32 {
        self.counter[4]
    }
    /// §1.4: milliseconds per **CPU cycle**.
    pub fn ms_per_cycle(&self) -> f32 {
        self.cycle[2]
    }
}

/// §1.5 step 2.
fn measure_cycle_freq<H: CalibrationHost + ?Sized>(host: &mut H, freq: u64) -> u64 {
    for _ in 0..5 {
        let ticks = host.qpc();
        let cycles = host.rdtsc();
        // The ratio of the two raw values (cycles / ticks).
        let ratio = cycles as f64 / ticks as f64;
        if ratio > 0.98 && ratio < 1.02 {
            return freq;
        }
    }
    let t0 = host.qpc();
    let c0 = host.rdtsc();
    let mut t1;
    loop {
        host.sleep_ex_100();
        t1 = host.qpc();
        // *[choice]* A zero `F` cannot measure time; stop after one sleep.
        if freq == 0 || t1.wrapping_sub(t0) as f64 / freq as f64 >= 0.33 {
            break;
        }
    }
    let c1 = host.rdtsc();
    let seconds = if freq == 0 {
        0.0
    } else {
        t1.wrapping_sub(t0) as f64 / freq as f64
    };
    ftol(c1.wrapping_sub(c0) as f64 / seconds) as u64
}

/// `cRZTimer` (§1.1): 24 bytes in the original; `start == 0` means stopped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RzTimer {
    pub unit: i32,
    /// The raw counter value when started; 0 = stopped.
    pub start: u64,
    /// Accumulated raw ticks.
    pub accumulated: u64,
}

impl RzTimer {
    /// `cRZTimer(unit)`: the unit, the rest zero.
    pub fn new(unit: i32) -> Self {
        RzTimer {
            unit,
            start: 0,
            accumulated: 0,
        }
    }

    /// The unit's counter (§1.2): the cycle counter for unit 0, else
    /// `QueryPerformanceCounter`.
    fn now<H: TimerHost + ?Sized>(&self, h: &H) -> u64 {
        if self.unit == TimerUnit::CYCLES {
            h.rdtsc()
        } else {
            h.qpc()
        }
    }

    /// `interval(d)`: `d − o` when `d > o`, else 1 (§1.3). `d` is the
    /// unsigned 64-bit difference `now − start` (wrapping) *[inferred]*.
    fn interval(d: u64, cal: &Calibration) -> u64 {
        if d > cal.overhead {
            d - cal.overhead
        } else {
            1
        }
    }

    /// `SetUnits(u)`.
    pub fn set_units(&mut self, u: i32) {
        self.unit = u;
        self.start = 0;
        self.accumulated = 0;
    }

    /// `Reset()`: stopped and zero, the unit kept.
    pub fn reset(&mut self) {
        self.start = 0;
        self.accumulated = 0;
    }

    /// `IsTimerRunning()`.
    pub fn is_running(&self) -> bool {
        self.start != 0
    }

    /// `Start()`: no running test, so a running timer loses its open
    /// interval *[quirk]* (§4 quirk 4). A counter reading of 0 leaves the
    /// timer stopped (`start == 0` is "stopped").
    pub fn start<H: TimerHost + ?Sized>(&mut self, h: &H) {
        self.start = self.now(h);
    }

    /// `Stop()`: now is always `QueryPerformanceCounter`, also for unit 0
    /// *[quirk]* (§4 quirk 1).
    pub fn stop<H: TimerHost + ?Sized>(&mut self, h: &H, cal: &Calibration) {
        if self.is_running() {
            let d = h.qpc().wrapping_sub(self.start);
            self.accumulated = self.accumulated.wrapping_add(Self::interval(d, cal));
            self.start = 0;
        }
    }

    /// `Restart()`: running from now with 0, running or not.
    pub fn restart<H: TimerHost + ?Sized>(&mut self, h: &H) {
        self.start = self.now(h);
        self.accumulated = 0;
    }

    /// The raw `t` of the three `GetElapsedTime` forms. Literal reading: the
    /// open interval's "now" is the unit's counter (only `Stop` is said to
    /// read the performance counter for unit 0; question TM2).
    pub fn elapsed_ticks<H: TimerHost + ?Sized>(&self, h: &H, cal: &Calibration) -> u64 {
        if self.is_running() {
            let d = self.now(h).wrapping_sub(self.start);
            self.accumulated.wrapping_add(Self::interval(d, cal))
        } else {
            self.accumulated
        }
    }

    /// `trunc(t × f[unit])` at the main thread's 24-bit precision
    /// *[compat-limit]*. `t` enters the x87 unit as a signed 64-bit integer
    /// (`fild`) *[inferred]* (question TM2).
    fn scaled(t: u64, f: f32) -> i64 {
        ftol(f64::from(mul_i64_f32(t as i64, f)))
    }

    /// `GetElapsedTime()` → u32.
    pub fn elapsed_u32<H: TimerHost + ?Sized>(&self, h: &H, cal: &Calibration) -> u32 {
        let t = self.elapsed_ticks(h, cal);
        match self.unit {
            0 | 1 => t as u32,
            u => match cal.factor(u) {
                Some(f) => Self::scaled(t, f) as u32,
                None => 0,
            },
        }
    }

    /// `GetElapsedTime(uint64&)`: units 2–6 put the truncated product in the
    /// low half and 0 in the high half.
    pub fn elapsed_u64<H: TimerHost + ?Sized>(&self, h: &H, cal: &Calibration) -> u64 {
        let t = self.elapsed_ticks(h, cal);
        match self.unit {
            0 | 1 => t,
            u => match cal.factor(u) {
                Some(f) => u64::from(Self::scaled(t, f) as u32),
                None => 0,
            },
        }
    }

    /// `GetElapsedTime(float&)`: `t` as a float, units 2–6 multiplied by
    /// `f[unit]`; other units 0.0.
    pub fn elapsed_f32<H: TimerHost + ?Sized>(&self, h: &H, cal: &Calibration) -> f32 {
        let t = self.elapsed_ticks(h, cal);
        match self.unit {
            // `fild` then a float store: one rounding of the signed value.
            0 | 1 => (t as i64) as f32,
            u => match cal.factor(u) {
                Some(f) => mul_i64_f32(t as i64, f),
                None => 0.0,
            },
        }
    }

    /// `SetElapsedTime(uint64 v)`: `v` is loaded as a signed 64-bit integer
    /// for units 2–6; other units leave `accumulated`. Then the timer runs.
    pub fn set_elapsed_u64<H: TimerHost + ?Sized>(&mut self, v: u64, h: &H, cal: &Calibration) {
        match self.unit {
            0 | 1 => self.accumulated = v,
            u => {
                if let Some(g) = cal.reciprocal(u) {
                    self.accumulated = Self::scaled(v, g) as u64;
                }
            }
        }
        self.start = self.now(h);
    }

    /// `SetElapsedTime(float v)`.
    pub fn set_elapsed_f32<H: TimerHost + ?Sized>(&mut self, v: f32, h: &H, cal: &Calibration) {
        match self.unit {
            0 | 1 => self.accumulated = ftol(f64::from(v)) as u64,
            u => {
                if let Some(g) = cal.reciprocal(u) {
                    // A float times a float at 24-bit precision.
                    let p = crate::x87::r24(f64::from(v) * f64::from(g));
                    self.accumulated = ftol(p) as u64;
                }
            }
        }
        self.start = self.now(h);
    }

    /// `Delay(n)`: a temporary timer of this unit runs until its 32-bit
    /// `GetElapsedTime()` reaches `n`, calling `Sleep(0)` meanwhile. For an
    /// unknown unit (elapsed always 0) and `n > 0` this never returns, as in
    /// the original.
    pub fn delay<H: TimerHost + ?Sized>(&self, n: u32, h: &H, cal: &Calibration) {
        let mut tmp = RzTimer::new(self.unit);
        tmp.start(h);
        while tmp.elapsed_u32(h, cal) < n {
            h.sleep0();
        }
    }

    /// `GetTimerCycle(out)`: the performance counter.
    pub fn timer_cycle<H: TimerHost + ?Sized>(h: &H) -> u64 {
        h.qpc()
    }
}
