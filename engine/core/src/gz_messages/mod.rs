// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! GZ framework messaging: the second-generation message queue
//! (`cGZMessageQueue2`), the named-queue registry (`cGZMessageQueueManager2`),
//! message server 2 (`cGZMessageServer2`) and the first-generation message
//! server (`cGZMessageServer`).
//!
//! Written only from `docs/specs/framework-gz-services.md`; section references
//! (`§n`) point at that spec. Places where the spec leaves a decision open are
//! tagged `[choice]` and, where they matter, carry a question id (GZn) in
//! `docs/specs/questions/framework-gz-services.md`.
//!
//! The core holds no COM objects. Messages, targets and queues are opaque
//! host tokens; every reference the original takes or drops is reported
//! through a host callback (`add_ref` / `release`), so the glue layer can map
//! them 1:1 onto the game's AddRef/Release. Message type and equality, and the
//! target's `DoMessage`, are host callbacks too. Delivery callbacks receive the
//! server mutably, so a target may re-enter it (register, post, send, tick)
//! exactly where the original allows.
//!
//! Locks: the original guards each structure with a critical section. The core
//! is single-threaded (`&mut self`), so every operation is atomic with respect
//! to the others; the lock boundaries only matter where the original releases
//! a lock around a callback, and those points are kept (delivery runs with no
//! internal borrow held).
//!
//! The fourteen published GZ1-GZ7 vectors are represented here. Whole-owner
//! completion remains withheld: GZ8-GZ10 cover residual cleanup traces,
//! base-service behavior and the complete body inventory. Platform resources,
//! pointer adjustment and service base lifecycle remain host responsibilities.
//! Native invalid-iterator re-entry and cancellation are unsupported: callers
//! must preserve any live traversal/assignment/pop and must not cancel a last/sole deque
//! element. Raw generation-1 pending removals/directed targets require external
//! lifetime holds. No deterministic behavior is promised for native UB.

pub mod manager;
pub mod queue;
pub mod registry;
pub mod server1;
pub mod server2;

#[cfg(test)]
mod tests;

pub use manager::{QueueHost, QueueManager2};
pub use queue::{signal_wait_millis, signal_wait_result, MessageHost, MessageQueue2};
pub use server1::{MessageServer1, Record, Server1Host, SERVER1_SNAPSHOT_LIMIT};
pub use server2::{MessageServer2, Server2Host};

/// An opaque host token (a message, a target or a queue). The glue layer
/// decides what the value means (typically the COM pointer).
pub type Token = u64;

/// Service id of message server 2 (§1).
pub const MESSAGE_SERVER2_SERVICE_ID: u32 = 0x04FA_845B;
/// Interface id of `cIGZMessageServer2` (§1).
pub const MESSAGE_SERVER2_IID: u32 = 0x6522_94C7;
/// Service id of the queue manager (§1).
pub const QUEUE_MANAGER2_SERVICE_ID: u32 = 0x64FA_8434;
/// Interface id of the queue manager (§1, §3 slot 0).
pub const QUEUE_MANAGER2_IID: u32 = 0x6522_94BC;
/// Service id of the first-generation message server (§1).
pub const MESSAGE_SERVER1_SERVICE_ID: u32 = 0x6406_3387;
/// Interface id of `cIGZMessageServer` (§4A).
pub const MESSAGE_SERVER1_IID: u32 = 0xC108_5722;
/// Tick priority of both servers and the queue manager (§1).
pub const TICK_PRIORITY: i32 = 5_000_000;
/// The only class id `CreateMessage` knows (§4.6).
pub const STANDARD_MESSAGE_CLSID: u32 = 0x46DF_45E7;
/// Timeout argument the blocking gets pass to the signal wait (§2 slots
/// 20/21; unsigned microseconds, GZ2).
pub const BLOCKING_WAIT_TIMEOUT: u32 = 1_000_000;
/// The wildcard notification type (§4.2).
pub const WILDCARD_TYPE: u32 = 0;
