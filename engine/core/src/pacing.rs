// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Pacing: how idle-driven work is bound to game days (simulator.md §5.7).
//!
//! The original paces three kinds of work by frames and milliseconds, so a
//! city evolves differently at different days per real second (§5.7 items
//! 1-4). FreeCity4's *[choice]* (§5.7, last paragraph) is
//! [`Pacing::Deterministic`]:
//!
//! - every kind-1 and kind-2 cycle (growth developers, aura, plumbing, the
//!   demand census, land value, crime, power, the BDS rebuild) runs to
//!   completion within the day it starts, at the step where the original at
//!   normal speed would have finished it: the day's single **settle** idle
//!   delivery (the simulator's [`crate::simulator::PacingHooks`]), after the
//!   day's agents;
//! - millisecond budgets (the adaptive helper of §5.5, the lot developer's
//!   5 ms, the BDS's 3 ms) are item counts, never a clock;
//! - posted day / month messages and lot `StateChanged` are delivered at that
//!   settle point, before the next day's agents (the plugin's job).
//!
//! [`Pacing::Original`] keeps the clock-bound behaviour (one step per
//! delivery, the host's limit timers): the conformance shadows compare our
//! cores with the original's under it.
//!
//! The mode is a process-wide setting ([`process`] / [`set_process`]; the
//! plugin sets it once at game start from `pacing = …`). Every core that has a
//! cycle captures it when it is constructed and keeps it per object
//! (`set_pacing` on each core, for tests and for shadows, which always run
//! [`Pacing::Original`]). The default is [`Pacing::Original`], so a core built
//! without the plugin (tests, vectors) behaves as before.

use std::sync::atomic::{AtomicU8, Ordering};

/// How idle-driven cycles are paced (module documentation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Pacing {
    /// The original's frame- and clock-bound pacing (simulator.md §5.7
    /// items 1-4): conformance shadows.
    #[default]
    Original = 0,
    /// simulator.md §5.7 *[choice]*: cycles complete within their day, item
    /// counts instead of milliseconds, posted messages at the day's settle.
    Deterministic = 1,
}

impl Pacing {
    /// The C ABI value (`FC4_PACING_*`): 0 original, 1 deterministic.
    pub fn from_u32(v: u32) -> Option<Pacing> {
        match v {
            0 => Some(Pacing::Original),
            1 => Some(Pacing::Deterministic),
            _ => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn is_deterministic(self) -> bool {
        self == Pacing::Deterministic
    }

    /// The `pacing = …` word.
    pub fn name(self) -> &'static str {
        match self {
            Pacing::Original => "original",
            Pacing::Deterministic => "deterministic",
        }
    }
}

static PROCESS: AtomicU8 = AtomicU8::new(Pacing::Original as u8);

/// The process-wide mode that new cores capture (default
/// [`Pacing::Original`]).
pub fn process() -> Pacing {
    match PROCESS.load(Ordering::Relaxed) {
        1 => Pacing::Deterministic,
        _ => Pacing::Original,
    }
}

/// Sets the process-wide mode. Cores constructed afterwards capture it;
/// existing ones keep theirs.
pub fn set_process(p: Pacing) {
    PROCESS.store(p as u8, Ordering::Relaxed);
}

/// [choice] The most steps one settle delivery runs ([`run_to_idle`]); a
/// cycle that is still not idle afterwards continues at the next settle.
/// Far above any shipped cycle (the longest, a growth developer's, is 20 + E
/// states with each time-sliced state finishing in one step).
pub const MAX_SETTLE_STEPS: u32 = 1 << 16;

/// Runs `step` until `idle()` holds, at most [`MAX_SETTLE_STEPS`] times.
/// Returns `idle()` at the end: false = the cycle is blocked (for example
/// the BDS waiting for a developer) or the cap was reached.
///
/// `step` returns whether it made progress; a step that made none (the
/// cycle waits for something outside it) ends the loop.
pub fn run_to_idle(mut step: impl FnMut() -> bool, mut idle: impl FnMut() -> bool) -> bool {
    let mut n = 0u32;
    while !idle() {
        if n >= MAX_SETTLE_STEPS {
            return false;
        }
        n += 1;
        if !step() {
            // No progress: the cycle waits for something outside it.
            return idle();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn pacing_values_round_trip() {
        assert_eq!(Pacing::from_u32(0), Some(Pacing::Original));
        assert_eq!(Pacing::from_u32(1), Some(Pacing::Deterministic));
        assert_eq!(Pacing::from_u32(2), None);
        assert_eq!(Pacing::Deterministic.as_u32(), 1);
        assert_eq!(Pacing::default(), Pacing::Original);
        assert_eq!(Pacing::Deterministic.name(), "deterministic");
        assert!(!Pacing::Original.is_deterministic());
    }

    #[test]
    fn run_to_idle_steps_until_idle() {
        let state = Cell::new(5u32);
        let steps = Cell::new(0u32);
        let idle = run_to_idle(
            || {
                steps.set(steps.get() + 1);
                state.set(state.get() - 1);
                true
            },
            || state.get() == 0,
        );
        assert!(idle);
        assert_eq!(steps.get(), 5);
        // Already idle: no step.
        assert!(run_to_idle(|| panic!("no step when idle"), || true));
    }

    #[test]
    fn run_to_idle_stops_without_progress() {
        let steps = Cell::new(0u32);
        let idle = run_to_idle(
            || {
                steps.set(steps.get() + 1);
                false
            },
            || false,
        );
        assert!(!idle);
        assert_eq!(steps.get(), 1);
    }

    #[test]
    fn run_to_idle_is_capped() {
        let steps = Cell::new(0u32);
        let idle = run_to_idle(
            || {
                steps.set(steps.get() + 1);
                true
            },
            || false,
        );
        assert!(!idle);
        assert_eq!(steps.get(), MAX_SETTLE_STEPS);
    }
}
