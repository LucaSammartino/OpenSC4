// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The 24-hour clock (`docs/specs/clock24.md`, `cSC424HourClock`, class
//! `0xA7E7F929`): the city's time of day, `0 … 86399` game seconds advanced
//! from a real-time timer times a magnification on the framework's idle loop,
//! and the visual time that lighting follows. Section references (`§n`)
//! point at that spec.
//!
//! # Structure
//!
//! - [`Clock24`]: the state of §2, the messages of §3, the behaviour of §4
//!   (the idle step, the visual time, modes, setters, pause counters and the
//!   cheats), the tuning of §6, the record of §7 and the interface methods of
//!   §8.
//! - [`ClockHost`]: the real-time clock, message posting, the framework's
//!   system-service and on-idle lists, the cheat code manager and the "Main
//!   Simulator" exemplar.
//! - [`Timer`]: the `cRZTimer` model of §4.3.
//!
//! # Precision (foundations §18)
//!
//! The clock's float formulas are `float` operations under the session's
//! 24-bit x87 precision, which `f32` arithmetic reproduces:
//!
//! - the step (§4.3): `f32(e × magnification)` is [`mul_u32_f32`] (an
//!   unsigned `e` below 2³¹ enters exactly, `x87::mul_i64_f32`, one
//!   rounding; from 2³¹ up the `float(uint32)` idiom, `x87::u32_to_float`,
//!   then one rounding), the product with `9.99999997e-07f` is one `f32`
//!   multiplication, and `trunc` is `x87::ftol32` read as `u32`;
//! - `GetTimePercentage` / `GetVisualTimePercentage`: [`mul_u32_f32`] of the
//!   seconds and `1.1574074e-05f`;
//! - `SetTimePercentage`: one `f32` multiplication by `86400.0f`, then
//!   `x87::ftol32` read as `u32` (a negative product wraps), `mod 86400`;
//! - `Stopwatch magnify n`: `f32(atol(n))`, one rounding (`as f32`).
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - The step is the low 32 bits of `_ftol`'s 64-bit truncation, compared
//!   unsigned (§4.3, CK8): a product of −1 or less wraps to a large step,
//!   which is clamped to 1200.
//! - `Timer::start` on a running timer does nothing (§4.3 allows it; the
//!   clock never starts a running timer). `Timer::restart` starts a stopped
//!   timer (CK15).
//! - `Init` registers each cheat and then subscribes to it, cheat by cheat
//!   (CK12). `Shutdown` unregisters only when the cheat code manager exists
//!   (CK13).
//! - The cheat text is split as §4.8 (CK11) says: separators space, tab and
//!   newline; a separator inside an odd number of quotes is kept; only an
//!   argument that starts and ends with a quote loses that outer pair.
//!   `atol` skips leading white space, takes one sign and wraps (CK10).
//! - `Init` starts the timer only when both pause counters are exactly 0, so
//!   a saved negative `pauseCount` leaves the clock frozen (§4.7, CK14).
//! - The dead fields at `+0x78` / `+0x8C` are omitted (CK3).

mod clock;
mod timer;

#[cfg(test)]
mod tests;

pub use clock::{atol, mul_u32_f32, split_command_line, Clock24, ClockState};
pub use timer::Timer;

/// Class and interface IDs (§1).
pub const CLSID_24_HOUR_CLOCK: u32 = 0xA7E7_F929;
pub const IID_24_HOUR_CLOCK: u32 = 0x681B_D53A;
pub const IID_GZ_UNKNOWN: u32 = 1;
pub const IID_GZ_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_GZ_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// Answered by the system-service base for any other IID *[inferred]* (§1).
pub const IID_SYSTEM_SERVICE: u32 = 0x287F_B697;

/// The system service (§1, §2).
pub const SERVICE_ID: u32 = 0xA7E7_F92A;
/// `0xFFE17B80`.
pub const SERVICE_PRIORITY: i32 = -2_000_000;

/// The cheat codes (§3.2) and the cheat message (§4.8).
pub const CHEAT_WHAT_TIME_IZ_IT: u32 = 0x281F_7858;
pub const CHEAT_STOPWATCH: u32 = 0xC9B7_3FBB;
pub const CHEAT_NAME_WHAT_TIME_IZ_IT: &str = "WhatTimeIzIt";
pub const CHEAT_NAME_STOPWATCH: &str = "Stopwatch";

/// Messages (§3.3, §4.8).
pub mod msg {
    /// data2 = the new hour, data3 = 0.
    pub const HOUR_CHANGED: u32 = 0x4477_8230;
    /// data2 = the hour, data3 = 0 with HourChanged, else the minute.
    pub const TIME_TICK: u32 = 0x4477_8231;
    pub const VISUAL_HOUR_CHANGED: u32 = 0x4477_8232;
    pub const VISUAL_TIME_TICK: u32 = 0x4477_8233;
    /// data2 = 1 day begins, 0 night begins.
    pub const DAY_NIGHT_CHANGED: u32 = 0x4477_8234;
    /// data2 = 1 paused, 0 resumed.
    pub const PAUSE_CHANGED: u32 = 0x4477_8235;
    pub const HIDDEN_PAUSE_CHANGED: u32 = 0x4477_8236;
    /// The cheat message: data1 = the cheat ID, void2 = the text.
    pub const CHEAT: u32 = 0x230E_27AC;
}

