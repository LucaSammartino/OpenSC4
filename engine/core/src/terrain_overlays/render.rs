// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{ContractGap, Error, OverlayManager};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthFunction {
    LessOrEqual,
    Greater,
    Less,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendFactor {
    SourceAlpha,
    OneMinusSourceAlpha,
    One,
    Zero,
    SourceColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawState {
    pub color_write: bool,
    pub depth_test: bool,
    pub blending: bool,
    pub textures: u8,
    pub stage_zero_mode: Option<u8>,
    pub depth_write: bool,
    pub depth_function: DepthFunction,
    pub blend: [BlendFactor; 2],
    pub alpha_greater_than: Option<f32>,
}

const STANDARD: DrawState = DrawState {
    color_write: true,
    depth_test: true,
    blending: true,
    textures: 1,
    stage_zero_mode: Some(1),
    depth_write: false,
    depth_function: DepthFunction::LessOrEqual,
    blend: [BlendFactor::SourceAlpha, BlendFactor::OneMinusSourceAlpha],
    alpha_greater_than: None,
};

/// Â§4.1, including state 4 preserving the existing stage-0 mode.
pub const DRAW_STATES: [DrawState; 5] = [
    STANDARD,
    DrawState {
        blend: [BlendFactor::SourceAlpha, BlendFactor::One],
        ..STANDARD
    },
    DrawState {
        blend: [BlendFactor::Zero, BlendFactor::SourceColor],
        ..STANDARD
    },
    DrawState {
        depth_function: DepthFunction::Greater,
        ..STANDARD
    },
    DrawState {
        depth_write: true,
        depth_function: DepthFunction::Less,
        stage_zero_mode: None,
        alpha_greater_than: Some(0.5),
        ..STANDARD
    },
];

#[derive(Clone, Copy, Debug)]
pub struct FrameOptions {
    pub snapshot: bool,
    pub decals: bool,
    pub shadows: bool,
    pub use_second_stage: bool,
}

/// Selection diagnostics only, not a GPU command stream or claimed completed
/// draws. Stats are never increased merely because a record was selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderSelection {
    pub skipped_snapshot: bool,
    pub decals: Vec<u32>,
    pub rough_shadows: bool,
}

impl OverlayManager {
    /// Â§4. Static candidates are deduplicated across visible cells. [choice,
    /// OV8] Internal order is ascending overlay ID; original draw order is open.
    pub fn select_frame(&self, options: FrameOptions, visible_cells: &[[u32; 2]]) -> Result<RenderSelection, Error> {
        self.require_init()?;
        if self.dynamic && options.snapshot {
            return Ok(RenderSelection {
                skipped_snapshot: true,
                decals: Vec::new(),
                rough_shadows: self.shadow_quality < 3 || !options.use_second_stage,
            });
        }
        let candidates: BTreeSet<u32> = if let Some(index) = &self.cell_index {
            visible_cells
                .iter()
                .filter_map(|c| index.get(c))
                .flatten()
                .copied()
                .collect()
        } else {
            self.overlays.iter().map(|(i, _)| i).collect()
        };
        let decals = if options.decals {
            candidates
                .into_iter()
                .filter(|&i| self.overlays.get(i).is_some_and(|o| o.decal_visible(options.snapshot)))
                .collect()
        } else {
            Vec::new()
        };
        Ok(RenderSelection {
            skipped_snapshot: false,
            decals,
            rough_shadows: self.shadow_quality < 3 || !options.use_second_stage,
        })
    }

    /// No fake successful renderer: camera inversion, placement matrices, shadow
    /// selection, minimum strength and array contracts are not published (OV7/8).
    /// Dynamic snapshots are the only completely specified no-draw path.
    pub fn draw(&mut self, options: FrameOptions, visible_cells: &[[u32; 2]]) -> Result<(), Error> {
        let selection = self.select_frame(options, visible_cells)?;
        if selection.skipped_snapshot {
            return Ok(());
        }
        Err(Error::Gap(ContractGap::DrawingDetailsOv8))
    }
}
