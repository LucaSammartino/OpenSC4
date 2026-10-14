// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic shadow (tooling, no spec behaviour). Work in progress: only
//! the trip-phase progress measure the shadow aligns on is here so far; the
//! shadow host, the captured-record reader and the driver are still to come
//! (see the lane report).

use super::cycle_state as cs;
use super::state::{State, PC_CAPACITY_SEND_BACKS, PC_SOURCE_COUNTER};

/// The units a trip phase has done since its pacing reset (§3.4): state 16
/// the sources confirmed (pass counter 3), state 17 the searches
/// (`processed`) plus the capacity send-backs (pass counter 4), state 18
/// the searches (`processed`); 0 in any other state.
pub fn progress(st: &State) -> u64 {
    match st.cycle_state {
        cs::CONFIRM_TRIPS => u64::from(st.pass_counters[PC_SOURCE_COUNTER]),
        cs::REROUTE_TRIPS => u64::from(st.processed as u32) + u64::from(st.pass_counters[PC_CAPACITY_SEND_BACKS]),
        cs::FULL_TRIPS => u64::from(st.processed as u32),
        _ => 0,
    }
}
