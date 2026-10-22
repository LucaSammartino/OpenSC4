// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl LightingManager {
    /// Slot 23, §4.1. This returns the base colour, without global-light scaling.
    pub fn get_color(&self, normal: Vector) -> Result<Color> {
        if self.statics.snapshot().use_maps {
            if let Some(map) = &self.maps[0] {
                let (x, y) = map.normal_pixel(self.map_normal(normal), self.rotation)?;
                return map.pixel_color(x, y);
            }
        }
        let key = dot(normal, self.key).max(0.0);
        let fill = dot(normal, self.fill).max(0.0);
        Ok(add(
            add([0.05; 3], scale([1.0, 1.0, 0.8], key)),
            scale([0.0, 0.1, 0.5], fill),
        )
        .map(|c| c.min(1.0)))
    }
    pub fn get_color_up(&self) -> Result<Color> {
        self.get_color(UP)
    }
    pub fn get_color_bytes(&self, normal: Vector) -> Result<[u8; 3]> {
        self.get_color(normal).map(color_bytes)
    }
    pub fn get_color_up_bytes(&self) -> Result<[u8; 3]> {
        self.get_color_bytes(UP)
    }
    pub fn get_color_at(&self, normal: Vector, position: Vector) -> Result<Color> {
        self.get_color(self.shadowed_normal(normal, position)?)
    }
    pub fn get_color_at_bytes(&self, position: Vector, normal: Vector) -> Result<[u8; 3]> {
        self.get_color_at(normal, position).map(color_bytes)
    }
    pub fn get_model_color(&self, normal: Vector) -> Result<Color> {
        if self.statics.snapshot().use_maps {
            if let Some(surface) = &self.maps[0] {
                let (x, y) = surface.normal_pixel(self.map_normal(normal), self.rotation)?;
                let model = self.maps[1].as_ref().ok_or(LightingError::MissingModelMap)?;
                return model.pixel_color(x, y);
            }
        }
        self.get_color(normal)
    }
    pub fn get_model_light(&self, bounds: Bounds, occupant: Option<Occupant>) -> Result<Color> {
        let p = self.statics.snapshot();
        let mut weight = p.model_weight;
        if let Some(o) = occupant {
            if o.network_flag_40000 {
                return Ok(product(
                    self.global_light,
                    lerp(WHITE, self.get_model_color(UP)?, weight),
                ));
            }
            if o.flora || o.group_5004 {
                weight = p.flora_weight;
            }
            // Literal ordered tests: a mobile interface can override flora (§4.3).
            if o.vehicle || o.pedestrian || o.aircraft || o.watercraft {
                weight = p.mobile_weight;
            }
        }
        let center = bounds.center();
        if p.debug_model {
            return Ok([(center[0] % 256.0) / 256.0, (center[2] % 256.0) / 256.0, 1.0]);
        }
        let normal = if self.shadows_on {
            if self.inside_city(center) {
                self.shadowed_normal(UP, center)?
            } else {
                UP
            }
        } else if bounds.max[0] - bounds.min[0] >= 50.0 || bounds.max[2] - bounds.min[2] >= 50.0 {
            let mut normal = [0.0; 3];
            for x in [bounds.min[0], bounds.max[0]] {
                for z in [bounds.min[2], bounds.max[2]] {
                    normal = add(normal, self.terrain_normal([x, center[1], z])?);
                }
            }
            scale(normal, 0.25)
        } else {
            self.terrain_normal(center)?
        };
        Ok(product(
            self.global_light,
            lerp(WHITE, self.get_model_color(normal)?, weight),
        ))
    }
    pub fn get_terrain_lighting(&self, position: Vector) -> Result<[u8; 3]> {
        let normal = self.shadowed_normal(self.terrain_normal(position)?, position)?;
        Ok(color_bytes(lerp(
            WHITE,
            self.get_color(normal)?,
            self.statics.snapshot().terrain_weight,
        )))
    }
    pub fn get_terrain_color(&self, position: Vector, normal: Vector) -> Result<[u8; 3]> {
        let normal = if self.shadows_on {
            let mut ground = position;
            ground[1] = self.heights[self.location_index(position)?];
            self.shadowed_normal(normal, ground)?
        } else {
            normal
        };
        let k = self.fade_term(position);
        Ok(color_bytes(scale(self.get_color(normal)?, 1.0 - k * k)))
    }
    pub fn get_terrain_color_floor(&self, position: Vector, normal: Vector) -> Result<[u8; 3]> {
        let k = self.fade_term(position);
        Ok(color_bytes(add(
            scale(self.fade_color, k * k),
            product(self.get_color(normal)?, self.scaled_light),
        )))
    }
    pub fn fade_term(&self, position: Vector) -> f32 {
        dot(sub(position, self.fade_origin), self.fade_gradient)
    }

    /// §4.4: input/output byte strides, rows of M, BGRA output. Buffers are
    /// validated before any writes; padding bytes are preserved.
    pub fn get_vertex_colors(
        &self,
        count: usize,
        normals: &[u8],
        normal_stride: usize,
        colors: &mut [u8],
        color_stride: usize,
        matrix: [[f32; 3]; 3],
    ) -> Result<()> {
        check_stream(count, normal_stride, 12, normals.len())?;
        check_stream(count, color_stride, 4, colors.len())?;
        for i in 0..count {
            let n = read_normal(normals, i * normal_stride);
            let transformed = matrix.map(|row| dot(row, n));
            let mut c = [0.0; 3];
            for (axis, value) in transformed.into_iter().enumerate() {
                let index = 2 * axis + usize::from(value < 0.0);
                c = add(c, scale(self.axes[index], value.abs()));
            }
            let [r, g, b] = color_bytes(c.map(|v| v.min(1.0)));
            colors[i * color_stride..i * color_stride + 4].copy_from_slice(&[b, g, r, 255]);
        }
        Ok(())
    }
    /// §4.5: writes R,G,B,R2,G2,B2; no fade at all with shadows off.
    #[allow(clippy::too_many_arguments)]
    pub fn get_terrain_vertex_colors(
        &self,
        j: usize,
        i0: usize,
        count: usize,
        normals: &[u8],
        normal_stride: usize,
        colors: &mut [u8],
        color_stride: usize,
    ) -> Result<()> {
        check_stream(count, normal_stride, 12, normals.len())?;
        check_stream(count, color_stride, 6, colors.len())?;
        if j > self.cells.1 || i0.checked_add(count).map_or(true, |end| end > self.cells.0 + 1) {
            return Err(LightingError::LocationOutsideGrid);
        }
        for t in 0..count {
            let mut n = read_normal(normals, t * normal_stride);
            let mut k2 = 0.0;
            if self.shadows_on {
                let index = self.grid_index(i0 + t, j)?;
                let depth = self.shadows.get(index).ok_or(LightingError::LocationOutsideGrid)?
                    - self.heights.get(index).ok_or(LightingError::LocationOutsideGrid)?;
                // §4.5 says to bend by grid depth, without the positive-depth
                // condition used by the point queries. Zero still normalizes n.
                n = self.bend_shadowed_normal(n, depth);
                let k = self.fade_term([(i0 + t) as f32 * 16.0, 0.0, j as f32 * 16.0]);
                k2 = k * k;
            }
            let c = self.get_color(n)?;
            let a = color_bytes(scale(c, 1.0 - k2));
            let b = color_bytes(add(scale(self.fade_color, k2), product(c, self.scaled_light)));
            colors[t * color_stride..t * color_stride + 3].copy_from_slice(&a);
            colors[t * color_stride + 3..t * color_stride + 6].copy_from_slice(&b);
        }
        Ok(())
    }
    fn shadowed_normal(&self, normal: Vector, position: Vector) -> Result<Vector> {
        if self.shadows_on {
            let d = self.shadow_depth(position)?;
            if d > 0.0 {
                return Ok(self.bend_shadowed_normal(normal, d));
            }
        }
        Ok(normal)
    }
}

fn check_stream(count: usize, stride: usize, size: usize, len: usize) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    if stride < size {
        return Err(LightingError::InvalidStride);
    }
    let end = (count - 1)
        .checked_mul(stride)
        .and_then(|v| v.checked_add(size))
        .ok_or(LightingError::BufferTooShort)?;
    if end > len {
        return Err(LightingError::BufferTooShort);
    }
    Ok(())
}
fn read_normal(bytes: &[u8], offset: usize) -> Vector {
    std::array::from_fn(|i| {
        f32::from_le_bytes(
            bytes[offset + 4 * i..offset + 4 * i + 4]
                .try_into()
                .expect("validated normal stream"),
        )
    })
}
