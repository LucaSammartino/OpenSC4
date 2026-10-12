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

//! Portable FreeCity4 simulation core.
//!
//! This crate contains no GZCOM or Windows types. It is written only from the
//! clean-room behaviour specs in `docs/specs/` (currently
//! `docs/specs/simulator.md`). Section references (`§n`) in comments point at
//! that spec. Places where the spec left a decision open are tagged `[choice]`.

#![forbid(unsafe_code)]

pub mod host;
pub mod ids;
pub mod julian;
pub mod save;
pub mod simulator;

pub use host::{AgentDelivery, AgentHandle, Broadcast, Clock, Host, Message};
pub use save::{LoadError, SaveRecord};
pub use simulator::{Simulator, Snapshot, Tuning};
