// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The framework timers (`docs/specs/timers.md`): the stopwatch `cRZTimer`
//! ([`RzTimer`], §1) with its calibration ([`Calibration`], §1.4, §1.5), and
//! the service-driven `cRZCallbackTimer` ([`CallbackTimer`], §2).
//!
//! Every clock is injected ([`TimerHost`], [`CalibrationHost`],
//! [`TimerService`]): the values come from the host *[compat-limit]*, only the
//! arithmetic is matched. Neither class is saved.

mod callback_timer;
mod rz_timer;

#[cfg(test)]
mod tests;

pub use callback_timer::{
    CallbackFn, CallbackTimer, CallbackTimerSet, Counted, FrameworkTimerService, ServiceLocator, TimerKey,
    TimerService, FRAMEWORK_TIMER_SERVICE_ID, FRAMEWORK_TIMER_SERVICE_IID,
};
pub use rz_timer::{Calibration, CalibrationHost, RzTimer, TimerHost, TimerUnit};
