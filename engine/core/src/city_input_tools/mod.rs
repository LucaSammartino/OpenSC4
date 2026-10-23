// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable Flora, Torch and CitySituations algorithms from city-tools.md.
//! See README.md for per-method support and fixtures.md for pre-source oracles.
//! Hosts implement primitive services only, never native tool/handler methods.

mod flora;
mod situations;
mod torch;

use std::num::NonZeroU64;

pub use crate::view_utilities::{Pixel, Vec3};
pub use flora::*;
pub use situations::*;
pub use torch::*;

/// Portable identity, NOT a pointer or a claim about native handle width.
/// Held flora/fire tokens transfer one explicit acquired obligation on successful
/// init; created effects transfer one created obligation. Other tokens borrow.
/// Copying an identity never duplicates an acquisition. No implicit Drop calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resource(pub NonZeroU64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveFailure {
    Unavailable,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// CT14: no silent zero/negative-price or overflow policy.
    NonpositivePrice,
    ArithmeticOverflow,
    CoordinateConversion,
    /// CT14/17: preserve the host's foreign failure, no fallback/default.
    ForeignResource(PrimitiveFailure),
    /// CT14: no invented reinitialization/use-after-shutdown contract.
    InactiveLifetime,
    /// CT17: no invented world-position default.
    MissingPosition,
    /// CT16: the no-lock/no-hit Move branch is not published.
    EmptyHover,
    UnknownMessage,
}

impl From<PrimitiveFailure> for Unsupported {
    fn from(value: PrimitiveFailure) -> Self {
        Self::ForeignResource(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub x: i32,
    pub z: i32,
}

/// CT14: checked portable domain, not Rust's saturating float-to-int cast.
fn trunc_i32(value: f32) -> Result<i32, Unsupported> {
    let truncated = f64::from(value).trunc();
    if !truncated.is_finite() || truncated < f64::from(i32::MIN) || truncated > f64::from(i32::MAX) {
        return Err(Unsupported::CoordinateConversion);
    }
    Ok(truncated as i32)
}

fn cell_at(p: Vec3) -> Result<Cell, Unsupported> {
    Ok(Cell {
        x: trunc_i32(p.x / 16.0)?,
        z: trunc_i32(p.z / 16.0)?,
    })
}

fn finite(p: Vec3) -> Result<Vec3, Unsupported> {
    if p.x.is_finite() && p.y.is_finite() && p.z.is_finite() {
        Ok(p)
    } else {
        Err(Unsupported::CoordinateConversion)
    }
}

#[cfg(test)]
mod tests;
