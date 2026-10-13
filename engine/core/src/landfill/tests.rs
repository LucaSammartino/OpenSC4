// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the landfill developer (§11).

use super::*;

#[test]
fn placeholder() {
    assert!(answers_iid(IID_LANDFILL_DEVELOPER));
}
