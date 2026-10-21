// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//! The underground pieces (`network-occupants.md` §3): the shared
//! `cSC4UndergroundOccupant` behaviour of §3.2 and the pipe/subway members of
//! §3.3/§3.5 that the spec fully states.
//!
//! Implemented: `GetSortValue`, `GetPosition`, `GetTerrainHeights`, the
//! five-parameter `Init`'s terrain-height store, slot 104's model choice,
//! `IsImaginaryConnection` (found case), pipe `SetWateredState`, pipe `Init`'s
//! `+0x270` reset, the folded pipe/subway `Shutdown`.
//!
//! Blocked (question NO17): the §3.4 underground record (its version-3 base,
//! its offsets against §3.1's layout, the wall-quad helper `0x0065B070`), so
//! the pipe (v3) and subway (v2) records too; `SetTerrainHeights`
//! (float[4] into five floats `+0x248…+0x258`); the uv ratios (class-info
//! layout); `GetZipperPieceType`'s result mapping; geometry and drawing.

use super::identity::NetOccClass;
use super::piece::{NetworkPiece, PieceHost};

/// `GetSortValue`'s fallback for the underground base (§3.2).
pub const UNDERGROUND_SORT_DEFAULT: u32 = 0x4A10_AFC2;
/// The network flags that make `GetSortValue` return the stored value.
pub const SORT_STORED_FLAGS: u32 = 0x0600_0000;
/// Slot 104: the burst-model flag.
pub const FLAG_BURST: u32 = 0x0002_0000;
/// Slot 104: the distressed-model flag.
pub const FLAG_DISTRESSED: u32 = 0x0001_0000;
/// `GetPosition`'s guard value (`FLT_MAX`, `0x00AAABB0`).
pub const HEIGHT_GUARD: f32 = f32::MAX;

/// Slot 104's result (plumbing.md cites `0x0065A880`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndergroundModel {
    Normal,
    Burst,
    Distressed,
}

/// The calls an underground piece makes into the game.
pub trait UndergroundHost {
    /// `FindUndergroundOccupant` at neighbour `dir`: that piece's network
    /// flags, or `None` when there is none.
    fn neighbour_flags(&mut self, dir: u32) -> Option<u32>;
    /// Primary slot 100 (`0x0065AF10`): sets `+0x260` from city slot 0x18C.
    fn refresh_cell_value(&mut self);
}

/// One of the five points `GetPosition` sums (stride 0x18 from `+0x98`):
/// only the dwords at `+0` and `+8` are named by §3.2.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerPoint {
    /// `+0` (summed into x).
    pub x: f32,
    /// `+8` (summed into z; slot 84 `SetVertexHeights` writes points 0…3's).
    pub z: f32,
}

/// The shared underground state on top of a [`NetworkPiece`].
#[derive(Clone, Debug)]
pub struct UndergroundPiece {
    pub piece: NetworkPiece,
    /// The class's `GetSortValue` default: the underground base ID, or
    /// the subway's (`0xCA16374F`) / pipe's own class ID (§3.2 "the pipe
    /// shape is the same" *[literal: its own class ID]*).
    pub sort_default: u32,
    /// The stored sort value (its field is NO7).
    pub sort_value: u32,
    /// `+0x98`…: the five points of `GetPosition`.
    pub points: [CornerPoint; 5],
    /// `+0x248`, `+0x24C`, `+0x250`, `+0x254`, `+0x258`.
    pub heights: [f32; 5],
    /// Pipe `+0x270` (reset by pipe `Init`).
    pub pipe_270: u32,
    /// Pipe watered state (`SetWateredState`).
    pub watered: u32,
    /// The owned pieces `Shutdown` releases (textures, wall vectors,
    /// `+0x244`), as a count of held resources.
    pub held_resources: u32,
}

