// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Shore ripple and selection kernels from `water-ripples.md` §§1–2,6.
//!
//! These are unbound pure-core owners, not complete native replacements. See
//! [`OWNER_CHECKLIST`]. In particular no host may supply traced shores, ripple
//! meshes, selection masks or owner draw results to bypass a missing contract.
//! Missing algorithms return a named [`ContractGap`], rather than fabricated
//! geometry. Raw tick units and flag/byte values are deliberately opaque (WR1–3).
//! Resource lifetimes use `Arc`; getters never use interior mutability.

mod geometry;
pub mod mesh;
pub mod render;
pub mod shore;
pub mod lifecycle;
pub mod retained;
mod ripple;
mod selection;
pub mod storage;
mod tuning;

pub use geometry::{
    candidate_pair, spline, AnimatedStrip, CandidatePair, FrameAnimation, PairDecision, PairFilter, Point,
    RippleVertex, SplinePoint, StripGeometry,
};
pub use ripple::{RebuildState, RippleMessage, RippleUpdate, WaterRipple};
pub use selection::{Material, SelectedTerrainArea, SelectionRenderProperties, TexturedCell};
pub use tuning::{RippleTuning, Zoom, RIPPLE_EXEMPLAR, RIPPLE_TEXTURE};

use std::sync::Arc;

/// A public checklist of owner coverage. `false` means no complete-owner credit.
pub const OWNER_CHECKLIST: &[(&str, bool, &str)] = &[
    (
        "cSTEWaterRipple: tuning and literal flag eligibility",
        true,
        "§§1.1–1.2",
    ),
    (
        "cSTEWaterRipple: spline, normals, pair filters",
        true,
        "§1.3; WR5 limits",
    ),
    ("cSTEWaterRipple: ordered shore tracing", false, "WR4"),
    (
        "cSTEWaterRipple: full strip traversal and preparation",
        false,
        "WR5–WR6",
    ),
    (
        "cSTEWaterRipple: animation and delayed rebuild state",
        true,
        "§1.5; WR6/WR10 limits",
    ),
    ("cSTEWaterRipple: frustum, render state, draw", false, "WR7"),
    (
        "cSTEWaterRipple: native lifecycle and all owner entry points",
        false,
        "WR8",
    ),
    (
        "cSTESelectedTerrainArea: five marking forms and exclusivity",
        true,
        "§2.2; WR9 limits",
    ),
    (
        "cSTESelectedTerrainArea: textured masks and material states",
        false,
        "WR7/WR9",
    ),
    (
        "cSTESelectedTerrainArea: native lifecycle and all entry points",
        false,
        "WR8",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractGap {
    ShoreTraceWr4,
    SplineTraversalWr5,
    StripPreparationWr5,
    AnimationOvershootWr6,
    SelectionGeometryWr7,
    RenderStatesWr7,
    NativeLifecycleWr8,
    PaletteOutsideTableWr9,
    PackedSelectionByteWr9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotInitialized,
    InvalidDimensions,
    InvalidCell,
    InvalidRectangle,
    InvalidTuning,
    InvalidGeometry,
    InvalidRandomValue,
    EmptyTexture,
    MissingTerrainService,
    UnsetRandomState,
    UndefinedTexturePreparation,
    UnsupportedTuning(retained::Unsupported),
    Gap(ContractGap),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell {
    pub x: u32,
    pub z: u32,
}

/// Inclusive vertex rectangle. Terrain-edit area uses `cell_area`, not vertex count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub min: Cell,
    pub max: Cell,
}

impl CellRect {
    pub fn new(min: Cell, max: Cell) -> Result<Self, Error> {
        if min.x > max.x || min.z > max.z {
            return Err(Error::InvalidRectangle);
        }
        Ok(Self { min, max })
    }

    pub fn cell_area(self) -> u64 {
        // Invalid external rectangles are rejected by owner entry points. Keep
        // this diagnostic helper total even for directly constructed bad input.
        u64::from(self.max.x.saturating_sub(self.min.x)) * u64::from(self.max.z.saturating_sub(self.min.z))
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: Cell {
                x: self.min.x.min(other.min.x),
                z: self.min.z.min(other.min.z),
            },
            max: Cell {
                x: self.max.x.max(other.max.x),
                z: self.max.z.max(other.max.z),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}

impl Bounds {
    pub fn contains_away_from_edge(self, p: Point) -> bool {
        p.x - self.min.x > 0.2 && self.max.x - p.x > 0.2 && p.z - self.min.z > 0.2 && self.max.z - p.z > 0.2
    }
}

/// External terrain data only. No shore-trace or owner-generated mesh callbacks.
pub trait TerrainProvider: Send + Sync {
    fn cell_counts(&self) -> [u32; 2];
    fn vertex_flags(&self, cell: Cell) -> u32;
    fn vertex_position(&self, cell: Cell) -> Point;
    fn water_level(&self) -> f32;
    fn edit_counter(&self) -> u64;
    /// Foreign height and shoreline refresh services. No host tracing is accepted.
    fn vertex_height(&self, _cell: Cell) -> Result<f32, Error> {
        Err(Error::MissingTerrainService)
    }
    fn refresh_shore_flags(&self) -> Result<(), Error> {
        Err(Error::MissingTerrainService)
    }
    fn set_vertex_flags(&self, _cell: Cell, _flags: u16) -> Result<(), Error> {
        Err(Error::MissingTerrainService)
    }
    fn city_extent(&self) -> Result<Point, Error> {
        Err(Error::MissingTerrainService)
    }
    fn current_zoom(&self) -> Result<Zoom, Error> {
        Err(Error::MissingTerrainService)
    }
}

/// External texture resource. Frame texture handles stay in the eventual adapter.
pub trait TextureBinding: Send + Sync {
    fn frame_count(&self) -> u32;
}

pub trait TextureFactory: Send + Sync {
    fn bind_slot6(&self, key: [u32; 3]) -> Option<Arc<dyn TextureBinding>>;
}

/// Random numbers are primitive inputs, never prepared owner output.
pub trait RandomSource {
    fn below(&mut self, exclusive_max: u32) -> u32;
    fn inclusive(&mut self, min: u64, max: u64) -> u64;
}

fn dimensions(terrain: &dyn TerrainProvider) -> Result<[u32; 2], Error> {
    let counts = terrain.cell_counts();
    // Rust boundary validation, not a claim about native unchecked dimensions (WR8).
    if counts.contains(&u32::MAX) || u64::from(counts[0] + 1) * u64::from(counts[1] + 1) > u64::from(u32::MAX) {
        return Err(Error::InvalidDimensions);
    }
    Ok(counts)
}

fn vertex_index(cell: Cell, counts: [u32; 2]) -> Result<u32, Error> {
    if cell.x > counts[0] || cell.z > counts[1] {
        return Err(Error::InvalidCell);
    }
    Ok(cell.z * (counts[0] + 1) + cell.x)
}

#[cfg(test)]
mod tests;
