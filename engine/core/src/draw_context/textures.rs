// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl<D: Driver> DrawContext<D> {
    pub fn set_texture(&mut self, texture: Option<TextureRef>, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        if self.reaches_driver(stage) {
            self.select(stage)?;
            self.emit(Command::Texture(texture.clone()))?;
        }
        self.stages[stage].texture = texture;
        Ok(())
    }

    pub fn set_tex_coord(&mut self, set: u32, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        if self.reaches_driver(stage) {
            self.select(stage)?;
            if self.stages[stage].coordinate_set != Some(set) {
                self.emit(Command::TexCoord(set))?;
            }
        }
        self.stages[stage].coordinate_set = Some(set);
        Ok(())
    }

    pub fn set_tex_wrap(&mut self, s: u8, t: u8, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        let requested = if self.properties.force_repeat { [3, 3] } else { [s, t] };
        if self.reaches_driver(stage) {
            self.select(stage)?;
        }
        for (index, value) in requested.into_iter().enumerate() {
            if self.stages[stage].wrap[index] != Some(value) {
                if self.reaches_driver(stage) {
                    self.emit(Command::TexParameter {
                        parameter: index as u8 + 2,
                        value,
                    })?;
                }
                self.stages[stage].wrap[index] = Some(value);
            }
        }
        Ok(())
    }

    pub fn set_tex_filter(&mut self, mag: u8, min: u8, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        let requested = if self.properties.force_point {
            [0, 0]
        } else if self.properties.force_bilinear {
            [1, 7]
        } else {
            [mag, min]
        };
        if self.reaches_driver(stage) {
            self.select(stage)?;
        }
        for (index, value) in requested.into_iter().enumerate() {
            if self.stages[stage].filter[index] != Some(value) {
                if self.reaches_driver(stage) {
                    self.emit(Command::TexParameter {
                        parameter: index as u8,
                        value,
                    })?;
                }
                self.stages[stage].filter[index] = Some(value);
            }
        }
        Ok(())
    }

    pub fn set_tex_transform(&mut self, matrix: Matrix, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        self.set_tex_coord(16, stage)?;
        if self.reaches_driver(stage) {
            self.select(stage)?;
            self.emit(Command::TexMatrix(Some(matrix)))?;
        }
        self.stages[stage].transform = Some(matrix);
        Ok(())
    }

    pub fn clear_tex_transform(&mut self, stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        if self.stages[stage].transform.is_some() {
            self.set_tex_coord(0, stage)?;
            if self.reaches_driver(stage) {
                self.select(stage)?;
                self.emit(Command::TexMatrix(None))?;
            }
            self.stages[stage].transform = None;
        }
        Ok(())
    }

    pub fn set_tex_env_mode(&mut self, mode: i32, stage: usize) -> Result<(), Error> {
        self.ready()?;
        if mode == -1 {
            return Ok(());
        }
        if stage > 1 {
            return Err(Error::InvalidStage);
        }
        if !(0..8).contains(&mode) {
            return Err(Error::InvalidValue);
        }
        let shift = if stage == 0 { 25 } else { 29 };
        if self.reaches_driver(stage) {
            self.select(stage)?;
        }
        if (self.word >> shift) & 7 != mode as u32 {
            if self.reaches_driver(stage) {
                self.emit(Command::TexEnv(mode as u8))?;
            }
            self.word = (self.word & !(7 << shift)) | ((mode as u32) << shift);
        }
        Ok(())
    }

    /// Arbitrary caller-supplied driver records are opaque. This does not stand
    /// in for the incomplete, owned default/highlight records (DC4).
    pub fn set_tex_combiner(&mut self, record: &[u8], stage: usize) -> Result<(), Error> {
        self.stage(stage)?;
        if stage > 1 {
            return Err(Error::InvalidStage);
        }
        if self.reaches_driver(stage) {
            self.select(stage)?;
            self.emit(Command::Combiner(record.to_vec()))?;
        }
        let shift = if stage == 0 { 25 } else { 29 };
        self.word = (self.word & !(7 << shift)) | (4 << shift);
        Ok(())
    }

    pub fn set_tex_color(&mut self, color: Rgba) -> Result<(), Error> {
        self.stage(0)?;
        self.select(0)?;
        self.emit(Command::TexColor(color))
    }

    pub fn prepare_second_pass(&mut self) -> Result<(), Error> {
        self.ready()?;
        if self.word & (1 << 28) == 0 {
            return Ok(());
        }
        let stage = self.stages[1].clone();
        // DC5: copying unknown defaults cannot fabricate effective values.
        let coord = stage.coordinate_set.ok_or(Error::Gap(ContractGap::StageDefaults))?;
        let s = stage.wrap[0].ok_or(Error::Gap(ContractGap::StageDefaults))?;
        let t = stage.wrap[1].ok_or(Error::Gap(ContractGap::StageDefaults))?;
        let mag = stage.filter[0].ok_or(Error::Gap(ContractGap::StageDefaults))?;
        let min = stage.filter[1].ok_or(Error::Gap(ContractGap::StageDefaults))?;
        self.set_texture(stage.texture, 0)?;
        self.set_tex_coord(coord, 0)?;
        self.set_tex_wrap(s, t, 0)?;
        self.set_tex_filter(mag, min, 0)?;
        if let Some(matrix) = stage.transform {
            self.set_tex_transform(matrix, 0)?;
        } else {
            self.clear_tex_transform(0)?;
        }
        self.set_depth_func(2)?;
        self.enable_alpha_test(0)?;
        match (self.word >> 29) & 7 {
            0 => {
                self.set_tex_env_mode(0, 0)?;
                self.enable_blend(0)?;
            }
            1 => {
                self.set_blend_func(0, 2)?;
                self.enable_blend(1)?;
            }
            2 => {
                self.set_tex_env_mode(0, 0)?;
                self.set_blend_func(4, 5)?;
                self.enable_blend(1)?;
            }
            _ => {}
        }
        Ok(())
    }
}
