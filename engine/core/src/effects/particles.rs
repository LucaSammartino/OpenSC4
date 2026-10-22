// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The particle component's start/stop protocol and its emitters
//! (`docs/specs/effects.md` §7.1, §7.1.1; EF1, EF13; conformance test 16).
//!
//! Only what the finish test (§7) observes is modelled: `started`, the
//! delayed-stop request, the live particles and the emitter state (phase,
//! loop counter, carry, countdown). Drawing, motion and the per-particle
//! render data are outside this model. EF14 supplies the slot-20 age test
//! and the first init draw (life); motion, terrain kills and other init
//! draws/birth hooks are not modelled. Native owner outputs are not inputs.
//!
//! The protocol (§7.1):
//! - base `Stop(mode1, _)` with `started` set: `delayed := mode1`; mode1 == 0
//!   is immediate (`started := false`, every particle destroyed); otherwise
//!   only the request is recorded and the particles play out;
//! - the component's `Stop(mode)` calls the base with `(mode == 0, 0)`, so
//!   **`Stop(0)` is the delayed stop** and `Stop(1)`/`Stop(2)` are immediate;
//! - `CheckForStop`, at the end of every draw-time update: live count 0 and a
//!   delayed stop requested → base `Stop(0, 0)` (immediate), `started` clears.
//!
//! The emitters (§7.1.1): the loop-budget emitter (flag A `0x02`) makes one
//! burst of `ceil(curve[0])` per loop; the rate emitter (neither `0x02` nor
//! `0x04`) integrates the count curve in sub-steps of at most 1/15 s; both
//! count loops of `D+0x14` seconds in `n` against `D+0x18` and call `Stop(0)`
//! when it is reached. Random draws come from the global `cRZRandom`
//! (`0x00B22DF8`), passed in by the caller.

use crate::rng::RzRandom;

/// Particle description flag A bits (§7.1, §7.1.1).
pub mod emitter_flag {
    /// `0x02` (`inject`): the loop-budget emitter.
    pub const LOOP_BUDGET: u32 = 0x02;
    /// `0x04` (`maintain`): the plain rate-integral emitter (no self-stop);
    /// wins over `0x02`.
    pub const PLAIN: u32 = 0x04;
    /// `0x08`: a finite emitter may be restarted by `Start(arg != 0)`, and
    /// the rate emitter holds at its last loop instead of stopping.
    pub const HOLD: u32 = 0x08;
    /// Bits 19 / 20 / 21: the rate scale `+0xC0` once, squared, cubed.
    pub const SCALE_1: u32 = 1 << 19;
    pub const SCALE_2: u32 = 1 << 20;
    pub const SCALE_3: u32 = 1 << 21;
}

/// The rate emitter's largest sub-step (`0x00AA0CF4`).
pub const RATE_SUBSTEP: f32 = 1.0 / 15.0;

/// Which emitter the dispatch picks (§7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterKind {
    /// `0x005B3CF0`: no self-stop.
    Plain,
    /// `0x005B8B50`: one burst per loop, counts loops against `D+0x18`.
    LoopBudget,
    /// `0x005B87B0`: the rate emitter, counts loops against `D+0x18`.
    Rate,
}

impl EmitterKind {
    /// By flag A: `0x04` → plain, else `0x02` → loop budget, else rate.
    pub fn from_flags(flags: u32) -> EmitterKind {
        if flags & emitter_flag::PLAIN != 0 {
            EmitterKind::Plain
        } else if flags & emitter_flag::LOOP_BUDGET != 0 {
            EmitterKind::LoopBudget
        } else {
            EmitterKind::Rate
        }
    }
}

