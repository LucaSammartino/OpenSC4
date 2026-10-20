// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network tools (`docs/specs/network-tools.md`): the tool state (§2),
//! `PlaceNetworkOccupants` / `PlaceNetworkOccupantById` (§3, §4), the
//! `PlaceNetwork` pipeline hand-off (§5), the commit (§6),
//! `DestroyOccupantsAndResolve` (§10) and the underground refit (§11).
//! Section references (`§n`) point at network-tools.md.
//!
//! # Structure
//!
//! - [`state`]: the tool state (§2) with the constructor defaults.
//! - [`place`]: slots 7 and 8 (§3, §4) and the `PlaceNetwork`/commit
//!   hand-off (§5, §6).
//! - [`resolve`]: `DestroyOccupantsAndResolve` (§10) and the underground
//!   refit (§11).
//! - [`host`]: the [`ToolHost`] seam.
//!
//! # Division of labour with the network geometry core
//!
//! `fc4_core::network_geometry` owns the geometry half of `PlaceNetwork`
//! (the drag, the validation passes, the height solver — network-geometry.md
//! §3-§9); this module owns the **tool**: the state (§2), the entry points
//! the game calls (§3-§4), the pipeline orchestration and the commit (§5-§6),
//! and the two state-heavy operations (§10-§11). The geometry runs through
//! [`ToolHost`] hooks (a host binds the geometry core; the default host is an
//! empty world, like `GeometryHost` there).
//!
//! # Choices (`[choice]`) and inferences (`[inferred]`)
//!
//! Tagged in the code and listed as numbered questions in the lane report:
//! the masked-run neighbour probe re-queries the host per direction (Q NT6),
//! the §11 records table reuses the piece struct (Q NT7), the re-solve set's
//! cell decoding assumes a 64×64 city (Q NT8).

pub mod host;
pub mod place;
pub mod resolve;
pub mod state;

#[cfg(test)]
mod tests;

pub use host::{place_network_occupant_by_id, Neighbour, NullHost, PassResult, ToolHost, ToolRun};
pub use place::{place_network, place_network_occupants};
pub use resolve::{
    destroy_occupants_and_resolve, update_underground_occupant_terrain_heights, Listed, UndergroundOutcome,
    UndergroundPiece,
};
pub use state::{CacheRecord, ToolCosts, ToolState};
