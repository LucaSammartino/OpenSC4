// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Dependencies injected into the simulator.
//!
//! All methods take `&self`: the simulator may be re-entered from inside
//! [`AgentDelivery::deliver`] (an agent may add or remove agents, pause, ...),
//! so neither side may hold an exclusive borrow across a call. Implementations
//! that need mutable state use interior mutability.

use crate::simulator::Simulator;

/// Opaque agent identity chosen by the host (in the GZCOM shim it is the
/// `cIGZMessageTarget2*` value). Two registrations are the same target iff
/// their handles are equal.
pub type AgentHandle = u64;

/// A scheduler or broadcast message: the `cIGZMessage2Standard` fields the
/// spec uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Message {
    pub msg_type: u32,
    pub data1: i64,
    pub data2: i64,
    pub data3: i64,
}

impl Message {
    pub const fn new(msg_type: u32, data1: i64, data2: i64, data3: i64) -> Self {
        Message {
            msg_type,
            data1,
            data2,
            data3,
        }
    }
}

/// Millisecond clock. Only differences between readings are used, and they
/// are computed with wrapping arithmetic.
pub trait Clock {
    fn now_ms(&self) -> u32;
}

/// Synchronous agent delivery plus reference-count hooks (spec §4).
pub trait AgentDelivery {
    /// Deliver `msg` to `agent` and return its "finished" flag.
    /// `sim` is the simulator doing the delivery; the agent may call back into it.
    fn deliver(&self, sim: &Simulator, agent: AgentHandle, msg: &Message) -> bool;
    /// Called once when a registration is added.
    fn add_ref(&self, agent: AgentHandle);
    /// Called once when a registration is removed.
    fn release(&self, agent: AgentHandle);
}

/// Broadcast (queued) message posting (spec §5.2 "post").
pub trait Broadcast {
    fn post(&self, msg: &Message);
}

/// Sources and sink for the CPU-cycle limit-timer calibration (spec §5.5).
/// Optional: a simulator without one simply does not calibrate.
pub trait TimerSource {
    /// High-resolution wall clock, in ticks (`QueryPerformanceCounter`).
    fn qpc(&self) -> u64;
    /// Wall-clock ticks per second (`QueryPerformanceFrequency`); must be > 0.
    fn qpc_frequency(&self) -> u64;
    /// Processor time-stamp counter, in cycles.
    fn tsc(&self) -> u64;
    /// Publish `CyclesToMillisecs` (ms per cycle) and `MillisecsToCycles` (cycles per ms).
    fn publish(&self, cycles_to_ms: f32, ms_to_cycles: f32);
}

/// Everything the simulator needs from its environment.
pub trait Host: Clock + AgentDelivery + Broadcast {}
impl<T: Clock + AgentDelivery + Broadcast + ?Sized> Host for T {}
