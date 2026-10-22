//! Portable path owners and byte algorithms from docs/specs/paths.md §§1–4.
//!
//! OS primitives receive framework bytes; the eventual native adapter owns system-code-page
//! conversion and thread CRT classification (§§2.3, 4). Unsupported classifier domains are errors.
//! PA4–PA8 supply the counted-string, entry, split and find contracts; no native binding is claimed.
//! Directory ownership and stable string sorting are reused from rz_files::directory.

pub mod directory;
pub mod helpers;
pub mod path;

pub use directory::{DirectoryProvider, FindData, FindSearch, RzDirectory, RzDirectoryEntry};
pub use helpers::{CurrentDirectory, PathParts};
pub use path::RzPath;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contract_tests;
