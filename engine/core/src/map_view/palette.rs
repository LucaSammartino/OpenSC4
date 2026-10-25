// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit supplied-palette helpers (Â§4.2), never owner palette construction.
use super::Unsupported;
use crate::grid::{GridElem, TractGrid};

/// A caller-supplied raw 256-entry table. Emission selects entries verbatim;
/// the caller must identify its own alpha-multiplied input if that is intended.
/// This type cannot complete SetDataView or Update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPalette(pub [u32; 256]);

/// Both tables are built by the owner, never returned by a foreign renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedPalettes {
    pub straight: RawPalette,
    pub premultiplied: RawPalette,
}

pub fn build_palettes(pairs: &[u32]) -> Result<OwnedPalettes, Unsupported> {
    // Unsafe native reads are diagnosed, never reported as a native false return.
    if pairs.len() < 2 || pairs.len() % 2 != 0 {
        return Err(Unsupported::RampGeneration);
    }
    let mut table = [0; 256];
    let mut stop = 0;
    for (index, color) in table.iter_mut().enumerate() {
        let index = index as u32;
        while stop + 3 < pairs.len() && pairs[stop + 2] <= index {
            stop += 2;
        }
        *color = pairs[stop + 1];
        if index < pairs[stop] || stop + 3 >= pairs.len() {
            continue;
        }
        let weight = index.wrapping_sub(pairs[stop]).wrapping_mul(256) / pairs[stop + 2].wrapping_sub(pairs[stop]);
        *color = 0;
        for shift in [0, 8, 16, 24] {
            let a = i64::from((pairs[stop + 1] >> shift) & 255);
            let b = i64::from((pairs[stop + 3] >> shift) & 255);
            *color |= ((a + ((b - a) * i64::from(weight) + 128).div_euclid(256)) as u32 & 255) << shift;
        }
    }
    Ok(OwnedPalettes {
        straight: RawPalette(table),
        premultiplied: RawPalette(table.map(premultiply_argb)),
    })
}

impl RawPalette {
    pub fn lookup(&self, index: u8) -> u32 {
        self.0[usize::from(index)]
    }
}

pub fn u8_index(value: u8) -> u8 {
    value
}

pub fn i8_index(value: i8) -> u8 {
    (i16::from(value) + 128) as u8
}

