// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Typed payloads for all seven messages in §6; raw pointer/clock/view decoding
/// belongs to adapters. They supply external data, never lighting calculations.
pub enum LightingMessage {
    ViewChanged {
        zoom: u8,
        rotation: u8,
    },
    CityStarted {
        terrain: Arc<dyn Terrain>,
        city_terrain: Option<Arc<dyn Terrain>>,
        zoom: u8,
        rotation: u8,
    },
    TerrainChanged(Rect),
    LongOperationBegin,
    LongOperationEnd,
    ExemplarChanged,
    RenderPropertiesChanged {
        threshold: f32,
        shadow_setting: u32,
        zoom: u8,
        rotation: u8,
    },
}

impl LightingManager {
    pub fn do_message(&mut self, message: LightingMessage, events: &mut dyn EventSink) -> Result<()> {
        match message {
            LightingMessage::ViewChanged { zoom, rotation } => self.do_zoom_and_rotation_change(zoom, rotation)?,
            LightingMessage::CityStarted {
                terrain,
                city_terrain,
                zoom,
                rotation,
            } => {
                let cells = terrain.cell_counts();
                let count = cells
                    .0
                    .checked_add(1)
                    .and_then(|x| cells.1.checked_add(1).and_then(|z| x.checked_mul(z)))
                    .ok_or(LightingError::InvalidDimensions)?;
                if cells.0 == 0 || cells.1 == 0 || cells.0 > i32::MAX as usize - 1 || cells.1 > i32::MAX as usize - 1 {
                    return Err(LightingError::InvalidDimensions);
                }
                let mut heights = Vec::new();
                heights
                    .try_reserve_exact(count)
                    .map_err(|_| LightingError::InvalidDimensions)?;
                heights.resize(count, 0.0);
                let mut shadows = Vec::new();
                shadows
                    .try_reserve_exact(count)
                    .map_err(|_| LightingError::InvalidDimensions)?;
                shadows.resize(count, 0.0);
                self.cells = cells;
                self.terrain = Some(terrain);
                self.city_terrain = city_terrain;
                if self.city_terrain.is_some() {
                    self.long_operation = false;
                }
                self.heights = heights;
                self.shadows = shadows;
                self.do_zoom_and_rotation_change(zoom, rotation)?;
            }
            LightingMessage::TerrainChanged(rect) => {
                if self.city_terrain.is_some() && self.shadows_on {
                    if self.long_operation {
                        self.dirty.union(rect);
                    } else {
                        self.update_terrain(rect, false, events)?;
                    }
                }
            }
            LightingMessage::LongOperationBegin => {
                self.long_operation = true;
                self.dirty = Rect::EMPTY;
            }
            LightingMessage::LongOperationEnd => {
                self.long_operation = false;
                if self.shadows_on && !self.dirty.is_empty() {
                    self.update_terrain(self.dirty, true, events)?;
                }
            }
            LightingMessage::ExemplarChanged => self.read_properties(),
            LightingMessage::RenderPropertiesChanged {
                threshold,
                shadow_setting,
                zoom,
                rotation,
            } => {
                self.statics.update(|p| p.change_threshold = threshold);
                let enabled = shadow_setting >= 2;
                if enabled != self.shadows_on {
                    self.shadows_on = enabled;
                    self.do_zoom_and_rotation_change(zoom, rotation)?;
                    events.emit(Event::Fractions {
                        day: self.day_fraction,
                        night: self.night_fraction,
                    });
                    events.emit(Event::Redraw);
                }
            }
        }
        Ok(())
    }
    pub fn do_zoom_and_rotation_change(&mut self, zoom: u8, rotation: u8) -> Result<()> {
        self.rotation = rotation % 4;
        self.map_rotation_initialized = true;
        self.zoom = zoom;
        self.key = rotate(normalize([-1.0, 1.0, 1.0]), rotation);
        self.fill = rotate(normalize([1.0, 0.5, -1.0]), rotation);
        self.shadow_direction = rotate(self.sun, rotation);
        let (w, d) = self.city_size();
        self.fade_origin = add(rotate([-w / 2.0, 0.0, d / 2.0], rotation), [w / 2.0, 0.0, d / 2.0]);
        // LT5: safe pre-city policy, NOT a native zero-gradient default.
        self.fade_gradient = if d == 0.0 {
            [0.0; 3]
        } else {
            rotate(
                [0.0, 0.0, -self.fade_strength / (2.0_f32.powi(i32::from(zoom)) * d)],
                rotation,
            )
        };
        if self.terrain.is_some() && self.shadows_on {
            self.copy_heights(Rect {
                min_x: 0,
                min_z: 0,
                max_x: self.cells.0 as i32,
                max_z: self.cells.1 as i32,
            })?;
            self.find_hill_shadows()?;
        }
        self.view_applied = true;
        // §3: deliberately use UP twice, including the -y cache entry.
        for (i, normal) in [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            UP,
            UP,
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ]
        .into_iter()
        .enumerate()
        {
            self.axes[i] = self.get_model_color(normal)?;
        }
        Ok(())
    }
    fn city_size(&self) -> (f32, f32) {
        (self.cells.0 as f32 * 16.0, self.cells.1 as f32 * 16.0)
    }
    pub(crate) fn inside_city(&self, p: Vector) -> bool {
        let (w, d) = self.city_size();
        p[0] >= 0.0 && p[2] >= 0.0 && p[0] < w && p[2] < d
    }
    pub(crate) fn grid_index(&self, i: usize, j: usize) -> Result<usize> {
        let (x, z) = (self.cells.0 + 1, self.cells.1 + 1);
        if i >= x || j >= z {
            return Err(LightingError::LocationOutsideGrid);
        }
        Ok(match self.rotation {
            0 => i * z + j,
            1 => j * x + (x - 1 - i),
            2 => (x - 1 - i) * z + (z - 1 - j),
            _ => (z - 1 - j) * x + i,
        })
    }
    pub(crate) fn location_index(&self, p: Vector) -> Result<usize> {
        if !p[0].is_finite() || !p[2].is_finite() || p[0] < 0.0 || p[2] < 0.0 {
            return Err(LightingError::LocationOutsideGrid);
        }
        let index = self.grid_index((p[0] / 16.0).floor() as usize, (p[2] / 16.0).floor() as usize)?;
        if index >= self.heights.len() || index >= self.shadows.len() {
            return Err(LightingError::LocationOutsideGrid);
        }
        Ok(index)
    }
    fn copy_heights(&mut self, rect: Rect) -> Result<()> {
        let terrain = self.terrain.as_ref().ok_or(LightingError::NoTerrain)?;
        if rect.is_empty() {
            return Ok(());
        }
        if rect.max_x as usize > self.cells.0 || rect.max_z as usize > self.cells.1 {
            return Err(LightingError::LocationOutsideGrid);
        }
        for i in rect.min_x..=rect.max_x {
            for j in rect.min_z..=rect.max_z {
                let index = self.grid_index(i as usize, j as usize)?;
                self.heights[index] = terrain.altitude(i as usize, j as usize);
            }
        }
        Ok(())
    }
    pub fn find_hill_shadows(&mut self) -> Result<()> {
        let [sx, sy, sz] = self.sun;
        if sx < 0.0 {
            return Ok(());
        }
        let r = |x| register::Number::new(x, self.register_precision);
        let drift = ((r(-1.0) / r(sx)) * r(sz)).store();
        if !(-1.0..=1.0).contains(&drift) {
            return Ok(());
        }
        let width = self.cells.1 + 1;
        let rows = self.cells.0 + 1;
        if self.heights.len() != width * rows || self.shadows.len() != width * rows {
            return Err(LightingError::LocationOutsideGrid);
        }
        let drop = (((r(16.0) / r(sx)) * r(sy)) * r(self.statics.snapshot().slope_factor)).store();
        let mut running = vec![0.0_f32; width];
        self.changed = Rect {
            min_x: -1,
            min_z: -1,
            max_x: 0,
            max_z: 0,
        };
        for row in 0..rows {
            for (column, h) in running.iter_mut().enumerate() {
                let index = row * width + column;
                *h = h.max(self.heights[index]);
                if self.shadows[index] != *h {
                    self.shadows[index] = *h;
                    self.changed.include(column as i32, row as i32);
                }
                *h += drop;
            }
            if drift <= 0.0 {
                for c in (1..width).rev() {
                    running[c] -= (running[c - 1] - running[c]) * drift;
                }
            } else {
                for c in 0..width - 1 {
                    running[c] += (running[c + 1] - running[c]) * drift;
                }
            }
        }
        Ok(())
    }
    pub fn changed_shadow_rect(&self) -> Rect {
        self.changed
    }
    pub fn shadow_depth(&self, p: Vector) -> Result<f32> {
        Ok(self.shadows[self.location_index(p)?] - p[1])
    }
    pub fn height_above_land(&self, p: Vector) -> Result<f32> {
        Ok(p[1] - self.heights[self.location_index(p)?])
    }
    pub fn bend_shadowed_normal(&self, n: Vector, depth: f32) -> Vector {
        let factor = (f64::from(self.statics.snapshot().normal_bend) * f64::from(depth) * 0.01) as f32;
        normalize(add(n, scale(self.fill, factor)))
    }
    pub fn get_shadow_strength_from_location(&self, xz: [f32; 2]) -> Result<f32> {
        let index = self.location_index([xz[0], 0.0, xz[1]])?;
        let depth = self.shadows[index] - self.heights[index];
        let p = self.statics.snapshot();
        if depth > p.depth_fade {
            return Ok(0.0);
        }
        let strength = 1.0 - p.night_reduction * self.night_fraction;
        Ok(if depth > 0.0 {
            strength * (1.0 - depth / p.depth_fade)
        } else {
            strength
        })
    }
    pub fn get_shadow_params(&self) -> (f32, f32, Color) {
        let p = self.statics.snapshot().decal;
        (p.a, p.b, p.color)
    }
    pub fn get_decal_shadow_params(&self, holder: &dyn PropertyHolder) -> DecalShadowParams {
        self.statics.snapshot().decal.overridden(holder)
    }
    pub fn terrain_normal(&self, p: Vector) -> Result<Vector> {
        let terrain = self.terrain.as_ref().ok_or(LightingError::NoTerrain)?;
        let (w, d) = self.city_size();
        let r = |x| register::Number::new(x, self.register_precision);
        let clamp = |coordinate: f32, extent: f32| {
            let upper = r(extent) - register::Number::double(0.001, self.register_precision);
            if coordinate < 0.0 {
                0.0
            } else if r(coordinate) > upper {
                upper.store()
            } else {
                coordinate
            }
        };
        Ok(terrain.normal(clamp(p[0], w), clamp(p[2], d)))
    }
    fn update_terrain(&mut self, rect: Rect, redisplay: bool, events: &mut dyn EventSink) -> Result<()> {
        let terrain = self.city_terrain.as_ref().ok_or(LightingError::NoTerrain)?;
        if !terrain.initialized() {
            return Ok(());
        }
        self.copy_heights(rect)?;
        self.find_hill_shadows()?;
        if !self.changed.is_empty() {
            // LT9: literal inverse of §3's indexing, inclusive vertex endpoints.
            // No invented expansion to adjacent cells.
            let world = self.changed_world_rect();
            events.emit(Event::TerrainLightingChanged(world));
            if redisplay {
                events.emit(Event::RedisplayTerrain(world));
            }
        }
        Ok(())
    }
    pub(super) fn changed_world_rect(&self) -> Rect {
        let mut world = Rect::EMPTY;
        let (x, z) = (self.cells.0 as i32, self.cells.1 as i32);
        for c in [self.changed.min_x, self.changed.max_x] {
            for r in [self.changed.min_z, self.changed.max_z] {
                let (i, j) = match self.rotation {
                    0 => (r, c),
                    1 => (x - c, r),
                    2 => (x - r, z - c),
                    _ => (c, z - r),
                };
                world.include(i, j);
            }
        }
        world
    }
}
