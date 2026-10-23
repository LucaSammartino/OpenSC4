// SPDX-License-Identifier: GPL-3.0-or-later
//! effects-parser §13 instruction/store owner. The arithmetic backend owns x87
//! values and control-state qualification (EFP7); it never returns a matrix.
use super::{RawFloat, Record, Unsupported};

/// Handle to a backend's extended value, not a narrowed host float.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extended(pub u32);
type Result<T> = std::result::Result<T, Unsupported>;
pub trait Arithmetic {
    fn input(&mut self, raw: RawFloat) -> Result<Extended>;
    fn load(&mut self, value: f32) -> Result<Extended>;
    fn store(&mut self, value: Extended) -> Result<f32>;
    fn mul(&mut self, a: Extended, b: Extended) -> Result<Extended>;
    fn add(&mut self, a: Extended, b: Extended) -> Result<Extended>;
    fn sub(&mut self, a: Extended, b: Extended) -> Result<Extended>;
    fn neg(&mut self, a: Extended) -> Result<Extended>;
    fn cos(&mut self, angle: Extended) -> Result<Extended>;
    fn sin(&mut self, angle: Extended) -> Result<Extended>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}
fn rounded(math: &mut dyn Arithmetic, value: Extended) -> Result<Extended> {
    let stored = math.store(value)?;
    math.load(stored)
}
fn product(math: &mut dyn Arithmetic, a: Extended, b: Extended, round: bool) -> Result<Extended> {
    let value = math.mul(a, b)?;
    if round {
        rounded(math, value)
    } else {
        Ok(value)
    }
}
fn read(record: &Record, row: usize, col: usize) -> f32 {
    let at = 4 + row * 12 + col * 4;
    f32::from_bits(u32::from_le_bytes(
        record.component.placement[at..at + 4].try_into().unwrap(),
    ))
}
pub fn apply(record: &mut Record, axis: Axis, raw: RawFloat, math: &mut dyn Arithmetic) -> Result<()> {
    let input = math.input(raw)?;
    let tau = math.load(f32::from_bits(0x40c90fdb))?;
    let angle = math.mul(input, tau)?; // no angle store
    let cosine = math.cos(angle)?;
    let cosine = rounded(math, cosine)?;
    let sine = math.sin(angle)?; // deliberately not stored yet
    compose(record, axis, cosine, sine, math)
}
/// Public arithmetic-stage boundary for RO5/RO6; coefficients alone are not
/// claimed to be a successful parse or a native trigonometric oracle.
pub fn compose(
    record: &mut Record,
    axis: Axis,
    cosine: Extended,
    sine: Extended,
    math: &mut dyn Arithmetic,
) -> Result<()> {
    let (first, second) = match axis {
        Axis::X => (1, 2),
        Axis::Y => (0, 2),
        Axis::Z => (0, 1),
    };
    let before = std::array::from_fn::<_, 3, _>(|row| [read(record, row, first), read(record, row, second)]);
    for (row, [a, b]) in before.iter().copied().enumerate() {
        let a = math.load(a)?;
        let b = math.load(b)?;
        let ca = product(math, cosine, a, row != 0)?;
        let sb = product(math, sine, b, row != 0)?;
        let result = if axis == Axis::Y {
            math.sub(ca, sb)?
        } else {
            math.add(ca, sb)?
        };
        let value = math.store(result)?;
        record.word(4 + row * 12 + first * 4, value.to_bits());
    }
    let negative = if axis != Axis::Y {
        let negative = math.neg(sine)?;
        Some(rounded(math, negative)?)
    } else {
        None
    };
    for (row, [a, b]) in before.iter().copied().enumerate() {
        let a = math.load(a)?;
        let b = math.load(b)?;
        let left = if let Some(negative) = negative {
            product(math, a, negative, row != 0)?
        } else {
            product(math, sine, a, row != 2)?
        };
        let cb = product(math, cosine, b, true)?;
        let result = math.add(left, cb)?;
        let value = math.store(result)?;
        record.word(4 + row * 12 + second * 4, value.to_bits());
    }
    record.component.placement[1] |= 4;
    record.increment();
    Ok(())
}

#[cfg(test)]
mod tests;
