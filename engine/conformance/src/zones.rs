// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `zone_…` for `fc4_core::zones` (`docs/specs/zones.md`).

use crate::json::Json;
use crate::runner::HostRef;
use crate::scalar::R;

pub(crate) fn exec(_host: &HostRef, name: &str, _op: &Json) -> Option<R<()>> {
    let _rest = name.strip_prefix("zone_")?;
    None
}
