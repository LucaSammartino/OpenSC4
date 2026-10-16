// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network flags (§5), the usable message, the pipe model choice and
//! the occupant removal counters the riot lock uses (§10).

/// Network occupant flags (§5). The low 13 bits are the network types
/// (`1 << type`, `crate::traffic_network::net_type`).
pub mod flag {
    pub const ROAD: u32 = 0x0001;
    pub const RAIL: u32 = 0x0002;
    pub const HIGHWAY: u32 = 0x0004;
    pub const STREET: u32 = 0x0008;
    pub const PIPE: u32 = 0x0010;
    pub const POWER_LINE: u32 = 0x0020;
    pub const AVENUE: u32 = 0x0040;
    pub const SUBWAY: u32 = 0x0080;
    pub const LIGHT_RAIL: u32 = 0x0100;
    pub const MONORAIL: u32 = 0x0200;
    pub const ONE_WAY_ROAD: u32 = 0x0400;
    pub const DIRT_ROAD: u32 = 0x0800;
    pub const GROUND_HIGHWAY: u32 = 0x1000;

    /// Every type but pipe, power line and subway (the surface networks).
    pub const SURFACE: u32 = 0x1F4F;
    /// Every network type.
    pub const ALL_TYPES: u32 = 0x1FFF;
    /// Pipe, power line and subway.
    pub const UNDERGROUND: u32 = 0x00B0;
    /// Pipe and subway.
    pub const PIPE_AND_SUBWAY: u32 = 0x0090;
    /// Road, highway and street.
    pub const ROAD_HIGHWAY_STREET: u32 = 0x000D;

    /// Path connections exist (traffic-network.md §7.2).
    pub const PATH_CONNECTIONS: u32 = 0x2000;
    /// When set, the occupant also calls its slot 58 (`SetTranslucency(1,
    /// 0x80)`).
    pub const TRANSLUCENT: u32 = 0x4000;
    /// Pipe distressed (leaking) / burst (plumbing.md).
    pub const PIPE_DISTRESSED: u32 = 0x1_0000;
    pub const PIPE_BURST: u32 = 0x2_0000;
    /// Meaning unknown (traffic-network.md record flag bit 0; NM3).
    pub const UNKNOWN_40000: u32 = 0x4_0000;
    /// Meaning unknown: the terrain rule keeps it for kind 6 (NM3).
    pub const UNKNOWN_80000: u32 = 0x8_0000;
    /// Part of an intersection or complex piece.
    pub const INTERSECTION: u32 = 0x10_0000;
    /// A lot connection node (excluded from the graph, the lots and style).
    pub const CONNECTION_NODE: u32 = 0x20_0000;
    /// Immovable (a prebuilt-model piece the tool did not place).
    pub const IMMOVABLE: u32 = 0x40_0000;
    /// Set together by road style when an under-texture is applied.
    pub const UNDER_TEXTURE: u32 = 0x600_0000;
    /// Set by `CreateConnectionNode` and the retrofit (presentation).
    pub const PRESENTATION_8000000: u32 = 0x800_0000;
    /// Usable (slots 19 `IsUsable` / 20 `SetUsable`).
    pub const USABLE: u32 = 0x1000_0000;
    /// Unknown; road style skips it.
    pub const UNKNOWN_40000000: u32 = 0x4000_0000;
    /// Unknown; the subway count and `UpdateRoadsInCell` skip it.
    pub const UNKNOWN_80000000: u32 = 0x8000_0000;

    /// Road style is skipped for any of these (§8.7 step 1): highway, light
    /// rail, monorail, `0x80000`, connection node, `0x40000000`.
    pub const ROAD_STYLE_SKIP: u32 = 0x4028_0304;
    /// `UpdateRoadsInCell` stops for any of these (§8.5 step 2).
    pub const ROADS_IN_CELL_SKIP: u32 = 0x8020_0000;
}

