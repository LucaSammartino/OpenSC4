// SPDX-License-Identifier: GPL-3.0-or-later
//! Word layout and packing, draw-context.md sections 2 and 5.1.

use super::Error;

pub const DEFAULT_STATE: PackedState = PackedState {
    value: 0x230a_8587,
    mask: u32::MAX,
};
pub const HIGHLIGHT_STATE: PackedState = PackedState {
    value: 0x030a_85b5,
    mask: 0x1fff_ffff,
};
pub const BOX_STATE: PackedState = PackedState {
    value: 0x020a_8197,
    mask: 0x1e1f_e3ff,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedState {
    pub value: u32,
    pub mask: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackInput {
    pub color_write: bool,
    pub depth_write: bool,
    pub depth_test: bool,
    pub alpha_test: bool,
    pub blend: bool,
    pub textures: usize,
    pub cull: bool,
    pub depth_func: i32,
    pub source_blend: i32,
    pub destination_blend: i32,
    pub alpha_func: i32,
    pub alpha_reference: f32,
    pub stage0_mode: i32,
    pub stage1_mode: i32,
}

#[cfg(test)]
pub(super) fn quantize(reference: f32) -> Result<u32, Error> {
    quantize_with_control(reference, super::Arithmetic::default())
}

pub(super) fn quantize_with_control(reference: f32, arithmetic: super::Arithmetic) -> Result<u32, Error> {
    // No clamping, including outside [0,1]. One binary32 bias store, then nibble.
    let product = arithmetic.mul(f64::from(reference), 15.0)?;
    let biased = arithmetic.add(product, 12_582_912.0)?;
    Ok(arithmetic.store(biased)?.to_bits() & 15)
}

impl PackedState {
    pub fn pack(input: PackInput) -> Result<Self, Error> {
        Self::pack_with_control(input, super::Arithmetic::default())
    }

    pub fn pack_with_control(input: PackInput, arithmetic: super::Arithmetic) -> Result<Self, Error> {
        let mut result = Self {
            value: 0,
            mask: u32::MAX,
        };
        for (bit, enabled) in [
            (0, input.color_write),
            (1, input.depth_write),
            (2, input.depth_test),
            (3, input.alpha_test),
            (4, input.blend),
            (5, input.textures != 0),
            (6, input.cull),
            (28, input.textures >= 2),
        ] {
            result.value |= u32::from(enabled) << bit;
        }
        for (shift, width, value) in [
            (7, 3, input.depth_func),
            (10, 3, input.alpha_func),
            (13, 4, input.source_blend),
            (17, 4, input.destination_blend),
            (25, 3, input.stage0_mode),
            (29, 3, input.stage1_mode),
        ] {
            let field = ((1u32 << width) - 1) << shift;
            if value == -1 {
                result.mask &= !field;
            } else if value < 0 || value as u32 >= 1 << width {
                return Err(Error::InvalidValue);
            } else {
                result.value |= (value as u32) << shift;
            }
        }
        if input.alpha_reference == -1.0 {
            result.mask &= !(15 << 21);
        } else {
            result.value |= quantize_with_control(input.alpha_reference, arithmetic)? << 21;
        }
        Ok(result)
    }

    pub fn apply_to(self, word: u32) -> u32 {
        (word & !self.mask) | self.value
    }
}