/// Only finite values whose truncated result is a representable table index.
/// This checks before Rust's saturating cast, never inventing native clipping.
pub fn float_index(value: f32) -> Result<u8, Unsupported> {
    // Masked x87 invalid produces integer-indefinite, whose low byte is zero.
    if !value.is_finite() || !(-9223372036854775808.0..9223372036854775808.0).contains(&f64::from(value)) {
        return Ok(0);
    }
    Ok(value.trunc() as i64 as u8)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScaleFactors {
    pub unsigned: u64,
    pub signed: i64,
}

impl Default for ScaleFactors {
    fn default() -> Self {
        Self {
            unsigned: 256,
            signed: 256,
        }
    }
}

impl ScaleFactors {
    pub fn from_scale(scale: Option<u32>) -> Self {
        match scale {
            Some(s) if s > 1 && s < 1 << 24 => Self {
                unsigned: 0xffff00 / u64::from(s),
                signed: 0x7fff00 / i64::from(s),
            },
            _ => Self::default(),
        }
    }

    pub fn unsigned_index(self, value: u16) -> u8 {
        ((u64::from(value) * self.unsigned) >> 16).min(255) as u8
    }

    pub fn signed_index(self, value: i16) -> u8 {
        (128 + ((i64::from(value) * self.signed) >> 16)).clamp(0, 255) as u8
    }
}

pub fn u16_index(value: u16, scale: u32) -> Result<u8, Unsupported> {
    Ok(ScaleFactors::from_scale(Some(scale)).unsigned_index(value))
}

pub fn i16_index(value: i16, scale: u32) -> Result<u8, Unsupported> {
    Ok(ScaleFactors::from_scale(Some(scale)).signed_index(value))
}

/// Â§3.1's per-channel alpha copy. Rounding to nearest is exact integer
/// arithmetic; division by 255 has no integer-input halfway ties.
pub fn premultiply_argb(argb: u32) -> u32 {
    let alpha = argb >> 24;
    let channel = |shift: u32| ((((argb >> shift) & 255u32) * alpha + 128) / 256) << shift;
    (argb & 0xff00_0000) | channel(16) | channel(8) | channel(0)
}

#[derive(Clone, Copy, Debug)]
pub enum Values<'a> {
    U8(&'a [u8]),
    I8(&'a [i8]),
    F32(&'a [f32]),
    U16(&'a [u16]),
    I16(&'a [i16]),
}

impl Values<'_> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::I16(v) => v.len(),
        }
    }

    pub(super) fn index(self, i: usize, factors: ScaleFactors) -> Result<u8, Unsupported> {
        match self {
            Self::U8(v) => Ok(u8_index(v[i])),
            Self::I8(v) => Ok(i8_index(v[i])),
            Self::F32(v) => float_index(v[i]),
            Self::U16(v) => Ok(factors.unsigned_index(v[i])),
            Self::I16(v) => Ok(factors.signed_index(v[i])),
        }
    }
}

/// Portable helper contract: refuse unsupported inputs before writing any
/// output. Native malformed-buffer/failure behavior is not specified (QV6).
pub fn emit_colors(values: Values<'_>, palette: &RawPalette, output: &mut [u32]) -> Result<(), Unsupported> {
    emit_colors_scaled(values, palette, output, ScaleFactors::default())
}

pub fn emit_colors_scaled(
    values: Values<'_>,
    palette: &RawPalette,
    output: &mut [u32],
    factors: ScaleFactors,
) -> Result<(), Unsupported> {
    if values.len() != output.len() {
        return Err(Unsupported::BufferLength);
    }
    for i in 0..values.len() {
        values.index(i, factors)?;
    }
    for (i, color) in output.iter_mut().enumerate() {
        *color = palette.lookup(values.index(i, factors)?);
    }
    Ok(())
}

pub trait PaletteSample: GridElem {
    fn palette_index(self) -> Result<u8, Unsupported>;
}

impl PaletteSample for u8 {
    fn palette_index(self) -> Result<u8, Unsupported> {
        Ok(u8_index(self))
    }
}

impl PaletteSample for i8 {
    fn palette_index(self) -> Result<u8, Unsupported> {
        Ok(i8_index(self))
    }
}

impl PaletteSample for f32 {
    fn palette_index(self) -> Result<u8, Unsupported> {
        float_index(self)
    }
}

/// Reuse the existing raw simulator grid API unchanged. Read x outer / z
/// inner as specified by that API. This does not create/resample/block the
/// owner's source grid, choose cell geometry, or complete Update.
pub fn emit_grid<T: PaletteSample>(
    grid: &dyn TractGrid<T>,
    palette: &RawPalette,
    output: &mut [u32],
) -> Result<(), Unsupported> {
    let geometry = grid.geometry();
    if geometry.count_x < 0 || geometry.count_z < 0 {
        return Err(Unsupported::GridGeometry);
    }
    let count = (geometry.count_x as usize)
        .checked_mul(geometry.count_z as usize)
        .ok_or(Unsupported::GridGeometry)?;
    if count != output.len() {
        return Err(Unsupported::BufferLength);
    }
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(count)
        .map_err(|_| Unsupported::HelperAllocation)?;
    for x in 0..geometry.count_x {
        for z in 0..geometry.count_z {
            indices.push(grid.get(x, z).palette_index()?);
        }
    }
    for (color, index) in output.iter_mut().zip(indices) {
        *color = palette.lookup(index);
    }
    Ok(())
}