/// Time constants (§4).
pub const SECONDS_PER_DAY: u32 = 86_400;
pub const NOON: u32 = 43_200;
/// A step of this many game seconds or less waits (§4.3).
pub const STEP_THRESHOLD: u32 = 60;
/// Steps are clamped to 20 game minutes (§4.3).
pub const MAX_STEP: u32 = 1_200;
/// `9.99999997e-07f` (§4.3).
pub const STEP_FACTOR_BITS: u32 = 0x3586_37BD;
/// `1.1574074e-05f` (§4.1, CK7), `f32(1 / 86400)`: noon gives exactly 0.5.
pub const PERCENT_FACTOR_BITS: u32 = 0x3742_2E45;

/// Constructor values (§2).
pub const DEFAULT_MAGNIFICATION: f32 = 100.0;
/// `0.05f`.
pub const DEFAULT_MIN_MAGNIFICATION_BITS: u32 = 0x3D4C_CCCD;
pub const DEFAULT_MAX_MAGNIFICATION: f32 = 2000.0;
pub const DEFAULT_NIGHT_START: u32 = 20;
pub const DEFAULT_NIGHT_END: u32 = 4;

/// Visual time modes (§4.5).
pub const MODE_FOLLOW: i32 = 0;
pub const MODE_NOON: i32 = 1;
pub const MODE_MIDNIGHT: i32 = 2;

/// The record (§7).
pub const RECORD_VERSION: u16 = 2;
pub const RECORD_LEN: usize = 26;

/// The "Main Simulator" exemplar and its clock properties (§6).
pub mod tuning {
    pub const EXEMPLAR_TYPE: u32 = 0x6534_284A;
    pub const EXEMPLAR_GROUP: u32 = 0xE7E2_C2DB;
    pub const EXEMPLAR_INSTANCE: u32 = 0xE959_0E52;
    pub const PROP_MAGNIFICATION: u32 = 0x897E_709F;
    pub const PROP_NIGHT_START: u32 = 0x09B7_3421;
    pub const PROP_NIGHT_END: u32 = 0x09B7_3422;
}

/// The clock's properties of the "Main Simulator" exemplar (§6). `None`
/// fields are missing properties.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClockTuning {
    pub magnification: Option<f32>,
    pub night_start: Option<u32>,
    pub night_end: Option<u32>,
}

/// The shipped values (§6): magnification 135, night 20 → 1.
pub const SHIPPED_TUNING: ClockTuning = ClockTuning {
    magnification: Some(135.0),
    night_start: Some(20),
    night_end: Some(1),
};

/// Everything the clock needs from the game.
pub trait ClockHost {
    /// A monotonic clock in microseconds, the timer's unit 3 (§4.3, CK2).
    fn now_micros(&self) -> u64;
    /// `MessagePost` of a standard message (§3.3): the type, `data2` and
    /// `data3` (`void1` and the `IGZUnknown` field are the clock; the glue
    /// fills them). No message server: nothing.
    fn post(&self, msg_type: u32, data2: u32, data3: u32);
    /// `AddSystemService` (§4.2 step 1).
    fn add_system_service(&self);
    /// `RemoveSystemService` (§4.2 Shutdown step 4).
    fn remove_system_service(&self);
    /// `AddToOnIdle`.
    fn add_to_on_idle(&self);
    /// `RemoveFromOnIdle`.
    fn remove_from_on_idle(&self);
    /// Whether the cheat code manager exists (§3.2).
    fn has_cheat_manager(&self) -> bool;
    /// `RegisterCheatCode(id, name)`.
    fn register_cheat(&self, id: u32, name: &str);
    /// Subscribes the clock's message target to the cheat.
    fn add_cheat_notification(&self, id: u32);
    /// Unsubscribes it.
    fn remove_cheat_notification(&self, id: u32);
    /// `UnregisterCheatCode(id)`.
    fn unregister_cheat(&self, id: u32);
    /// The "Main Simulator" exemplar's clock properties (§6); `None` when the
    /// exemplar is missing.
    fn tuning(&self) -> Option<ClockTuning>;
}

/// `QueryInterface` (§1): the four IIDs the clock answers itself, and the
/// system-service interface its base answers *[inferred]*.
pub fn answers_iid(iid: u32) -> bool {
    matches!(
        iid,
        IID_GZ_UNKNOWN | IID_24_HOUR_CLOCK | IID_GZ_MESSAGE_TARGET2 | IID_GZ_SERIALIZABLE | IID_SYSTEM_SERVICE
    )
}

/// `hour(s) = s / 3600` (§4.1).
pub fn hour_of(s: u32) -> u32 {
    s / 3600
}

/// `minute(s) = (s − hour(s) × 3600) / 60`.
pub fn minute_of(s: u32) -> u32 {
    s.wrapping_sub(hour_of(s).wrapping_mul(3600)) / 60
}

/// `second(s) = s − (hour(s) × 60 + minute(s)) × 60`.
pub fn second_of(s: u32) -> u32 {
    s.wrapping_sub(hour_of(s).wrapping_mul(60).wrapping_add(minute_of(s)).wrapping_mul(60))
}
