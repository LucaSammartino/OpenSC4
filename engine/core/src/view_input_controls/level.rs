// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ControlHost, Pixel, Unsupported, Vec3};
use crate::terrain::{Terrain, VertRect, CELL_WIDTH};

/// Typed primitive terrain boundary. The production Terrain implementation is
/// used directly; leveling never calls a host LevelTerrain or brush algorithm.
pub trait TerrainAccess {
    fn cell_width(&self) -> f32;
    fn cell_dimensions(&self) -> (u32, u32);
    fn altitude_at_index(&self, index: u32) -> f32;
    fn set_altitude_rect(&mut self, rect: VertRect, height: f32, redisplay: bool, data4: u32);
}
impl TerrainAccess for Terrain {
    fn cell_width(&self) -> f32 {
        CELL_WIDTH
    }
    fn cell_dimensions(&self) -> (u32, u32) {
        (self.cells_x(), self.cells_z())
    }
    fn altitude_at_index(&self, index: u32) -> f32 {
        Terrain::altitude_at_index(self, index)
    }
    fn set_altitude_rect(&mut self, rect: VertRect, height: f32, redisplay: bool, data4: u32) {
        Terrain::set_altitude_rect(self, rect, height, redisplay, data4);
    }
}
pub trait LevelView: ControlHost {
    fn has_city_view(&self) -> bool;
    fn pick_city_terrain(&mut self, at: Pixel) -> Option<Vec3>;
    fn automatic_cell_cursor(&self) -> bool;
    fn set_automatic_cell_cursor(&mut self, on: bool);
    fn mark_selected(&mut self, cells: VertRect, mode: u32, on: bool);
    fn clear_selection(&mut self);
}
#[derive(Debug)]
pub struct LevelTerrain {
    width: f32,
    dragging: bool,
    start: (i32, i32),
    end: (i32, i32),
    old_cursor: bool,
}
impl LevelTerrain {
    pub const ID: u32 = 0x88CE_A620;
    pub const CURSOR: u32 = 0x071E_5703;
    pub fn new(terrain: &impl TerrainAccess) -> Result<Self, Unsupported> {
        let width = terrain.cell_width();
        if !width.is_finite() || width <= 0.0 {
            return Err(Unsupported::PrimitiveData);
        }
        Ok(Self {
            width,
            dragging: false,
            start: (0, 0),
            end: (0, 0),
            old_cursor: false,
        })
    }
    pub fn dragging(&self) -> bool {
        self.dragging
    }
    /// Local supported Init work. Terrain/view reference acquisition belongs
    /// to a future adapter; this core does not implement the base COM lifetime.
    pub fn init(&self, host: &mut impl LevelView) {
        super::set_cursor(host, Self::CURSOR);
    }
    pub fn key_down(&self, _key: u32, _modifiers: u32) -> bool {
        false
    }
    pub fn mouse_down(&mut self, host: &mut impl LevelView, at: Pixel) -> bool {
        if !host.has_city_view() {
            return false;
        }
        if let Some(p) = host.pick_city_terrain(at) {
            self.dragging = true;
            host.capture();
            self.start = ((p.x / self.width) as i32, (p.z / self.width) as i32);
            self.end = self.start;
            self.old_cursor = host.automatic_cell_cursor();
            host.set_automatic_cell_cursor(false);
        }
        true
    }
    pub fn mouse_move(&mut self, host: &mut impl LevelView, at: Pixel) -> bool {
        if !self.dragging {
            return false;
        }
        if let Some(p) = host.pick_city_terrain(at) {
            let reciprocal = 1.0 / self.width;
            let cell = ((p.x * reciprocal) as i32, (p.z * reciprocal) as i32);
            if cell != self.end {
                self.end = cell;
                self.compute_and_mark(host);
            }
        }
        true
    }
    pub fn selected_area(&self) -> VertRect {
        VertRect::new(
            self.start.0.min(self.end.0),
            self.start.1.min(self.end.1),
            self.start.0.max(self.end.0),
            self.start.1.max(self.end.1),
        )
    }
    pub fn compute_and_mark(&self, host: &mut impl LevelView) {
        host.mark_selected(self.selected_area(), 2, true);
    }
    pub fn level(&self, terrain: &mut impl TerrainAccess) -> Result<(), Unsupported> {
        let (cx, cz) = terrain.cell_dimensions();
        // VIC3: picks outside the city have no specified vertex-index contract.
        if self.start.0 < 0
            || self.start.1 < 0
            || self.end.0 < 0
            || self.end.1 < 0
            || self.start.0 as u32 > cx
            || self.start.1 as u32 > cz
            || self.end.0 as u32 > cx
            || self.end.1 as u32 > cz
            || cx > i32::MAX as u32 - 1
            || cz > i32::MAX as u32 - 1
        {
            return Err(Unsupported::PrimitiveData);
        }
        let index = (cx + 1)
            .checked_mul(self.start.1 as u32)
            .and_then(|n| n.checked_add(self.start.0 as u32))
            .ok_or(Unsupported::PrimitiveData)?;
        let height = terrain.altitude_at_index(index);
        let mut rect = self.selected_area();
        rect.x1 = (rect.x1 + 1).min(cx as i32);
        rect.z1 = (rect.z1 + 1).min(cz as i32);
        terrain.set_altitude_rect(rect, height, true, 1);
        Ok(())
    }
    pub fn mouse_up(
        &mut self,
        host: &mut impl LevelView,
        terrain: &mut impl TerrainAccess,
        at: Pixel,
    ) -> Result<bool, Unsupported> {
        if !self.dragging {
            return Ok(false);
        }
        if let Some(p) = host.pick_city_terrain(at) {
            self.end = ((p.x / self.width) as i32, (p.z / self.width) as i32);
        }
        self.compute_and_mark(host);
        let result = self.level(terrain);
        host.clear_selection();
        host.set_automatic_cell_cursor(self.old_cursor);
        host.release_capture();
        self.dragging = false;
        result.map(|()| true)
    }
}
