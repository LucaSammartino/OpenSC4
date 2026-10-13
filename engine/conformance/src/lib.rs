// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Runner for the language-neutral conformance vectors in
//! `engine/conformance/vectors/*.json`. The format is documented in
//! `engine/conformance/README.md`; this crate is one implementation of it,
//! running the vectors against `fc4-core`.

pub(crate) mod aura;
pub(crate) mod budget;
pub(crate) mod crime;
pub(crate) mod demand;
pub(crate) mod development;
pub(crate) mod development_simulator;
pub(crate) mod flammability;
pub(crate) mod flora;
pub mod foundations;
pub(crate) mod grid;
pub(crate) mod growth_developer;
pub mod json;
pub(crate) mod landfill;
pub(crate) mod landvalue;
pub(crate) mod lot_developer;
pub(crate) mod neighbors;
pub(crate) mod ordinance;
pub(crate) mod plumbing;
pub(crate) mod pollution;
pub(crate) mod power;
pub(crate) mod protection;
pub mod runner;
pub(crate) mod scalar;
pub(crate) mod simgrid;
pub(crate) mod tract_developer;
pub mod variants;
pub(crate) mod weather;
pub(crate) mod zones;

pub use runner::{run_vector_file, run_vector_text};