/// The particle description fields the emitters read (§7.1.1), plus the
/// detail scales. Life is sampled by particle init, not supplied by an owner.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterDesc {
    /// Flags A (`D+0x00`).
    pub flags_a: u32,
    /// `D+0x08`; bit 0 set: the burst is not scaled.
    pub flags_b: u32,
    /// Lower particle-life bound (`D+0x0C`), seconds.
    pub life_min: f32,
    /// Upper particle-life bound and max burst age (`D+0x10`), seconds.
    pub max_age: f32,
    /// Period (`D+0x14`), seconds per loop.
    pub period: f32,
    /// Loop count (`D+0x18`), 0 = unlimited.
    pub loop_count: u32,
    /// Pre-roll (`D+0x1C`), seconds.
    pub preroll: f32,
    /// Start-delay range (`D+0x20..+0x24`).
    pub start_delay: (f32, f32),
    /// Interval range (`D+0x28..+0x2C`).
    pub interval: (f32, f32),
    /// Count curve (`D+0x88..+0x8C`).
    pub curve: Vec<f32>,
    /// The detail/density scales `*(+0x1E8)`, `+0x1E0`, `+0x1C4`.
    pub scales: [f32; 3],
    /// The rate scale `+0xC0`.
    pub rate_scale: f32,
}

impl EmitterDesc {
    /// A description with no scaling, no delays (both ranges negative), no
    /// pre-roll, period 1 s, one loop, a one-entry curve, and a degenerate
    /// life range `[life, life]` (init still consumes a random draw).
    pub fn new(flags_a: u32, count: f32, life: f32) -> EmitterDesc {
        EmitterDesc {
            flags_a,
            flags_b: 0,
            life_min: life,
            max_age: life,
            period: 1.0,
            loop_count: 1,
            preroll: 0.0,
            start_delay: (-1.0, -1.0),
            interval: (-1.0, -1.0),
            curve: vec![count],
            scales: [1.0; 3],
            rate_scale: 1.0,
        }
    }

    fn kind(&self) -> EmitterKind {
        EmitterKind::from_flags(self.flags_a)
    }

    /// `m`: the product of the three detail scales.
    fn m(&self) -> f32 {
        self.scales[0] * self.scales[1] * self.scales[2]
    }

    /// `m'`: `m` times `+0xC0` once, squared or cubed for bits 19/20/21.
    /// EF14: with several bits set the lowest wins; with none the
    /// rate scale is not applied.
    fn m_rate(&self) -> f32 {
        let k = self.rate_scale;
        let f = if self.flags_a & emitter_flag::SCALE_1 != 0 {
            k
        } else if self.flags_a & emitter_flag::SCALE_2 != 0 {
            k * k
        } else if self.flags_a & emitter_flag::SCALE_3 != 0 {
            k * k * k
        } else {
            1.0
        };
        self.m() * f
    }

    /// The rate at loop fraction `frac`: one entry → `curve[0]`, else linear
    /// interpolation at `(len − 1)·frac`. EF14's documented safe choices:
    /// an empty curve is 0; clamp out-of-range fractions (native UB).
    fn rate_at(&self, frac: f64) -> f32 {
        match self.curve.len() {
            0 => 0.0,
            1 => self.curve[0],
            len => {
                let pos = (len - 1) as f64 * frac.clamp(0.0, 1.0);
                let i = pos.floor() as usize;
                let t = pos - i as f64;
                if t > 0.0 {
                    (f64::from(self.curve[i]) + t * (f64::from(self.curve[i + 1]) - f64::from(self.curve[i]))) as f32
                } else {
                    self.curve[i]
                }
            }
        }
    }
}

/// The stored float age and life used by main slot 20 (EF14).
#[derive(Clone, Debug, PartialEq)]
struct Particle {
    age: f32,
    life: f32,
}

impl Particle {
    fn update(&mut self, t: f32) -> bool {
        self.age += t;
        self.age < self.life
    }
}

