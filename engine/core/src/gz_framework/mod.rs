// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable GZ framework owners, specified by `gz-framework-core.md`.
//! Platform operations and application hooks are foreign; see README for closure.

pub mod buffer;
pub mod com;
pub mod filesystem;
pub mod formatting;
pub mod framework;
pub mod logging;
pub mod script;
pub mod support;

pub type Token = u64;
/// An unresolved operation is never reported as native success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unsupported(pub &'static str);
pub type Result<T> = std::result::Result<T, Unsupported>;

/// QI returns an acquired reference. Other token arguments are borrowed.
pub trait References {
    fn add_ref(&mut self, object: Token);
    fn release(&mut self, object: Token);
    fn query(&mut self, object: Token, iid: u32) -> Option<Token>;
}

#[cfg(test)]
mod tests;