impl UndergroundPiece {
    /// The underground state over `piece` (an underground, pipe or subway
    /// piece; the class picks the sort default).
    pub fn new(piece: NetworkPiece) -> UndergroundPiece {
        let sort_default = match piece.class {
            NetOccClass::Subway | NetOccClass::Pipe => piece.class.class_id(),
            _ => UNDERGROUND_SORT_DEFAULT,
        };
        UndergroundPiece {
            piece,
            sort_default,
            sort_value: 0,
            points: [CornerPoint::default(); 5],
            heights: [0.0; 5],
            pipe_270: 0,
            watered: 0,
            held_resources: 0,
        }
    }

    /// `GetSortValue` (`0x0065A0E0`).
    pub fn get_sort_value(&self) -> u32 {
        if self.piece.has_any_network_flag(SORT_STORED_FLAGS) {
            self.sort_value
        } else {
            self.sort_default
        }
    }

    /// `GetPosition` (`0x0065A720`): x/z from the first point plus the next
    /// four points' x/z (a plain sum, as §3.2 states), y the minimum of the
    /// four heights `+0x248…+0x254` starting from `FLT_MAX`.
    pub fn get_position(&self) -> [f32; 3] {
        let y = self.heights[..4]
            .iter()
            .fold(HEIGHT_GUARD, |m, &h| if h < m { h } else { m });
        let mut x = self.points[0].x;
        let mut z = self.points[0].z;
        for p in &self.points[1..] {
            x += p.x;
            z += p.z;
        }
        [x, y, z]
    }

    /// `GetTerrainHeights` slot 94 (`0x00659DD0`): `+0x24C…+0x258`.
    pub fn get_terrain_heights(&self) -> [f32; 4] {
        [self.heights[1], self.heights[2], self.heights[3], self.heights[4]]
    }

    /// The five-parameter `Init`'s height store (`0x0065C6B0`): the floats at
    /// `p2+4/+0x10/+0x1C/+0x28` into `+0x24C…+0x258`.
    pub fn store_init_heights(&mut self, p2: &[f32; 4]) {
        self.heights[1..].copy_from_slice(p2);
    }

    /// Slot 84 `SetVertexHeights` (`0x0065AE30`), the stores only: points
    /// 0…3's `+8` (`+0xA0/+0xB8/+0xD0/+0xE8`). The rebuild is the host's.
    pub fn set_vertex_heights(&mut self, h: &[f32; 4]) {
        for (p, &v) in self.points.iter_mut().zip(h.iter()) {
            p.z = v;
        }
    }

    /// Slot 104 (`0x0065A880`): burst before distressed.
    pub fn model_choice(&self) -> UndergroundModel {
        if self.piece.network_flags & FLAG_BURST != 0 {
            UndergroundModel::Burst
        } else if self.piece.network_flags & FLAG_DISTRESSED != 0 {
            UndergroundModel::Distressed
        } else {
            UndergroundModel::Normal
        }
    }

    /// `IsImaginaryConnection` (`0x0065A640`): a neighbour found →
    /// `!(bit 31 of its flags)`. §3.2 does not say what a missing neighbour
    /// gives (NO17), so that case is `None`.
    pub fn is_imaginary_connection(&self, dir: u32, host: &mut dyn UndergroundHost) -> Option<bool> {
        host.neighbour_flags(dir).map(|f| f & 0x8000_0000 == 0)
    }

    /// Pipe `SetWateredState` (`0x00648540`, non-virtual): store; on a change
    /// call primary slot 100.
    pub fn set_watered_state(&mut self, state: u32, host: &mut dyn UndergroundHost) {
        if self.watered != state {
            self.watered = state;
            host.refresh_cell_value();
        }
    }

    /// Pipe `Init`'s own step after the underground `Init`: `+0x270 = 0`.
    pub fn pipe_init_tail(&mut self) {
        self.pipe_270 = 0;
    }

    /// The underground `Shutdown` (`0x0065CD90`), also the folded pipe/subway
    /// slot 4 (`0x00652CF0`): release the owned resources, then the base
    /// `Shutdown`.
    pub fn shutdown(&mut self, host: &mut dyn PieceHost) {
        self.held_resources = 0;
        self.piece.shutdown(host);
    }
}