/// One particle component (`cSCParticleSimulatorBase` with the emitter of
/// `cSC4ParticlesEffect` / `cSC4ModelBasedParticlesEffect`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSimulator {
    desc: EmitterDesc,
    /// `+0x05`.
    started: bool,
    /// `+0x0C`: a delayed stop was requested.
    delayed_stop: bool,
    /// The live particles (`+0x14` is their count).
    live: Vec<Particle>,
    /// `P` (`+0x120`), in loops.
    phase: f64,
    /// `n` (`+0x128`).
    loops: u32,
    /// `ρ` (`+0x130`), loops per second.
    rho: f64,
    /// `c` (`+0x138`).
    carry: f32,
    /// `w` (`+0x190`), seconds.
    countdown: f32,
    /// `dt` (`+0x108`).
    dt: f32,
    /// Total age (`+0x110`).
    age: f32,
    /// Pre-roll pending (`+0x118`).
    preroll_pending: bool,
    /// Emission suppressed (`+0x21A`).
    suppressed: bool,
    /// `+0x188`: the last `Start` argument.
    start_arg: u32,
    /// `+0x18C`: the last component `Stop` mode.
    stop_mode: u32,
    /// Particles created over the component's life (diagnostics).
    created: u64,
}

impl ParticleSimulator {
    /// A component with its description set (`SetDescription`).
    pub fn new(desc: EmitterDesc) -> ParticleSimulator {
        let mut p = ParticleSimulator {
            desc: EmitterDesc::new(0, 0.0, 0.0),
            started: false,
            delayed_stop: false,
            live: Vec::new(),
            phase: 0.0,
            loops: 0,
            rho: 1.0,
            carry: 0.0,
            countdown: 0.0,
            dt: 0.0,
            age: 0.0,
            preroll_pending: false,
            suppressed: false,
            start_arg: 0,
            stop_mode: 0,
            created: 0,
        };
        p.set_description(desc);
        p
    }

    /// `SetDescription` `0x005B7C90`: `ρ := 1 / D+0x14`, the period replaced
    /// by the local float `1e-6` unless greater; clears `+0x21A` and resets
    /// scale `+0x1E0`. EF14: the replacement is used for `ρ` only; burst
    /// age and gaps still use the description's raw period.
    pub fn set_description(&mut self, mut desc: EmitterDesc) {
        let period = if desc.period > 1e-6 { desc.period } else { 1e-6 };
        self.rho = 1.0 / f64::from(period);
        self.suppressed = false;
        desc.scales[1] = 1.0;
        self.desc = desc;
    }

    /// `+0x21A`: while set the emitter dispatch is skipped.
    pub fn set_suppressed(&mut self, on: bool) {
        self.suppressed = on;
    }

    /// `Start(arg)` `0x005B3000`. A finite emitter (`D+0x18 > 0`, flag A
    /// `0x08` clear) is not restarted by `Start(arg != 0)`. Otherwise
    /// `started` is set (no stop pending), `P`, `n`, `c` and the age reset,
    /// the countdown seeded from the start-delay range if `D+0x20 ≥ 0`, else
    /// the interval range if `D+0x28 ≥ 0`, else 0 (`rng` is the global
    /// `cRZRandom`), and the pre-roll is pending only for `arg != 0`.
    pub fn start(&mut self, arg: u32, rng: &mut RzRandom) {
        let d = &self.desc;
        if arg != 0 && d.loop_count > 0 && d.flags_a & emitter_flag::HOLD == 0 {
            return;
        }
        self.started = true;
        self.delayed_stop = false;
        self.phase = 0.0;
        self.loops = 0;
        self.carry = 0.0;
        self.age = 0.0;
        let (sd, iv) = (d.start_delay, d.interval);
        self.countdown = if sd.0 >= 0.0 {
            rng.range_f64(f64::from(sd.0), f64::from(sd.1)) as f32
        } else if iv.0 >= 0.0 {
            rng.range_f64(f64::from(iv.0), f64::from(iv.1)) as f32
        } else {
            0.0
        };
        self.start_arg = arg;
        self.stop_mode = 0;
        self.preroll_pending = arg != 0;
    }

