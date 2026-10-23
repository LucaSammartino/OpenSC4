// SPDX-License-Identifier: GPL-3.0-or-later
//! Particle owner contracts from particles.md. Renderer, terrain, resource and
//! stream primitives remain foreign; unresolved numerics have numbered limits.

pub mod decal;
pub mod descriptions;
pub mod dynamic;
pub mod geometry;
pub mod ordinary;
pub mod pool;

/// No inferred numeric replacement for an explicitly open spec domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Untraced particle force/terrain/transform helpers and field meanings.
    PT1,
    /// Event-name composition.
    PT2,
    /// Static-initialized constructor values.
    PT3,
    /// Geometry construction, drawing and wave propagation.
    PT8,
    /// Decal transforms, variance and complete projection records.
    PT9,
    /// Dynamic terrain, sector key/radius and draw shading arithmetic.
    PT10,
    /// Body friction, collision response and joints.
    PT11,
    /// Ordinary initialization/service and transform/configuration inventory.
    PT13,
}

#[cfg(test)]
mod tests;
