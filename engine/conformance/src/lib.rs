// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Runner for the language-neutral conformance vectors in
//! `engine/conformance/vectors/*.json`. The format is documented in
//! `engine/conformance/README.md`; this crate is one implementation of it,
//! running the vectors against `fc4-core`.

pub(crate) mod budget;
pub mod foundations;
pub mod json;
pub(crate) mod landvalue;
pub mod runner;
pub(crate) mod simgrid;
pub mod variants;

pub use runner::{run_vector_file, run_vector_text};
