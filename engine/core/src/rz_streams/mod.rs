// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! RZ framework streams (docs/specs/output-streams.md): `cRZOStream` (§1),
//! `cRZTextOStream` (§2), `cRZNullOStream` (§3), `cRZOFStream` (§4) and the
//! input side `cRZIStream` / `cRZNullIStream` (§5).
//!
//! Pure Rust. The game's vtable dispatch is modelled by the [`GzOStream`] /
//! [`GzIStream`] traits: every base body that calls a virtual slot (the
//! string length bytes through `SetUint8`, every write through `SetVoid`, the
//! text numbers through `SetRZCharStr`) calls the trait method, so a derived
//! owner's override is honoured. The concrete destinations are typed
//! providers: [`ByteSink`] (the derived `SetVoid`/`Flush`/`GetError`),
//! [`TextSink`] (the derived `SetRZCharStr`), [`ByteSource`] (the derived
//! `GetVoid`/`Skip`/`GetError`) and [`file::OfFileOpener`] (the file the
//! `ofstream` of §4 writes to). Open questions: OS1-OS6 in
//! docs/specs/questions/output-streams.md.

pub mod com;
pub mod file;
pub mod istream;
pub mod ostream;
pub mod text;

#[cfg(test)]
mod tests;

pub use com::{CountedRef, ReleaseOutcome, StreamCore, IID_GZ_ISTREAM, IID_GZ_OSTREAM, IID_GZ_UNKNOWN};
pub use file::{OfFile, OfFileOpener, OfOpenMode, RzOFStream, StdFileOpener};
pub use istream::{ByteSource, GzIStream, NullSource, RzIStream, RzNullIStream};
pub use ostream::{ByteSink, GzOStream, NullSink, RzNullOStream, RzOStream};
pub use text::{format_g, RzTextOStream, TextSink};

/// A `cIGZSerializable` as the streams see it: `SetGZSerializable(o)` is
/// `o.Write(stream)` (§1.3 slot 16) and `GetGZSerializable(o)` is
/// `o.Read(stream)` (§5 slot 16).
pub trait GzSerializable {
    fn write(&mut self, stream: &mut dyn GzOStream) -> bool;
    fn read(&mut self, stream: &mut dyn GzIStream) -> bool;
}

/// `strlen` of a C string held in a byte slice: the bytes before the first
/// NUL (the whole slice when it has none).
pub fn c_str(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == 0) {
        Some(n) => &s[..n],
        None => s,
    }
}
