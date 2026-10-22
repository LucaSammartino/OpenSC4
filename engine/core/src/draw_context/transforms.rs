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
        if model.is_none() && self.model_view_valid {
            return Ok(());
        }
        let view = if self.model_view_valid {
            self.composed
        } else {
            self.camera.as_ref().ok_or(Error::MissingCamera)?.view_transform()?
        };
        let composed = if let Some(model) = model {
            self.model_transform = model;
            view.followed_by(model)
        } else {
            view
        };
        // Invalid until the matrix has actually reached the device (DC6).
        self.model_view_valid = false;
        let matrix = composed.matrix();
        self.emit(Command::LoadMatrix(matrix))?;
        self.composed = composed;
        self.matrix = matrix;
        self.model_view_valid = model.is_none();
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
        let color = std::array::from_fn(|index| rgb[index] * self.default_shade[index]);
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
        let light = self
            .lighting
            .as_ref()
            .ok_or(Error::MissingLighting)?
            .model_light(instance)?;
        let color = if let Some(tint) = tint {
            std::array::from_fn(|index| light[index] * tint[index])
        } else {
            light
        };
        self.emit(Command::ColorMultiplier(color))?;
        // DC8: section 8 specifies this driver multiplier only; it does not
        // publish a write to default_shade or the shade-override flag here.
        Ok(())
    }
}
