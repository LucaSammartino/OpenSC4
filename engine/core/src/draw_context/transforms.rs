// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl<D: Driver> DrawContext<D> {
    fn load_projection(&mut self) -> Result<(), Error> {
        let projection = self.camera.as_ref().ok_or(Error::MissingCamera)?.projection()?;
        let mut transposed = [0.0; 16];
        for row in 0..4 {
            for col in 0..4 {
                transposed[col * 4 + row] = projection[row][col];
            }
        }
        self.emit(Command::MatrixMode(1))?;
        self.emit(Command::LoadMatrix(transposed))?;
        self.emit(Command::MatrixMode(0))
    }

    pub fn set_camera(&mut self, camera: Arc<dyn Camera>) -> Result<(), Error> {
        self.ready()?;
        self.camera = Some(camera);
        self.model_view_valid = false;
        self.load_projection()?;
        self.set_model_transform(None)
    }

    pub fn set_model_view_transform_changed(&mut self) {
        self.model_view_valid = false;
    }

    pub fn set_model_transform(&mut self, model: Option<Transform>) -> Result<(), Error> {
        self.ready()?;
        self.qualified_numeric()?;
        if model.is_none() && self.model_view_valid {
            return Ok(());
        }
        if !self.model_view_valid {
            self.composed = self.camera.as_ref().ok_or(Error::MissingCamera)?.view_transform()?;
            self.composed_counter = self.composed_counter.wrapping_add(1);
        }
        if let Some(model) = model {
            self.model_counter = self.model_counter.wrapping_add(1);
            self.model_transform = model;
            if model.flags != 0 {
                self.composed = self.composed.followed_by(model)?;
                self.composed_counter = self.composed_counter.wrapping_add(1);
            }
        }
        // Complete owner stores precede the driver. No MatrixMode call here.
        self.matrix = self.composed.qualified_matrix()?;
        self.matrix_loaded = true;
        self.emit(Command::LoadMatrix(self.matrix))?;
        self.model_view_valid = model.is_none();
        Ok(())
    }

    /// Distinct raw-matrix overload, section 8.4; no compact scale or counters.
    pub fn set_model_matrix(&mut self, input: Matrix) -> Result<(), Error> {
        use super::numeric::{add, mul, store};
        self.ready()?;
        self.qualified_numeric()?;
        if !self.model_view_valid {
            self.composed = self.camera.as_ref().ok_or(Error::MissingCamera)?.view_transform()?;
            self.composed_counter = self.composed_counter.wrapping_add(1);
        }
        let mut matrix = [0.0; 16];
        for row in 0..3 {
            for col in 0..4 {
                let p = [
                    mul(f64::from(self.composed.rotation[row][0]), f64::from(input[col]))?,
                    mul(f64::from(self.composed.rotation[row][1]), f64::from(input[4 + col]))?,
                    mul(f64::from(self.composed.rotation[row][2]), f64::from(input[8 + col]))?,
                    mul(f64::from(self.composed.translation[row]), f64::from(input[12 + col]))?,
                ];
                let sum = if col == 0 {
                    add(add(add(p[1], p[3])?, p[2])?, p[0])?
                } else {
                    add(add(add(p[2], p[1])?, p[0])?, p[3])?
                };
                matrix[4 * col + row] = store(sum)?;
            }
        }
        for col in 0..4 {
            let sum = if col == 0 {
                add(add(f64::from(input[4]), f64::from(input[0]))?, f64::from(input[8]))?
            } else {
                add(
                    add(f64::from(input[col]), f64::from(input[8 + col]))?,
                    f64::from(input[4 + col]),
                )?
            };
            // Keep explicit zero multiplication; nonfinite input is outside the
            // qualified domain, never silently simplified to input[12+col].
            matrix[4 * col + 3] = store(add(mul(sum, 0.0)?, f64::from(input[12 + col]))?)?;
        }
        self.matrix = matrix;
        self.matrix_loaded = true;
        self.emit(Command::LoadMatrix(self.matrix))?;
        self.model_view_valid = false;
        Ok(())
    }

    pub fn set_depth_offset(&mut self, offset: i32) -> Result<(), Error> {
        self.ready()?;
        if self.depth_offset == offset {
            return Ok(());
        }
        let bias = self
            .properties
            .depth_offset
            .checked_mul(offset)
            .ok_or(Error::InvalidValue)?;
        self.camera.as_ref().ok_or(Error::MissingCamera)?.set_depth_bias(bias)?;
        self.load_projection()?;
        self.depth_offset = offset;
        Ok(())
    }

    pub fn set_shade(&mut self, rgb: Rgb) -> Result<(), Error> {
        self.ready()?;
        self.qualified_numeric()?;
        let mut color = [0.0; 3];
        for (index, value) in color.iter_mut().enumerate() {
            *value = numeric::store(numeric::mul(
                f64::from(rgb[index]),
                f64::from(self.default_shade[index]),
            )?)?;
        }
        self.set_self_lit_shade(color)
    }

    pub fn set_self_lit_shade(&mut self, rgb: Rgb) -> Result<(), Error> {
        self.ready()?;
        self.emit(Command::ColorMultiplier(rgb))?;
        self.shade_override = true;
        Ok(())
    }

    pub fn reset_shade(&mut self) -> Result<(), Error> {
        self.ready()?;
        if self.shade_override {
            self.emit(Command::ColorMultiplier(self.default_shade))?;
            self.shade_override = false;
        }
        Ok(())
    }

    pub fn set_model_shade(&mut self, instance: u64, tint: Option<Rgb>) -> Result<(), Error> {
        self.ready()?;
        let light = match self.lighting.as_ref() {
            Some(manager) => manager.checked_model_light(instance)?.unwrap_or(self.default_shade),
            None => self.default_shade,
        };
        let color = if let Some(tint) = tint {
            self.qualified_numeric()?;
            let mut color = [0.0; 3];
            for (index, value) in color.iter_mut().enumerate() {
                *value = numeric::store(numeric::mul(f64::from(light[index]), f64::from(tint[index]))?)?;
            }
            color
        } else {
            light
        };
        self.emit(Command::ColorMultiplier(color))?;
        self.shade_override = true;
        Ok(())
    }

    pub fn set_lighting_enabled(&mut self, enabled: bool) -> Result<(), Error> {
        self.turn_on_lighting(u8::from(enabled))
    }

    pub fn turn_on_lighting(&mut self, enabled: u8) -> Result<(), Error> {
        self.ready()?;
        if enabled == self.lighting_enabled {
            return Ok(());
        }
        let color = if enabled != 0 {
            if let Some(manager) = self.lighting.as_ref() {
                self.default_shade = manager.default_light()?;
            }
            self.default_shade
        } else {
            [1.0; 3]
        };
        self.emit(Command::ColorMultiplier(color))?;
        self.lighting_enabled = enabled;
        Ok(())
    }
}
