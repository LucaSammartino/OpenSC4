// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The particle component's start/stop protocol and the emitter's
//! self-stop (`docs/specs/effects.md` §7, §7.1, re-answers34 EF1;
//! conformance test 16).
//!
//! Only the part the finish test (§7) observes is modelled: `started`, the
//! delayed-stop request, the live particles' remaining lives, and the emit
//! budget. Drawing, motion and the per-particle render data are the host's.
//! The particle description is render data the core does not read (EF6 of
//! the old pool: the render sections are located, not read), so the budget
//! and the lives are given by the caller ([`EmitterParams`]).
//!
//! The protocol (§7.1):
//! - base `Stop(mode1, _)` with `started` set: `delayed := mode1`; mode1 == 0
//!   is immediate (`started := false`, every particle destroyed); otherwise
//!   only the request is recorded and the particles play out;
//! - the component's `Stop(mode)` calls the base with `(mode == 0, 0)`, so
//!   **`Stop(0)` is the delayed stop** and `Stop(1)`/`Stop(2)` are immediate;
//! - `CheckForStop`, at the end of every draw-time update: live count 0 and a
//!   delayed stop requested → base `Stop(0, 0)` (immediate), `started` clears;
//! - the emitter calls the component's `Stop(0)` on itself when its loop or
//!   duration budget is exhausted; the plain emitter (description flag bit 2)
//!   never does.

/// Particle description flag bits that pick the emitter (§7.1).
pub mod emitter_flag {
    /// Bit 1 (`0x02`): the loop-budget emitter.
    pub const LOOP_BUDGET: u32 = 0x02;
    /// Bit 2 (`0x04`): the plain rate-integral emitter (no self-stop); wins
    /// over bit 1.
    pub const PLAIN: u32 = 0x04;
}

/// Which emitter path a model-based particle component takes (§7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterKind {
    /// `0x005B3CF0`: no self-stop.
    Plain,
    /// `0x005B8B50`: counts loops against `emit_loop_count`.
    LoopBudget,
    /// `0x005B87B0`: self-stops when its accumulated duration runs out.
    DurationBudget,
}

impl EmitterKind {
    /// The model-based particle effect's choice by the description's flags:
    /// bit 2 → plain, else bit 1 → loop budget, else duration budget.
    pub fn from_flags(flags: u32) -> EmitterKind {
        if flags & emitter_flag::PLAIN != 0 {
            EmitterKind::Plain
        } else if flags & emitter_flag::LOOP_BUDGET != 0 {
            EmitterKind::LoopBudget
        } else {
            EmitterKind::DurationBudget
        }
    }
}

/// The caller-supplied emit parameters (render data, not read by the core).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterParams {
    pub kind: EmitterKind,
    /// The description's `emit_loop_count` (`+0x18`).
    pub emit_loop_count: u32,
    /// The loop interval (the countdown `+0x190`, seeded in `Start` from the
    /// description's `+0x20..+0x2C` range; the caller resolves the range).
    pub loop_interval: f32,
    /// Particles emitted per loop [choice, EF13: the per-loop count and the
    /// moment within a loop are not specified; one burst per loop is
    /// modelled, at the moment the countdown expires].
    pub per_loop: u32,
    /// Each particle's life in effect-seconds.
    pub life: f32,
    /// The duration budget of [`EmitterKind::DurationBudget`], seconds.
    pub duration: f32,
}

/// One particle component's simulator (`cSCParticleSimulatorBase` with the
/// emitter of `cSC4ParticlesEffect` / `cSC4ModelBasedParticlesEffect`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSimulator {
    params: EmitterParams,
    /// `+0x05`.
    started: bool,
    /// `+0x0C`: a delayed stop was requested.
    delayed_stop: bool,
    /// The live particles' remaining lives (`+0x14` is their count).
    live: Vec<f32>,
    /// `+0x128`: loops emitted.
    loops: u32,
    /// `+0x190`: time to the next loop.
    countdown: f32,
    /// The duration emitter's remaining budget.
    duration_left: f32,
}

impl ParticleSimulator {
    pub fn new(params: EmitterParams) -> ParticleSimulator {
        ParticleSimulator {
            params,
            started: false,
            delayed_stop: false,
            live: Vec::new(),
            loops: 0,
            countdown: 0.0,
            duration_left: 0.0,
        }
    }

    /// `Start`: `started` set, no stop pending, the budgets reset.
    /// [choice, EF13] The first loop is due at once (countdown 0); the
    /// interval applies between loops.
    pub fn start(&mut self) {
        self.started = true;
        self.delayed_stop = false;
        self.loops = 0;
        self.countdown = 0.0;
        self.duration_left = self.params.duration;
    }

