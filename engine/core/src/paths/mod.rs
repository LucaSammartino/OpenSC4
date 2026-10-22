//! Portable path owners and byte algorithms from docs/specs/paths.md §§1–4.
//!
//! No host path parsing, Unicode folding, wildcard matching or sorting is used. OS primitives
//! receive framework bytes; the eventual native adapter owns system-code-page conversion (§4).
//! PA4–PA8 document the unpublished string/entry and boundary contracts; no native binding is claimed.

pub mod directory;
pub mod helpers;
pub mod path;

pub use directory::{DirectoryProvider, FindData, FindSearch, RzDirectory, RzDirectoryEntry};
pub use helpers::{CurrentDirectory, PathParts};
pub use path::RzPath;

#[cfg(test)]
mod tests;
