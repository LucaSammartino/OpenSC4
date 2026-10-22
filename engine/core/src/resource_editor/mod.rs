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

/// Portable dispatch metadata for the published behavioral owners (§8).
/// Identity-only curve/catalog/helper classes deliberately have no model here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorKind {
    Text(u16),
    Combo(u16),
    Spinner,
    Grid,
    Colors,
    ResourceKeyTable,
}

pub fn editor_kind(class: u32) -> Option<EditorKind> {
    use crate::variant::mem;
    Some(match class {
        0x671E_972A => EditorKind::Text(mem::RZ_CHAR),
        0x490E_01A7 => EditorKind::Text(mem::SINT64),
        0xC71E_972F => EditorKind::Text(mem::UINT32),
        0x471E_9733 => EditorKind::Text(mem::SINT32),
        0x471E_9737 => EditorKind::Text(mem::SINT8),
        0xC71E_973C => EditorKind::Text(mem::UINT8),
        0xC71E_9740 => EditorKind::Text(mem::FLOAT32),
        0x47C3_5AFC => EditorKind::Text(mem::BOOL),
        0x8739_DFC1 => EditorKind::Combo(mem::UINT32),
        0x87C3_4EDA => EditorKind::Combo(mem::UINT8),
        0x871E_9744 => EditorKind::Spinner,
        0x0A09_AC46 => EditorKind::Grid,
        0xCA0C_B10F => EditorKind::Colors,
        RESOURCE_TABLE_CLASS => EditorKind::ResourceKeyTable,
        _ => return None,
    })
}

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