    /// The component's `Stop(mode)` `0x005B21E0`: stores the mode and calls
    /// the base with `(mode == 0, 0)` — mode 0 is the delayed stop.
    pub fn stop(&mut self, mode: u32) {
        self.stop_mode = mode;
        self.base_stop(mode == 0);
    }

    /// The base `Stop(mode1, 0)`.
    fn base_stop(&mut self, mode1: bool) {
        if !self.started {
            return;
        }
        self.delayed_stop = mode1;
        if !mode1 {
            self.started = false;
            self.live.clear();
        }
    }

    /// `IsActive` of the component (§7 table): the `started` byte.
    pub fn is_active(&self) -> bool {
        self.started
    }

    pub fn delayed_stop_requested(&self) -> bool {
        self.delayed_stop
    }

    /// `+0x14`.
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// Diagnostic remaining lives, in creation order, computed from the
    /// stored age and life (updates accumulate age, not remaining life).
    pub fn live(&self) -> Vec<f32> {
        self.live.iter().map(|p| p.life - p.age).collect()
    }

    /// `n` (`+0x128`).
    pub fn loops_emitted(&self) -> u32 {
        self.loops
    }

    /// `P` (`+0x120`).
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// `w` (`+0x190`).
    pub fn countdown(&self) -> f32 {
        self.countdown
    }

    /// `c` (`+0x138`).
    pub fn carry(&self) -> f32 {
        self.carry
    }

    /// Total age (`+0x110`).
    pub fn age(&self) -> f32 {
        self.age
    }

    pub fn particles_created(&self) -> u64 {
        self.created
    }

    /// The draw-time update `0x005B8540` (only while started): the step
    /// (plus the pending pre-roll once) is stored and added to the age;
    /// every live particle is aged first, then the emitter dispatch runs
    /// unless suppressed, then `CheckForStop`.
    pub fn update(&mut self, dt: f32, rng: &mut RzRandom) {
        if !self.started {
            return;
        }
        let mut t = dt;
        if self.preroll_pending {
            t += self.desc.preroll;
            self.preroll_pending = false;
        }
        self.dt = t;
        self.age += t;
        self.live.retain_mut(|p| p.update(t));
        if !self.suppressed {
            self.dispatch(rng);
        }
        self.check_for_stop();
    }

    /// Init always sets alive and age 0 and draws life first, even for a
    /// degenerate range or a particle rejected by the later slot-20 test.
    /// Plain emitters skip slot 20 entirely (`initial_age == None`).
    fn create(&mut self, initial_age: Option<f32>, rng: &mut RzRandom) {
        self.created += 1;
        let mut p = Particle {
            age: 0.0,
            life: rng.range_f64(f64::from(self.desc.life_min), f64::from(self.desc.max_age)) as f32,
        };
        let alive = match initial_age {
            Some(a) => p.update(a),
            None => true,
        };
        if alive {
            self.live.push(p);
        }
    }

    /// The dispatch `0x005B96E0`.
    fn dispatch(&mut self, rng: &mut RzRandom) {
        if self.delayed_stop {
            return;
        }
        let mut e = if self.countdown > 0.0 {
            self.countdown -= self.dt;
            if self.countdown > 0.0 {
                return;
            }
            -self.countdown
        } else {
            self.dt
        };
        let pre = f64::from(self.desc.preroll);
        if pre > 0.0 && f64::from(e) > pre - 0.01 {
            e = (pre - 0.01) as f32;
        }
        match self.desc.kind() {
            EmitterKind::Plain => self.emit_plain(e, rng),
            EmitterKind::LoopBudget => self.emit_loop_budget(e, rng),
            EmitterKind::Rate => self.emit_rate(e, rng),
        }
    }

