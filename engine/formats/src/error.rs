// SPDX-License-Identifier: GPL-3.0-or-later
//! Error type shared by every reader in this crate.

use std::fmt;

use crate::tgi::Tgi;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while reading game data. No reader panics; they all return one of these.
#[derive(Debug)]
pub enum Error {
    /// An operating-system I/O error.
    Io(std::io::Error),
    /// The data does not start with the expected signature.
    BadMagic { what: &'static str, found: Vec<u8> },
    /// A well-formed file in a variant this crate does not read (for example DBPF 2.0).
    Unsupported { what: &'static str, detail: String },
    /// The data ended before a structure was complete.
    Truncated {
        what: &'static str,
        needed: u64,
        available: u64,
    },
    /// A record points outside its container.
    OutOfBounds {
        what: &'static str,
        offset: u64,
        size: u64,
        limit: u64,
    },
    /// The data is structurally invalid.
    Corrupt { what: &'static str, detail: String },
    /// A text exemplar line could not be parsed (1-based line number).
    Text { line: usize, detail: String },
    /// No entry with this key.
    NotFound(Tgi),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::BadMagic { what, found } => write!(f, "{what}: bad signature {found:02X?}"),
            Error::Unsupported { what, detail } => write!(f, "{what}: unsupported: {detail}"),
            Error::Truncated {
                what,
                needed,
                available,
            } => {
                write!(f, "{what}: truncated (needs {needed} bytes, {available} available)")
            }
            Error::OutOfBounds {
                what,
                offset,
                size,
                limit,
            } => {
                write!(f, "{what}: range {offset}+{size} is outside 0..{limit}")
            }
            Error::Corrupt { what, detail } => write!(f, "{what}: corrupt: {detail}"),
            Error::Text { line, detail } => write!(f, "text exemplar line {line}: {detail}"),
            Error::NotFound(tgi) => write!(f, "no entry {tgi}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub(crate) fn corrupt(what: &'static str, detail: impl Into<String>) -> Error {
    Error::Corrupt {
        what,
        detail: detail.into(),
    }
}
