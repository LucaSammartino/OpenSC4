// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{Error, OverlayManager, Unsupported};

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

/// §4.1, including state 4 preserving the existing stage-0 mode.
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
    /// Diagnostic explicit-cell selection, retaining native first-encounter order.
    pub fn select_frame(
        &mut self,
        options: FrameOptions,
        visible_cells: &[[u32; 2]],
    ) -> Result<RenderSelection, Error> {
        self.require_init()?;
        if self.dynamic && options.snapshot {
            return Ok(RenderSelection {
                skipped_snapshot: true,
                decals: Vec::new(),
                rough_shadows: self.shadow_quality < 3 || !options.use_second_stage,
            });
        }
        let candidates: Vec<u32> = if !self.dynamic && self.selection_byte != 0 && self.cell_index.is_some() {
            self.collection_stamp = self.collection_stamp.wrapping_add(1);
            self.collect_cells_at_stamp(visible_cells.iter().map(|c| [c[0] as i32, c[1] as i32]))?
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

    fn collect_cells_at_stamp(&mut self, cells: impl IntoIterator<Item = [i32; 2]>) -> Result<Vec<u32>, Error> {
        let mut selected = Vec::new();
        if let Some(index) = &self.cell_index {
            for cell in cells {
                if let Some(entries) = index.get(&cell) {
                    for &id in entries {
                        // Portable backing check only; no native in-use/shown filter here.
                        let overlay = self
                            .overlays
                            .payload_mut(id)
                            .ok_or(Error::Unsupported(Unsupported::VisibleRows))?;
                        if overlay.stamp != self.collection_stamp {
                            overlay.stamp = self.collection_stamp;
                            selected.push(id);
                        }
                    }
                }
            }
        }
        Ok(selected)
    }

    /// Raw GetDrawRows outputs for the initialized shift-zero index (§4.5).
    /// Invalid backing/conversion is a portable Unsupported boundary.
    pub fn collect_visible_rows(
        &mut self,
        row_begin: i32,
        row_end: i32,
        starts: &[i32],
        ends: &[i32],
    ) -> Result<Vec<u32>, Error> {
        self.require_init()?;
        if self.dynamic || self.selection_byte == 0 || self.cell_index.is_none() {
            return Ok(self.overlays.iter().map(|(id, _)| id).collect());
        }
        self.collection_stamp = self.collection_stamp.wrapping_add(1);
        let end = row_end
            .checked_sub(1)
            .ok_or(Error::Unsupported(Unsupported::VisibleRows))?;
        let mut cells = Vec::new();
        for row in row_begin..end {
            let row = usize::try_from(row).map_err(|_| Error::Unsupported(Unsupported::VisibleRows))?;
            let start = *starts.get(row).ok_or(Error::Unsupported(Unsupported::VisibleRows))?;
            let end = ends
                .get(row)
                .and_then(|v| v.checked_sub(1))
                .ok_or(Error::Unsupported(Unsupported::VisibleRows))?;
            for col in start..end {
                if col < 0 {
                    return Err(Error::Unsupported(Unsupported::VisibleRows));
                }
                cells.push([col, row as i32]);
            }
        }
        self.collect_cells_at_stamp(cells)
    }
}
