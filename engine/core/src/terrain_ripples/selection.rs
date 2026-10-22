// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{dimensions, vertex_index, Cell, CellRect, ContractGap, Error, TerrainProvider, TextureFactory};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionRenderProperties {
    pub prefer_opaque: bool,
    pub no_add_signed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub rgb: [f32; 3],
    pub alpha: f32,
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexturedCell {
    pub key: u32,
    pub b1: u8,
    pub b2: u8,
    /// The packed form has no b3 input; its stored byte is unpublished (WR9).
    /// None is an explicit unknown, never silently substituted with zero.
    pub b3: Option<u8>,
    pub material: Material,
}

/// §2.1 two mutually exclusive vertex-indexed tables. BTreeMap order is only
/// deterministic storage, not claimed native tile submission order (WR7).
#[derive(Default)]
pub struct SelectedTerrainArea {
    terrain: Option<Arc<dyn TerrainProvider>>,
    factory: Option<Arc<dyn TextureFactory>>,
    counts: [u32; 2],
    plain: BTreeMap<u32, u32>,
    textured: BTreeMap<u32, TexturedCell>,
    state_preference: Option<bool>,
    render_states_dirty: bool,
}

impl SelectedTerrainArea {
    pub fn init(
        &mut self,
        terrain: Arc<dyn TerrainProvider>,
        factory: Arc<dyn TextureFactory>,
        properties: SelectionRenderProperties,
    ) -> Result<(), Error> {
        let counts = dimensions(terrain.as_ref())?;
        self.shutdown();
        self.counts = counts;
        self.terrain = Some(terrain);
        self.factory = Some(factory);
        self.state_preference = Some(properties.prefer_opaque);
        // §2.3 requires four states; their actual parameters are missing (WR7).
        // Resource/state initialization is intentionally not claimed complete.
        self.render_states_dirty = true;
        Ok(())
    }

    pub fn plain_cells(&self) -> &BTreeMap<u32, u32> {
        &self.plain
    }
    pub fn textured_cells(&self) -> &BTreeMap<u32, TexturedCell> {
        &self.textured
    }
    pub fn render_states_dirty(&self) -> bool {
        self.render_states_dirty
    }
    pub fn state_preference(&self) -> Option<bool> {
        self.state_preference
    }
    pub fn is_initialized(&self) -> bool {
        self.terrain.is_some()
    }

    fn index(&self, cell: Cell) -> Result<u32, Error> {
        if self.terrain.is_none() {
            return Err(Error::NotInitialized);
        }
        vertex_index(cell, self.counts)
    }

    fn put_plain(&mut self, index: u32, colour: u32) {
        self.textured.remove(&index);
        self.plain.insert(index, colour);
    }

    fn put_textured(&mut self, index: u32, cell: TexturedCell) {
        self.plain.remove(&index);
        self.textured.insert(index, cell);
    }

    /// §2.2 list form. Validate all input before mutations: Rust boundary policy
    /// for invalid cells, whose native behavior is not specified (WR9).
    pub fn mark_list(&mut self, cells: &[Cell], colour: u32, clear: bool) -> Result<(), Error> {
        if self.terrain.is_none() {
            return Err(Error::NotInitialized);
        }
        let indices: Vec<_> = cells.iter().map(|c| self.index(*c)).collect::<Result<_, _>>()?;
        if clear {
            self.clear_current_selections();
        }
        for index in indices {
            self.put_plain(index, colour);
        }
        Ok(())
    }

    pub fn mark_rectangle(
        &mut self,
        rect: CellRect,
        colour: u32,
        clear: bool,
        properties: SelectionRenderProperties,
    ) -> Result<(), Error> {
        self.index(rect.min)?;
        self.index(rect.max)?;
        CellRect::new(rect.min, rect.max)?;
        if clear {
            self.clear_current_selections();
        }
        // Literal rectangle-specific property refresh, regardless of clear; WR7.
        self.refresh_render_properties(properties);
        for z in rect.min.z..=rect.max.z {
            for x in rect.min.x..=rect.max.x {
                let index = vertex_index(Cell { x, z }, self.counts)?;
                self.put_plain(index, colour);
            }
        }
        Ok(())
    }

    pub fn mark_cell(&mut self, cell: Cell, colour: u32, clear: bool) -> Result<(), Error> {
        let index = self.index(cell)?;
        if clear {
            self.clear_current_selections();
        }
        self.put_plain(index, colour);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // The published overload has seven inputs.
    pub fn mark_palette(
        &mut self,
        cell: Cell,
        key: u32,
        b1: u8,
        b2: u8,
        palette: u32,
        b3: u8,
        clear: bool,
    ) -> Result<(), Error> {
        let index = self.index(cell)?;
        let rgb = match palette {
            0 => [1.0, 0.0, 0.0],
            1 => [0.0, 1.0, 0.0],
            2 => [0.0, 0.0, 1.0],
            3 => [0.4, 0.3, 0.0],
            4 => [0.0; 3],
            // Native reads unrelated constants; don't invent a clamped palette.
            _ => return Err(Error::Gap(ContractGap::PaletteOutsideTableWr9)),
        };
        if clear {
            self.clear_current_selections();
        }
        self.put_textured(
            index,
            TexturedCell {
                key,
                b1,
                b2,
                b3: Some(b3),
                material: Material {
                    rgb,
                    alpha: 1.0,
                    mode: 2,
                },
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mark_packed(
        &mut self,
        cell: Cell,
        key: u32,
        b1: u8,
        b2: u8,
        colour: u32,
        mode: u32,
        clear: bool,
        properties: SelectionRenderProperties,
    ) -> Result<(), Error> {
        let index = self.index(cell)?;
        // §2.2 is 0xRRGGBBxx: the low byte is not alpha.
        let rgb = [(colour >> 24) as u8, (colour >> 16) as u8, (colour >> 8) as u8].map(|v| f32::from(v) / 255.0);
        let (mode, alpha) = if mode == 3 && properties.no_add_signed {
            (1, 0.5)
        } else {
            (mode, 1.0)
        };
        // Literal alpha 1 outside the published fallback; confirmation WR9.
        if clear {
            self.clear_current_selections();
        }
        self.put_textured(
            index,
            TexturedCell {
                key,
                b1,
                b2,
                b3: None,
                material: Material { rgb, alpha, mode },
            },
        );
        Ok(())
    }

    pub fn clear_current_selections(&mut self) {
        self.plain.clear();
        self.textured.clear();
    }

    pub fn refresh_render_properties(&mut self, properties: SelectionRenderProperties) {
        if self.state_preference != Some(properties.prefer_opaque) {
            self.state_preference = Some(properties.prefer_opaque);
            self.render_states_dirty = true;
        }
    }

    /// §2.3 empty updates do nothing. Nonempty draw cannot be replaced with a
    /// callback asking the host to draw an original-owner selection; WR7 blocks.
    pub fn update(&mut self, properties: SelectionRenderProperties) -> Result<(), Error> {
        if !self.is_initialized() {
            return Err(Error::NotInitialized);
        }
        if self.plain.is_empty() && self.textured.is_empty() {
            return Ok(());
        }
        self.refresh_render_properties(properties);
        Err(Error::Gap(ContractGap::SelectionGeometryWr7))
    }

    pub fn shutdown(&mut self) {
        self.clear_current_selections();
        self.terrain = None;
        self.factory = None;
        self.counts = [0; 2];
        self.state_preference = None;
        self.render_states_dirty = false;
    }
}
