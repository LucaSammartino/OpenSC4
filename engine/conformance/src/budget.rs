// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/budget.md` (filled in with the budget core).

use fc4_core::Simulator;

use crate::json::Json;
use crate::runner::HostRef;

/// Executes a budget op; `None` if `name` is not one.
pub(crate) fn exec(_sim: &Simulator, _host: &HostRef, _name: &str, _op: &Json) -> Option<Result<(), String>> {
    None
}
