// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tool's bridge slots (§1.1). The names are the Mac build's; the pops
//! were read from each target's `ret`.
//!
//! These are read from a second thread while `OnTick` runs (HANDOFF §5.1),
//! so they take `&self` over plain data and never a `RefCell`.

use super::BridgeTool;

/// The results of the §1.1 slots, as the C ABI would return them.
pub struct ToolBridgeGetters;

impl ToolBridgeGetters {
    /// Slot 30 `GetPossibleBridges(&list)`: copies the candidate list
    /// `+0x1F4` into `list`; false if it is empty.
    pub fn get_possible_bridges(tool: &BridgeTool, list: &mut Vec<u32>) -> bool {
        if tool.candidates.is_empty() {
            return false;
        }
        list.clear();
        list.extend_from_slice(&tool.candidates);
        true
    }

    /// Slot 31 `GetPreferredBridge()`: returns `+0x200`.
    pub fn get_preferred_bridge(tool: &BridgeTool) -> i32 {
        tool.preferred
    }

    /// Slot 32 `SetPreferredBridge(index)`: `+0x200 := index`.
    pub fn set_preferred_bridge(tool: &mut BridgeTool, index: i32) {
        tool.preferred = index;
    }

    /// Slot 33 `GetBridgeAltitudeOverride()`: returns `+0x204`.
    pub fn get_bridge_altitude_override(tool: &BridgeTool) -> f32 {
        tool.altitude_override
    }

    /// Slot 34 `SetBridgeAltitudeOverride(h)`: `+0x204 := h`.
    pub fn set_bridge_altitude_override(tool: &mut BridgeTool, h: f32) {
        tool.altitude_override = h;
    }

    /// Slot 35 `GetBridgeName(index, &string)`: the bridge type's name
    /// (`+0x15C`). `index` is an index into the network type's bridge type
    /// vector, **without a bounds check** (§1.1).
    pub fn get_bridge_name(tool: &BridgeTool, index: u32) -> Option<&str> {
        tool.types.get(index as usize).map(|t| t.name.as_str())
    }

    /// Slot 36 `GetMajorBridge()`: the **last** record's bridge type index
    /// (`+0x0C`); −1 if there is no record.
    pub fn get_major_bridge(tool: &BridgeTool) -> i32 {
        match tool.major() {
            Some(r) => r.bridge_type as i32,
            None => -1,
        }
    }

    /// Slot 37 `GetMajorBridgeAltitude(&h)`: `h :=` last record `+0x10`;
    /// false if there is no record.
    pub fn get_major_bridge_altitude(tool: &BridgeTool) -> Option<f32> {
        tool.major().map(|r| r.deck)
    }

    /// Slot 38 `GetMajorBridgeReferenceAltitude(&h)`: `h :=` last record
    /// `+0x14`; false if there is no record.
    pub fn get_major_bridge_reference_altitude(tool: &BridgeTool) -> Option<f32> {
        tool.major().map(|r| r.reference)
    }

    /// Slot 39 `IsMajorBridgeLandBridge()`: the last record's byte `+0x18`;
    /// false if there is no record.
    pub fn is_major_bridge_land_bridge(tool: &BridgeTool) -> Option<bool> {
        tool.major().map(|r| r.land != 0)
    }

    /// Slot 40 `GetBridgeBitmapKey(index, &a, &b)`: the bridge type's
    /// `+0x148` and `+0x14C`; `None` (outputs untouched) if either is 0.
    pub fn get_bridge_bitmap_key(tool: &BridgeTool, index: u32) -> Option<(u32, u32)> {
        let t = tool.types.get(index as usize)?;
        let (a, b) = t.bitmap_key;
        if a == 0 || b == 0 {
            None
        } else {
            Some((a, b))
        }
    }

    /// `Reset` (§1.1): `+0x200 := −1` (no preference) and `+0x204 := −1.0`
    /// (no override).
    pub fn reset_preferences(tool: &mut BridgeTool) {
        tool.preferred = -1;
        tool.altitude_override = -1.0;
    }
}
