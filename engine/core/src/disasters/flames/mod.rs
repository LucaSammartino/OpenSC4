// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable visual bodies from `docs/specs/fire-flames.md` §§1–4.
//!
//! This is the visual component, not a replacement for the full fire occupant.
//! See README.md for the published/missing inventory and FF2–FF4 limitations.
//! Foreign services expose mesh data, seed sources, and visual-effect primitives;
//! all flame selection, placement, stage and view decisions run here.

mod flame;
mod model;
mod occupant;

pub use flame::{EffectFactory, Flame, FlameInitial, Stage, VisualEffect};
pub use model::{get_points_on_model, Mesh, ModelInstance, ModelPoint, PlacementError, Unsupported};
pub use occupant::{FireFlames, OccupantInit, Placement, FIRE_OCCUPANT_GROUP, FLORA_TYPE};

#[cfg(test)]
mod tests;
