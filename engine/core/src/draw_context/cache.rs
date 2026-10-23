// SPDX-License-Identifier: GPL-3.0-or-later
use super::state::quantize;
use super::*;

impl<D: Driver> DrawContext<D> {
    fn toggle(&mut self, bit: u32, on: i32, command: fn(bool) -> Command) -> Result<(), Error> {
        self.ready()?;
        if on == -1 {
            return Ok(());
        }
        let enabled = on != 0;
        if (self.word & (1 << bit) != 0) != enabled {
            self.emit(command(enabled))?;
            self.word = (self.word & !(1 << bit)) | (u32::from(enabled) << bit);
        }
        Ok(())
    }

    pub fn enable_color_mask(&mut self, on: i32) -> Result<(), Error> {
        self.toggle(0, on, Command::ColorMask)
    }
    pub fn enable_depth_mask(&mut self, on: i32) -> Result<(), Error> {
        self.toggle(1, on, Command::DepthMask)
    }
    pub fn enable_depth_test(&mut self, on: i32) -> Result<(), Error> {
        self.toggle(2, on, |enabled| Command::Capability { capability: 1, enabled })
    }
    pub fn enable_alpha_test(&mut self, on: i32) -> Result<(), Error> {
        let on = if on != -1 && self.properties.force_no_alpha_test {
            0
        } else {
            on
        };
        self.toggle(3, on, |enabled| Command::Capability { capability: 0, enabled })
    }
    pub fn enable_blend(&mut self, on: i32) -> Result<(), Error> {
        let on = if on != -1 && self.properties.force_no_blend {
            0
        } else {
            on
        };
        self.toggle(4, on, |enabled| Command::Capability { capability: 4, enabled })
    }
    pub fn enable_cull_face(&mut self, on: i32) -> Result<(), Error> {
        self.toggle(6, on, |enabled| Command::Capability { capability: 3, enabled })
    }

    fn texture_bit(&mut self, stage: usize, enabled: bool) -> Result<(), Error> {
        let bit = if stage == 0 { 5 } else { 28 };
        if (self.word & (1 << bit) != 0) != enabled {
            if self.reaches_driver(stage) {
                self.select(stage)?;
                self.emit(Command::Capability { capability: 5, enabled })?;
            }
            self.word = (self.word & !(1 << bit)) | (u32::from(enabled) << bit);
        }
        Ok(())
    }

    pub fn enable_texture(&mut self, on: i32, stage: usize) -> Result<(), Error> {
        self.ready()?;
        if on == -1 {
            return Ok(());
        }
        let enabled = on != 0;
        match (stage, enabled) {
            (0, true) => self.texture_bit(0, true),
            (0, false) => {
                self.texture_bit(0, false)?;
                self.texture_bit(1, false)
            }
            (1, true) => {
                self.texture_bit(0, true)?;
                self.texture_bit(1, true)
            }
            (1, false) => self.texture_bit(1, false),
            (_, true) if self.reaches_driver(stage) => {
                self.select(stage)?;
                self.emit(Command::Capability {
                    capability: 5,
                    enabled: true,
                })
            }
            _ => Ok(()),
        }
    }

    pub fn set_depth_func(&mut self, function: i32) -> Result<(), Error> {
        self.ready()?;
        if function == -1 {
            return Ok(());
        }
        if !(0..8).contains(&function) {
            return Err(Error::InvalidValue);
        }
        if (self.word >> 7) & 7 != function as u32 {
            self.emit(Command::DepthFunc(function as u8))?;
            self.word = (self.word & !(7 << 7)) | ((function as u32) << 7);
        }
        Ok(())
    }

    pub fn set_alpha_func(&mut self, function: i32, reference: f32) -> Result<(), Error> {
        self.ready()?;
        let f = if function == -1 {
            (self.word >> 10) & 7
        } else {
            if !(0..8).contains(&function) {
                return Err(Error::InvalidValue);
            }
            function as u32
        };
        let q = if reference == -1.0 {
            (self.word >> 21) & 15
        } else {
            quantize(reference)?
        };
        let bits = (f << 10) | (q << 21);
        let mask = (7 << 10) | (15 << 21);
        if self.word & mask != bits {
            self.emit(Command::AlphaFunc {
                function: f as u8,
                reference: if reference == -1.0 { q as f32 / 15.0 } else { reference },
            })?;
            self.word = (self.word & !mask) | bits;
        }
        Ok(())
    }