    /// The component's `Stop(mode)`: mode 0 is the delayed stop, any other
    /// mode immediate (§7.1). Nothing happens when not started.
    pub fn stop(&mut self, mode: u32) {
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

    pub fn loops_emitted(&self) -> u32 {
        self.loops
    }

    /// The draw-time update (slot 9): age the live particles (a particle
    /// dies when its remaining life reaches 0), emit (slot 15), then
    /// `CheckForStop`. [choice, EF13] Ageing precedes emission, so a burst's
    /// particles get their full life; once a delayed stop is requested the
    /// emitter emits no more.
    pub fn update(&mut self, dt: f32) {
        if !self.started {
            return;
        }
        for l in &mut self.live {
            *l -= dt;
        }
        self.live.retain(|&l| l > 0.0);
        if !self.delayed_stop {
            self.emit(dt);
        }
        self.check_for_stop();
    }

    fn emit(&mut self, dt: f32) {
        let p = self.params;
        match p.kind {
            EmitterKind::Plain => self.emit_due_loops(dt),
            EmitterKind::LoopBudget => {
                self.emit_due_loops(dt);
                if self.loops >= p.emit_loop_count {
                    self.stop(0);
                }
            }
            EmitterKind::DurationBudget => {
                self.emit_due_loops(dt);
                self.duration_left -= dt;
                if self.duration_left <= 0.0 {
                    self.stop(0);
                }
            }
        }
    }

    /// Emits every loop whose countdown expired within `dt` (the loop budget
    /// caps the count).
    fn emit_due_loops(&mut self, dt: f32) {
        let p = self.params;
        self.countdown -= dt;
        while self.countdown <= 0.0 {
            if p.kind == EmitterKind::LoopBudget && self.loops >= p.emit_loop_count {
                break;
            }
            self.loops = self.loops.saturating_add(1);
            let life = p.life;
            self.live.extend((0..p.per_loop).map(|_| life));
            if p.loop_interval <= 0.0 {
                // [choice, EF13] A zero interval emits one loop per update.
                self.countdown = 0.0;
                break;
            }
            self.countdown += p.loop_interval;
        }
    }

    /// `CheckForStop`: a requested delayed stop becomes immediate once no
    /// particle is alive.
    fn check_for_stop(&mut self) -> bool {
        if self.started && self.delayed_stop && self.live.is_empty() {
            self.base_stop(false);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(kind: EmitterKind) -> EmitterParams {
        EmitterParams {
            kind,
            emit_loop_count: 1,
            loop_interval: 1.0,
            per_loop: 4,
            life: 3.0,
            duration: 2.0,
        }
    }

    /// Steps until inactive; returns the update count (None: still active).
    fn run(p: &mut ParticleSimulator, dt: f32, max: usize) -> Option<usize> {
        for i in 1..=max {
            p.update(dt);
            if !p.is_active() {
                return Some(i);
            }
        }
        None
    }

    #[test]
    fn emitter_choice_by_flags() {
        assert_eq!(EmitterKind::from_flags(0x04), EmitterKind::Plain);
        assert_eq!(EmitterKind::from_flags(0x06), EmitterKind::Plain);
        assert_eq!(EmitterKind::from_flags(0x02), EmitterKind::LoopBudget);
        assert_eq!(EmitterKind::from_flags(0x00), EmitterKind::DurationBudget);
        assert_eq!(EmitterKind::from_flags(0x01), EmitterKind::DurationBudget);
    }

    #[test]
    fn t16_loop_budget_self_stops_after_the_last_particle() {
        // One loop of four particles (life 3 s): Stop(0) at once, inactive
        // only when the last particle dies (3 s at 0.5 s per update).
        let mut p = ParticleSimulator::new(params(EmitterKind::LoopBudget));
        p.start();
        p.update(0.5);
        assert_eq!((p.loops_emitted(), p.live_count()), (1, 4));
        assert!(p.delayed_stop_requested() && p.is_active());
        assert_eq!(run(&mut p, 0.5, 100), Some(6));
        assert_eq!((p.live_count(), p.loops_emitted()), (0, 1));
        assert!(!p.delayed_stop_requested());
    }

    #[test]
    fn t16_duration_budget_self_stops() {
        // Duration 2 s, a loop per second: loops at 0, 1 (and the budget
        // runs out at 2 s), then the last particle (emitted at 1 s) dies.
        let mut p = ParticleSimulator::new(params(EmitterKind::DurationBudget));
        p.start();
        let n = run(&mut p, 0.25, 1000).expect("self-stops");
        assert!(p.loops_emitted() >= 2);
        assert!(n > 8, "not before the duration budget, got {n}");
    }

    #[test]
    fn plain_emitter_never_self_stops() {
        let mut p = ParticleSimulator::new(params(EmitterKind::Plain));
        p.start();
        assert_eq!(run(&mut p, 0.5, 1000), None);
        // A delayed Stop(0) then plays the particles out.
        p.stop(0);
        assert!(p.is_active() && p.live_count() > 0);
        assert!(run(&mut p, 0.5, 100).is_some());
    }

    #[test]
    fn hard_stop_is_immediate_and_stop_needs_started() {
        let mut p = ParticleSimulator::new(params(EmitterKind::Plain));
        p.stop(0);
        assert!(!p.is_active() && !p.delayed_stop_requested());
        p.start();
        p.update(0.5);
        assert!(p.live_count() > 0);
        p.stop(1);
        assert!(!p.is_active());
        assert_eq!(p.live_count(), 0);
        p.start();
        p.update(0.5);
        p.stop(2);
        assert!(!p.is_active());
    }

    #[test]
    fn volcano_particle_ends_within_its_life() {
        // §7.1: emit_loop_count 1, life 3-20 s: ends on its own, well before
        // the crater decal's 50 s.
        for life in [3.0f32, 20.0] {
            let mut p = ParticleSimulator::new(EmitterParams {
                life,
                ..params(EmitterKind::LoopBudget)
            });
            p.start();
            let n = run(&mut p, 1.0 / 30.0, 10_000).expect("ends") as f32 / 30.0;
            assert!(n <= life + 0.1 && n < 50.0, "{life}: {n}");
        }
    }
}
