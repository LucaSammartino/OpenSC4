// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable contracts of `docs/specs/resource-exemplar-editor.md`.
//!
//! These are models, not GZCOM classes. Unknown factory/lifecycle, pointer
//! ownership and curve/catalog methods are deliberately outside this module.
//! Undefined native reads and nonterminating loops return explicit errors.

mod config;
mod controls;
mod table;
mod text;
mod view;

pub use config::*;
pub use controls::*;
pub use table::*;
pub use text::*;
pub use view::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorError {
    InvalidField,
    InvalidValue,
    UnsupportedType(u16),
    UnspecifiedFormatting(u16),
    MissingControl(usize),
    InvalidSelection(usize),
    /// Native UINT8 list GetValue counter would never terminate (§10).
    NonterminatingUint8List,
    /// Native code reads outside its allocation (§11.3, §12).
    OutOfBoundsNativeRead,
    HostFailure,
}

#[cfg(test)]
mod tests;