/// `HasNetworkFlag(m)` (slot 8): all bits of `m`.
pub fn has_all(flags: u32, m: u32) -> bool {
    flags & m == m
}

/// `HasAnyNetworkFlag(m)` (slot 9).
pub fn has_any(flags: u32, m: u32) -> bool {
    flags & m != 0
}

/// The outcome of `SetNetworkFlag` / `ClearNetworkFlag` on a network
/// occupant (§5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlagChange {
    /// The flags afterwards.
    pub flags: u32,
    /// `Some(data1)`: post (queued) `0x89D3E9F3` with IGZUnknown = the
    /// occupant and this data1, before the bits are applied.
    pub usable_post: Option<u32>,
    /// The occupant also calls its slot 58 `SetTranslucency(1, 0x80)`.
    pub translucency: bool,
}

/// `SetNetworkFlag(bits)` (`0x00619EC0`, §5) *[compat]*: the usable message
/// with data1 = 1 when the new bits contain `0x10000000`, the occupant did
/// not have it and the occupant is in the city (`GetFlags() & 1`); then the
/// bits are applied. Bits containing `0x4000` also call slot 58.
pub fn set_network_flag(flags: u32, bits: u32, in_city: bool) -> FlagChange {
    let post = (bits & flag::USABLE != 0 && flags & flag::USABLE == 0 && in_city).then_some(1);
    FlagChange {
        flags: flags | bits,
        usable_post: post,
        translucency: bits & flag::TRANSLUCENT != 0,
    }
}

/// `ClearNetworkFlag(bits)` (`0x00619F70`, §5): the usable message with
/// data1 = 0 when it clears `0x10000000` that was set (the city test is not
/// made); then the bits are cleared.
pub fn clear_network_flag(flags: u32, bits: u32) -> FlagChange {
    let post = (bits & flag::USABLE != 0 && flags & flag::USABLE != 0).then_some(0);
    FlagChange {
        flags: flags & !bits,
        usable_post: post,
        translucency: false,
    }
}

/// The visual model of an underground occupant (§5 "The pipe flags").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipeModel {
    Normal,
    Distressed,
    Burst,
}

/// Burst if `0x20000` is set, else distressed if `0x10000` is set, else
/// normal (§5).
pub fn pipe_model(flags: u32) -> PipeModel {
    if flags & flag::PIPE_BURST != 0 {
        PipeModel::Burst
    } else if flags & flag::PIPE_DISTRESSED != 0 {
        PipeModel::Distressed
    } else {
        PipeModel::Normal
    }
}

/// An occupant's removal counters (§10 step 3, `0x005E8A60`, `0x005E8AA0`,
/// `0x005E8AE0`): a byte counter per bit, bit 1 the system's and bit 2 the
/// player's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemovalCounters {
    /// `[system (bit 1), player (bit 2)]`.
    pub counts: [u8; 2],
}

impl RemovalCounters {
    /// `SetRemovalFlags(m)`: increments the counter of each bit of `m`
    /// (wrapping, a byte).
    pub fn set_removal_flags(&mut self, m: u32) {
        for (i, c) in self.counts.iter_mut().enumerate() {
            if m & (1 << i) != 0 {
                *c = c.wrapping_add(1);
            }
        }
    }

    /// `UnsetRemovalFlags(m)`: decrements them (0 wraps to 255).
    pub fn unset_removal_flags(&mut self, m: u32) {
        for (i, c) in self.counts.iter_mut().enumerate() {
            if m & (1 << i) != 0 {
                *c = c.wrapping_sub(1);
            }
        }
    }

    /// `CanRemove(level)`: true while the level's counter is 0 (a level of
    /// several bits needs all of them at 0; other bits are ignored)
    /// [choice].
    pub fn can_remove(&self, level: u32) -> bool {
        self.counts
            .iter()
            .enumerate()
            .all(|(i, c)| level & (1 << i) == 0 || *c == 0)
    }
}