    pub fn set_blend_func(&mut self, source: i32, destination: i32) -> Result<(), Error> {
        self.ready()?;
        let src = if source == -1 {
            (self.word >> 13) & 15
        } else {
            if !(0..16).contains(&source) {
                return Err(Error::InvalidValue);
            }
            source as u32
        };
        let dst = if destination == -1 {
            (self.word >> 17) & 15
        } else {
            if !(0..16).contains(&destination) {
                return Err(Error::InvalidValue);
            }
            destination as u32
        };
        let bits = (src << 13) | (dst << 17);
        let mask = 255 << 13;
        if self.word & mask != bits {
            self.emit(Command::BlendFunc {
                source: src as u8,
                destination: dst as u8,
            })?;
            self.word = (self.word & !mask) | bits;
        }
        Ok(())
    }

    pub fn set_transparency(&mut self, alpha: f32) -> Result<(), Error> {
        self.ready()?;
        self.emit(Command::AlphaMultiplier(alpha))
    }
    pub fn reset_transparency(&mut self) -> Result<(), Error> {
        self.set_transparency(1.0)
    }

    pub fn set_render_state(&mut self, packed: PackedState) -> Result<(), Error> {
        self.ready()?;
        let target = packed.apply_to(self.word);
        let changed = self.word ^ target;
        // Section 5.2 specifies word-field order, not the dependency-expanding
        // EnableTextureStateFlag order. Send the two texture bits separately.
        if changed & 1 != 0 {
            self.enable_color_mask((target & 1) as i32)?;
        }
        if changed & 2 != 0 {
            self.enable_depth_mask(((target >> 1) & 1) as i32)?;
        }
        if changed & 4 != 0 {
            self.enable_depth_test(((target >> 2) & 1) as i32)?;
        }
        if changed & 8 != 0 {
            self.enable_alpha_test(((target >> 3) & 1) as i32)?;
        }
        if changed & 16 != 0 {
            self.enable_blend(((target >> 4) & 1) as i32)?;
        }
        if changed & 32 != 0 {
            self.texture_bit(0, target & (1 << 5) != 0)?;
        }
        if changed & 64 != 0 {
            self.enable_cull_face(((target >> 6) & 1) as i32)?;
        }
        if changed & (7 << 7) != 0 {
            self.set_depth_func(((target >> 7) & 7) as i32)?;
        }
        if changed & ((7 << 10) | (15 << 21)) != 0 {
            self.set_alpha_func(((target >> 10) & 7) as i32, ((target >> 21) & 15) as f32 / 15.0)?;
        }
        if changed & (255 << 13) != 0 {
            self.set_blend_func(((target >> 13) & 15) as i32, ((target >> 17) & 15) as i32)?;
        }
        if changed & (7 << 25) != 0 {
            self.set_tex_env_mode(((target >> 25) & 7) as i32, 0)?;
        }
        if changed & (1 << 28) != 0 {
            self.texture_bit(1, target & (1 << 28) != 0)?;
        }
        if changed & (7 << 29) != 0 {
            self.set_tex_env_mode(((target >> 29) & 7) as i32, 1)?;
        }
        self.set_tex_wrap(3, 3, 0)?;
        self.set_tex_filter(1, 7, 0)?;
        self.clear_tex_transform(0)?;
        self.clear_tex_transform(1)
    }

    pub fn set_default_render_state(&mut self) -> Result<(), Error> {
        self.set_render_state(self.stack.last().copied().unwrap_or(self.default_state))
    }

    pub fn set_default_render_state_unilaterally(&mut self) -> Result<(), Error> {
        self.ready()?;
        for stage in 0..self.stages.len() {
            self.select(stage)?;
            self.emit(Command::TexMatrix(None))?;
            self.emit(Command::TexCoord(0))?;
        }
        // Reconstruct only after the entire ungated driver loop (DC6/DC7).
        for stage in 0..self.stages.len() {
            self.stages[stage] = Stage::unpublished_default();
        }
        self.word = !self.default_state.value;
        self.set_render_state(self.default_state)
    }

    pub fn set_render_state_highlight(&mut self, color: usize) -> Result<(), Error> {
        self.ready()?;
        if color >= self.highlights.len() {
            return Err(Error::InvalidValue);
        }
        self.set_render_state(self.highlight_state)?;
        self.set_tex_color(self.highlights[color])?;
        self.set_tex_combiner(&HIGHLIGHT_COMBINER_BYTES.map(|byte| byte.unwrap()), 0)
    }
}
