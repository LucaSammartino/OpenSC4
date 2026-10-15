// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The history warehouse (`docs/specs/history.md`, `cSC4HistoryWarehouse`,
//! class `0x89EFA536`): time series of signed 32-bit values stamped with the
//! simulation day, recorded by the simulators and read back by the graphs,
//! the advisors and the demand simulator. Section references (`§n`) point at
//! that spec.
//!
//! # Structure
//!
//! - [`HistoryWarehouse`]: the state of §2, the messages of §3, the
//!   behaviour of §4 (recording, queue lengths, the yearly thinning pass,
//!   `Coalesce`, `Dispose`, ranges and queries), the record of §7 and the
//!   interface methods of §8.
//! - [`HistoryHost`]: the simulator's date and the pass timer, message
//!   subscriptions, agent registration and the synchronous `MessageSend` of
//!   `RecordHistoryData`.
//! - `stats`: the ranges (§4.6) and the regression (§4.7) at the caller's
//!   x87 precision ([`Precision`], foundations §18, HW3).
//!
//! # Precision (foundations §18)
//!
//! - The regression of `GetTrend` / `GetPoint` (and the `GetPoint` values
//!   `GetMinMaxAverage` adds) is computed at the caller's precision: each
//!   operation rounds once, with `x87::r24` at 24 bits, through [`add_r24`]
//!   for sums whose operands may carry more than 24 bits (the `int32` dates
//!   and values), and as IEEE `double` at 53 bits. The results are stored to
//!   `float` (`as f32`).
//! - `GetMinMaxAverage`'s `sum`, `min`, `max` and `count` are `float`
//!   variables: `sum += value` is one `fadd` at the caller's precision
//!   followed by a store to `float` (at 24 bits the store does not round
//!   again; at 53 bits it does). `min` / `max` compare the `int32` value with
//!   the `float` exactly and store the value as `float`. `avg = sum / count`
//!   is one correctly rounded `float` division.
//! - `GetCount` converts the count to `float`.
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - `RecordHistoryData` returns false (§4.1, HW12) and is an associated function:
//!   it only sends the record message through the host, which delivers it
//!   to every subscriber (the warehouse's own `DoMessage` stores it). It
//!   never touches the warehouse itself, so the host may re-enter the
//!   warehouse from inside the send.
//! - `Read` drops a point whose date is not after the previous one when the
//!   series has no point yet (the original writes into its list header, §7).
//!   It stops at the first failed read and returns false; the series read
//!   completely before the failure stay inserted, the one being read is
//!   dropped (§7, HW7: the original keeps the partial series' points).
//! - `r` of §4.7 multiplies each centred value by the rounded reciprocal of
//!   its standard deviation (HW5, not a choice any more).
//! - Ranges are walked like a list (`stats::walk_range`, HW6); a widened
//!   range with `from > to` and neither pre-check true is the first point
//!   after `to` and its predecessor.
//! - The pass cursor is held as the key of its series (`None` = the end); a
//!   series inserted during a pass is visited if its key comes after the
//!   cursor, as with a map iterator.
//! - `Coalesce` with a granularity other than 0–3 groups every point under
//!   −1000, so its single group is never closed and nothing changes.

mod stats;
mod warehouse;

#[cfg(test)]
mod tests;

pub use stats::{add_r24, fit_line, plain_range, statistics, walk_range, widened_range, Precision, Stats};
pub use warehouse::HistoryWarehouse;

/// Class and interface IDs (§1).
pub const CLSID_HISTORY_WAREHOUSE: u32 = 0x89EF_A536;
pub const IID_HISTORY_WAREHOUSE: u32 = 0x69F4_A803;
pub const IID_GZ_UNKNOWN: u32 = 1;
pub const IID_GZ_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// The message-target sub-object exists but `QueryInterface` does not answer
/// it (§1, [quirk]).
pub const IID_GZ_MESSAGE_TARGET2: u32 = 0x4522_94AA;

/// Agent registration (§3): one target at the day and the year levels.
pub const AGENT_NAME: &str = "cSC4HistoryWarehouse";
pub const AGENT_LEVEL_DAY: u32 = 4;
pub const AGENT_LEVEL_YEAR: u32 = 7;
/// Enabled whether or not the city is established (simulator.md §4).
pub const AGENT_FLAGS: u32 = 3;

/// The record version `Write` writes (§7); `Read` accepts 0–2.
pub const RECORD_VERSION: u16 = 2;

/// A saved date that reads as "the previous date + 1" (§7).
pub const DATE_UNSET: u32 = 0xCDCD_CDCD;

/// The yearly pass stops a call after this many timer milliseconds (§4.3).
pub const PASS_TIME_LIMIT_MS: u32 = 5;

/// `passState` (§2).
pub const PASS_IDLE: u32 = 0;
pub const PASS_ARMED: u32 = 1;
pub const PASS_RUNNING: u32 = 2;

/// Messages (§3).
pub mod msg {
    /// Record a value: data1 = key, data2 = the value. The same number as
    /// the class ID.
    pub const RECORD: u32 = 0x89EF_A536;
    /// Set a queue length: data1 = key, data2 = the length.
    pub const SET_QUEUE_LENGTH: u32 = 0x89EF_A537;
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_YEAR: u32 = 0x6695_6817;
}

/// `Coalesce` granularities (§4.4).
pub mod granularity {
    pub const MONTH: i32 = 0;
    pub const HALF_YEAR: i32 = 1;
    pub const YEAR: i32 = 2;
    pub const FIVE_YEARS: i32 = 3;
}

/// One point: a value stamped with a Julian day number (§2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub value: i32,
    pub date: u32,
}

impl Point {
    pub const fn new(date: u32, value: i32) -> Point {
        Point { value, date }
    }
}

/// One series (§2): its points in list order (ascending date, at most one per
/// day) and its queue length (`0` = no limit, thinned by the yearly pass).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Series {
    pub points: Vec<Point>,
    pub max_count: u32,
}

/// Everything the warehouse needs from the game.
pub trait HistoryHost {
    /// The simulator's current date number (`cISC4Simulator` slot 8).
    fn sim_date(&self) -> u32;
    /// A millisecond timer for the yearly pass (§4.3); only differences are
    /// used, with wrapping arithmetic.
    fn now_ms(&self) -> u32;
    /// Subscribes the message-target sub-object (+4) to `msg_type`.
    fn subscribe(&self, msg_type: u32);
    /// Unsubscribes it.
    fn unsubscribe(&self, msg_type: u32);
    /// `AddAgent(+4, level, name, flags)` on the city's simulator.
    fn add_agent(&self, level: u32, name: &str, flags: u32);
    /// `RemoveAgent(+4)` on the city's simulator (both levels).
    fn remove_agent(&self);
    /// `MessageSend` of a standard message (synchronous): every subscriber,
    /// the warehouse included, sees it before this returns.
    fn send(&self, msg_type: u32, data1: u32, data2: u32);
}

/// `QueryInterface` (§1): true for the three IIDs the original answers, and
/// for nothing else (not even `cIGZMessageTarget2`, [quirk]).
pub fn answers_iid(iid: u32) -> bool {
    matches!(iid, IID_GZ_UNKNOWN | IID_HISTORY_WAREHOUSE | IID_GZ_SERIALIZABLE)
}