    /// The loop-budget emitter `0x005B8B50`: one burst per loop.
    fn emit_loop_budget(&mut self, e: f32, rng: &mut RzRandom) {
        self.phase = f64::from(self.phase as f32 + e * self.rho as f32);
        if self.phase <= f64::from(self.loops) {
            return;
        }
        let d = &self.desc;
        let mut x = d.curve.first().copied().unwrap_or(0.0);
        if x > 20.0 && d.flags_b & 1 == 0 {
            x *= d.m();
        }
        // [choice] A negative count creates nothing.
        let burst = x.ceil().max(0.0) as u32;
        let (period, max_age, loop_count, iv) = (d.period, d.max_age, d.loop_count, d.interval);
        while self.phase > f64::from(self.loops) {
            let a = ((self.phase - f64::from(self.loops)) * f64::from(period)) as f32;
            if a < max_age && burst != 0 {
                for _ in 0..burst {
                    self.create(Some(a), rng);
                }
            }
            self.loops += 1;
            if loop_count != 0 && self.loops >= loop_count {
                self.stop(0);
                return;
            }
            if iv.0 >= 0.0 {
                let gap = rng.range_f64(f64::from(iv.0), f64::from(iv.1));
                self.phase -= gap * self.rho;
                if self.phase < f64::from(self.loops) {
                    self.countdown = ((f64::from(self.loops) - self.phase) * f64::from(period)) as f32;
                    self.phase = f64::from(self.loops);
                    return;
                }
            }
        }
    }

    /// The rate emitter `0x005B87B0`: sub-steps of at most 1/15 s.
    fn emit_rate(&mut self, mut e: f32, rng: &mut RzRandom) {
        let (period, loop_count, iv, hold) = (
            self.desc.period,
            self.desc.loop_count,
            self.desc.interval,
            self.desc.flags_a & emitter_flag::HOLD != 0,
        );
        let m = self.desc.m_rate();
        while e > 0.0 {
            let mut s = e.min(RATE_SUBSTEP);
            self.phase += f64::from(s) * self.rho;
            let mut gap = false;
            if self.phase > f64::from(self.loops) + 1.0 {
                self.loops += 1;
                if loop_count == 0 || self.loops != loop_count {
                    if iv.0 >= 0.0 {
                        s -= ((self.phase - f64::from(self.loops)) * f64::from(period)) as f32;
                        self.phase = f64::from(self.loops);
                        gap = true;
                    }
                } else if !hold {
                    self.stop(0);
                    return;
                } else {
                    self.loops = loop_count - 1;
                    self.phase = f64::from(loop_count) - 1e-6;
                }
            }
            let r = self.desc.rate_at(self.phase - f64::from(self.loops));
            let x = r * s * m + self.carry;
            let n = x.floor();
            self.carry = x - n;
            let n = n.max(0.0) as u32;
            for k in 0..n {
                self.create(Some(e - k as f32 * s / n as f32), rng);
            }
            e -= s;
            if gap {
                e -= rng.range_f64(f64::from(iv.0), f64::from(iv.1)) as f32;
                if e < 0.0 {
                    self.countdown = -e;
                    return;
                }
            }
        }
    }

    /// EF14(a): top up to ceil(curve[0]), with no scaling, wrapping,
    /// counter/carry/countdown updates, slot 20 or birth hook.
    fn emit_plain(&mut self, e: f32, rng: &mut RzRandom) {
        self.phase += f64::from(e) * self.rho;
        if self.phase <= f64::from(self.loops) {
            return;
        }
        // [choice, EF14] Empty curve -> 0; T <= live -> no creations,
        // avoiding the native unsigned-underflow loop (~2^32 attempts).
        let target = self.desc.curve.first().copied().unwrap_or(0.0).ceil().max(0.0) as u32;
        for _ in self.live.len() as u64..u64::from(target) {
            self.create(None, rng);
        }
    }

    /// `CheckForStop` `0x005B9D40`: a requested delayed stop becomes
    /// immediate once no particle is alive.
    fn check_for_stop(&mut self) -> bool {
        if self.started && self.delayed_stop && self.live.is_empty() {
            self.base_stop(false);
            return true;
        }
        false
    }
}

#[cfg(test)]
#[path = "particles_tests.rs"]
mod tests;
