// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The cost of a bridge (§12) and the failure codes (§13).

use super::{BridgeRecord, BridgeTool};

/// §13: the failure codes the network tool sets around bridges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// `0xF0000003`: `IdentifyBridgeCells` lowered `v`.
    NoSpan,
    /// `0xF0000001`: a wet step that the water pass skipped.
    BadTerrain,
}

/// §13's `0xF0000003`.
pub const CODE_NO_SPAN: u32 = 0xF000_0003;
/// §13's `0xF0000001`.
pub const CODE_BAD_TERRAIN: u32 = 0xF000_0001;

impl Failure {
    /// The code as the game stores it.
    pub fn code(self) -> u32 {
        match self {
            Failure::NoSpan => CODE_NO_SPAN,
            Failure::BadTerrain => CODE_BAD_TERRAIN,
        }
    }
}

/// §12: the bridge term of `GetCostOfSolution` (slot 20), in integers
/// (`0x00625740`–`0x0062577F`):
///
/// `(end − start) × (bridgeType+0x154 + tool+0x320) + bridgeType+0x150`
///
/// The bridge type is found from the record's own network type (`+0x00`) and
/// index (`+0x0C`). `per_step` is the tool's `+0x320` (100 for road, rail and
/// their kin, 1000 for highway and avenue). Land and water bridges cost the
/// same way.
pub fn bridge_cost(tool: &BridgeTool, r: &BridgeRecord, per_step: u32) -> u32 {
    let Some(t) = tool.type_of_record(r) else {
        return 0;
    };
    let steps = r.end.saturating_sub(r.start);
    steps
        .saturating_mul(t.cost_per_step.saturating_add(per_step))
        .saturating_add(t.cost_fixed)
}

/// §12 over every record of the run.
pub fn total_bridge_cost(tool: &BridgeTool, per_step: u32) -> u32 {
    tool.records
        .iter()
        .fold(0u32, |a, r| a.saturating_add(bridge_cost(tool, r, per_step)))
}

/// The bridge term for the geometry core's `extras::CostBridge` (§12): the
/// per-step part and the fixed part.
pub fn cost_terms(tool: &BridgeTool, r: &BridgeRecord, per_step: u32) -> (u32, u32) {
    let Some(t) = tool.type_of_record(r) else {
        return (0, 0);
    };
    (
        t.cost_per_step.saturating_add(per_step),
        t.cost_fixed,
    )
}
