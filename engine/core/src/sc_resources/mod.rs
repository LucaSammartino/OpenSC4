// SPDX-License-Identifier: GPL-3.0-or-later
//! SC resource scripts and text persistence, docs/specs/sc-resources.md.
//! Foreign boundaries are files, buffer creation/conversion and graph surveying.
//! See README.md for owner closure and explicitly unsettled contracts.
pub mod ascii;
pub mod command;
pub mod files;
pub mod image;
pub mod lexical;
pub mod token;

pub use crate::effects_parser::script::{check_switches, find_positional, find_switch, ScriptToken};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Parse { code: u32, message: String },
    Unsupported(&'static str),
    Io,
    InvalidContext,
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn parse_error(code: u32, message: impl Into<String>) -> Error {
    Error::Parse {
        code,
        message: message.into(),
    }
}
