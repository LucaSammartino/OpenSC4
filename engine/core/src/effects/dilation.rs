// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The animation time dilation the effects tick scales by (`docs/specs/effects.md`
//! §6, re-answers34 EF5; conformance test 17).
//!
//! The simulator keeps a five-float table of tunables; the getter
//! (`GetAnimationTimeDilation`, simulator slot 32) picks one entry from the
//! simulator's state, in this order: emergency pause → `[4]`; U-Drive-It
//! driving → `[4]`; paused → `[0]`; otherwise by the sim speed (0 → `[1]`,
//! 1 → `[2]`, 2 → `[3]`). A hidden pause is never consulted.
//!
//! An [`EffectsHost`](super::EffectsHost) answers
//! [`simulator`](super::EffectsHost::simulator) with
//! [`animation_time_dilation`] of its simulator's state.

use crate::x87::r24;

/// The shipped tunables: `[0] = 0.0` (paused), `[1] = 1.0` (speed 0),
/// `[2] = 1.5` (speed 1), `[3] = 2.0` (speed 2), `[4] = 1.0` (emergency
/// pause, driving).
pub const SHIPPED_DILATION: [f32; 5] = [0.0, 1.0, 1.5, 2.0, 1.0];

/// The simulator state the getter reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimTimeState {
    /// `IsEmergencyPaused` (simulator slot 17).
    pub emergency_paused: bool,
    /// The automata manager's `IsDrivingMode` (U-Drive-It view up).
    pub driving: bool,
    /// `IsPaused` (simulator slot 15).
    pub paused: bool,
    /// `IsHiddenPaused` (simulator slot 16): carried for completeness, never
    /// consulted (EF5).
    pub hidden_paused: bool,
    /// `GetSimSpeed`.
    pub speed: u32,
}

/// `GetAnimationTimeDilation` with the given tunables table.
///
/// [choice, EF11] A sim speed above 2 is not covered by the spec's table; it
/// is read as speed 0 (`[1]`), the getter's unscaled value.
pub fn animation_time_dilation_with(table: &[f32; 5], s: &SimTimeState) -> f32 {
    if s.emergency_paused || s.driving {
        return table[4];
    }
    if s.paused {
        return table[0];
    }
    match s.speed {
        1 => table[2],
        2 => table[3],
        _ => table[1],
    }
}

/// `GetAnimationTimeDilation` with the shipped tunables.
pub fn animation_time_dilation(s: &SimTimeState) -> f32 {
    animation_time_dilation_with(&SHIPPED_DILATION, s)
}

/// The tick's dilated step `dt' := trunc(r24(µs × dilation))` (§6), `µs`
/// unsigned; `None` (no simulator) leaves `µs` as is. A non-positive or NaN
/// product is 0 (the component updates then run with `dt = 0`, §6 quirk).
pub fn dilated_us(us: u32, dilation: Option<f32>) -> u64 {
    match dilation {
        Some(d) => {
            let v = r24(f64::from(us) * f64::from(d));
            if v.is_nan() || v <= 0.0 {
                0
            } else {
                (v.trunc() as u64).min(u64::from(u32::MAX))
            }
        }
        None => u64::from(us),
    }
}
